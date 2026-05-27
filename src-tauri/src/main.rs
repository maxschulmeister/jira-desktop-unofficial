#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{Manager, WebviewUrl, WebviewWindow};
use uuid::Uuid;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

static LAST_TITLES: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

// ─────────────────────────────────────────────────────────────────────────────
// Native error dialog — per platform
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
fn show_error_dialog(title: &str, message: &str) {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let encode = |s: &str| -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0u16))
            .collect()
    };

    let title_w   = encode(title);
    let message_w = encode(message);

    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            std::ptr::null_mut(),
            message_w.as_ptr(),
            title_w.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_OK
                | windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

#[cfg(target_os = "linux")]
fn show_error_dialog(title: &str, message: &str) {
    use std::process::Command;

    if Command::new("zenity")
        .args(["--error", &format!("--title={}", title), &format!("--text={}", message), "--width=460"])
        .status().map(|s| s.success()).unwrap_or(false) { return; }

    if Command::new("kdialog")
        .args(["--error", message, "--title", title])
        .status().map(|s| s.success()).unwrap_or(false) { return; }

    if Command::new("xmessage")
        .args(["-center", &format!("{}\n\n{}", title, message)])
        .status().is_ok() { return; }

    eprintln!("ERROR: {}\n{}", title, message);
}

#[cfg(target_os = "macos")]
fn show_error_dialog(title: &str, message: &str) {
    use std::process::Command;
    let script = format!(
        "display dialog \"{}\" with title \"{}\" buttons {{\"OK\"}} default button \"OK\" with icon stop",
        message.replace('"', "\\\""), title.replace('"', "\\\"")
    );
    if Command::new("osascript").args(["-e", &script]).status().is_err() {
        eprintln!("ERROR: {}\n{}", title, message);
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn show_error_dialog(title: &str, message: &str) {
    eprintln!("ERROR: {}\n{}", title, message);
}

// ─────────────────────────────────────────────────────────────────────────────
// Runtime availability checks
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
fn check_webkit_available() -> bool {
    use std::process::Command;
    for pattern in &["webkit2gtk", "webkit2gtk-4", "webkit2gtk-4.0", "webkit2gtk-4.1", "webkit2gtk-5"] {
        if Command::new("pkg-config").args(["--exists", pattern])
            .status().map(|s| s.success()).unwrap_or(false) { return true; }
    }
    if let Ok(output) = Command::new("sh")
        .args(["-c", "find /usr/lib /usr/lib64 -name 'libwebkit2gtk*.so*' 2>/dev/null | head -1"])
        .output() { if !output.stdout.is_empty() { return true; } }
    if let Ok(output) = Command::new("ldconfig").args(["-p"]).output() {
        if String::from_utf8_lossy(&output.stdout).contains("webkit") { return true; }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn check_webkit_available() -> bool { true }

/// Uses wry's official webview_version() which calls Microsoft's
/// GetAvailableCoreWebView2BrowserVersionString API directly.
///
///   Ok(version)  → WebView2 is installed and loadable  → true
///   Err(_)       → confirmed not installed/broken       → false
#[cfg(target_os = "windows")]
fn check_webview2_available() -> bool {
    match wry::webview_version() {
        Ok(version) => {
            println!("[JDU] WebView2 runtime found: {}", version);
            true
        }
        Err(e) => {
            println!("[JDU] WebView2 not available: {}", e);
            false
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn check_webview2_available() -> bool { true }

// ─────────────────────────────────────────────────────────────────────────────
// Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
fn update_window_title(window: tauri::Window, title: String) {
    let window_label = window.label().to_string();
    let mut last_titles = LAST_TITLES.lock().unwrap();
    if let Some(last_title) = last_titles.get(&window_label) {
        if last_title == &title { return; }
    }
    last_titles.insert(window_label.clone(), title.clone());
    println!("📝 Updating title for window '{}' to: {}", window_label, title);
    if !title.is_empty() && title != "null" && title != "undefined" {
        let _ = window.set_title(&format!("{} [JDU]", title));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// JS injections
// ─────────────────────────────────────────────────────────────────────────────

fn inject_drag_drop_fix(window: &WebviewWindow) {
    let window_clone = window.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(1000));
        let _ = window_clone.eval(r#"
            (function() {
                if (window.__jdu_dnd_patched__) return;
                window.__jdu_dnd_patched__ = true;
                console.log('[JDU] Applying HTML5 drag-drop fix');

                const style = document.createElement('style');
                style.textContent = `
                    html, body { touch-action: pan-y; }
                    [draggable="true"] { -webkit-user-drag: element; cursor: grab; }
                    [draggable="true"]:active { cursor: grabbing; }
                `;
                document.head.appendChild(style);

                document.addEventListener('dragover', (e) => {
                    e.preventDefault();
                    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
                }, false);

                document.addEventListener('drop', (e) => { e.preventDefault(); }, false);

                const observer = new MutationObserver((mutations) => {
                    for (const mutation of mutations) {
                        for (const node of mutation.addedNodes) {
                            if (node.nodeType !== 1) continue;
                            const cards = node.matches
                                ? (node.matches('[role="button"], .ghx-issue, .js-issue-title, [data-testid*="card"]')
                                    ? [node]
                                    : [...node.querySelectorAll('[role="button"], .ghx-issue, [data-testid*="card"]')])
                                : [];
                            for (const card of cards) { card.setAttribute('draggable', 'true'); }
                        }
                    }
                });

                if (document.body) {
                    observer.observe(document.body, { childList: true, subtree: true });
                }
                console.log('[JDU] HTML5 drag-drop fix applied');
            })();
        "#);
    });
}

const TITLE_SYNC_JS: &str = r#"
    (function() {
        if (window.__jdu_title_sync__) return;
        window.__jdu_title_sync__ = true;

        let lastSentTitle = '';
        let debounceTimer = null;

        function sendTitle() {
            if (!window.__TAURI__) return;
            const title = (document.title || '').trim();
            if (!title || title === 'null' || title === 'undefined') return;
            if (title === lastSentTitle) return;

            clearTimeout(debounceTimer);
            debounceTimer = setTimeout(() => {
                lastSentTitle = title;
                window.__TAURI__.core.invoke('update_window_title', { title })
                    .catch(err => console.error('[JDU] title sync error:', err));
            }, 150);
        }

        if (document.readyState === 'complete') { sendTitle(); }
        else { window.addEventListener('load', sendTitle); }

        const titleEl = document.querySelector('title');
        if (titleEl) {
            new MutationObserver(sendTitle).observe(titleEl, {
                subtree: true, characterData: true, childList: true
            });
        }

        let lastHref = location.href;
        new MutationObserver(() => {
            if (location.href !== lastHref) { lastHref = location.href; setTimeout(sendTitle, 300); }
        }).observe(document.body || document.documentElement, { subtree: true, childList: true });

        sendTitle();
    })();
"#;


// ─────────────────────────────────────────────────────────────────────────────
// Window command
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn open_website_window(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let hostname = url
        .replace("https://", "").replace("http://", "")
        .split('/').next().unwrap_or(&url).to_string();

    let window_id = format!("website-window-{}", Uuid::new_v4());

    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.hide();
    }

    let parsed_url = url.parse().map_err(|e| format!("Invalid URL: {}", e))?;

    let builder = tauri::WebviewWindowBuilder::new(&app, &window_id, WebviewUrl::External(parsed_url))
        .title(&format!("🔄 Loading {}...", hostname))
        .inner_size(1200.0, 800.0)
        .resizable(true)
        .visible(true)
        .decorations(true)
        // Disables WebView2's native drag interception so HTML5 DnD works on Windows.
        // No-op on macOS and Linux — safe to call unconditionally.
        .disable_drag_drop_handler();

    match builder.build() {
        Ok(new_window) => {
            let app_handle = app.clone();
            let window_label_clone = new_window.label().to_string();

            new_window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let mut last_titles = LAST_TITLES.lock().unwrap();
                    last_titles.remove(&window_label_clone);
                    if let Some(main_window) = app_handle.get_webview_window("main") {
                        let _ = main_window.show();
                    }
                }
            });

            let win_title = new_window.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(800));
                let _ = win_title.eval(TITLE_SYNC_JS);
            });

            inject_drag_drop_fix(&new_window);
            Ok(())
        }
        Err(e) => Err(format!("Window creation failed: {}", e)),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Entry point
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    // ── Panic hook — last-resort safety net on Windows ──────────────────────
    // If anything panics after this point (including deep inside Tauri/wry),
    // the user sees a dialog instead of a silent disappearing window.
    // windows_subsystem = "windows" hides stderr so eprintln is useless.
    #[cfg(target_os = "windows")]
    std::panic::set_hook(Box::new(|info| {
        let msg = info.payload().downcast_ref::<String>().map(|s| s.as_str())
            .or_else(|| info.payload().downcast_ref::<&str>().copied())
            .unwrap_or("Unknown panic");

        let lower = msg.to_lowercase();
        let _is_webview2 = lower.contains("webview") || lower.contains("edge")
            || lower.contains("class not registered") || lower.contains("runtime");

        if _is_webview2 {
            show_error_dialog(
                "JDU — Missing Runtime",
                "Microsoft WebView2 Runtime is not installed or is corrupted.\n\n\
                 JDU requires WebView2 to display web content.\n\n\
                 Please download and install it from:\n\
                 https://developer.microsoft.com/en-us/microsoft-edge/webview2/\n\n\
                 Choose the 'Evergreen Bootstrapper' or 'Standalone Installer'.",
            );
        } else {
            show_error_dialog(
                "JDU — Unexpected Error",
                &format!("JDU crashed unexpectedly.\n\nDetails:\n{}", msg),
            );
        }
        std::process::exit(1);
    }));

    // ── Linux: confirm WebKitGTK is present before Tauri tries to use it ────
    #[cfg(target_os = "linux")]
    if !check_webkit_available() {
        show_error_dialog(
            "JDU — Missing Runtime",
            "WebKitGTK is not installed.\n\
             JDU requires WebKitGTK to display web content.\n\n\
             Install with:\n\
             • Ubuntu/Debian : sudo apt install libwebkit2gtk-4.0-37\n\
             • Fedora/RHEL   : sudo dnf install webkit2gtk4.0\n\
             • Arch Linux    : sudo pacman -S webkit2gtk",
        );
        std::process::exit(1);
    }

    // ── Windows: confirm WebView2 is present via the official wry API ──────── 
    #[cfg(target_os = "windows")]
    if !check_webview2_available() {
        show_error_dialog(
            "JDU — Missing Runtime",
            "Microsoft WebView2 Runtime is not installed.\n\n\
             JDU requires WebView2 to display web content.\n\n\
             Please download and install it from:\n\
             https://developer.microsoft.com/en-us/microsoft-edge/webview2/\n\n\
             Choose the 'Evergreen Bootstrapper' or 'Standalone Installer'.",
        );
        std::process::exit(1);
    }

    // ── macOS: WKWebView ships with the OS — no check needed ─────────────────
    // Built into macOS since 10.10 (Yosemite, 2014). Tauri would surface
    // a clear error on its own if it were somehow absent.

    // ── Start Tauri ──────────────────────────────────────────────────────────
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![open_website_window, update_window_title])
        .setup(|app| {
            // Guard against a missing "main" label in tauri.conf.json — that
            // causes a silent clean exit (event loop has no windows to run).
            match app.get_webview_window("main") {
                None => {
                    #[cfg(target_os = "windows")]
                    show_error_dialog(
                        "JDU — Config Error",
                        "No window labelled 'main' was found.\n\n\
                         Check that tauri.conf.json defines a window with:\n\
                         \"label\": \"main\"",
                    );
                    Err("No 'main' window defined in tauri.conf.json".into())
                }
                Some(main_window) => {
                    let _ = main_window.set_title("JDU - Jira Desktop Unofficial");
                    inject_drag_drop_fix(&main_window);
                    Ok(())
                }
            }
        })
        .run(tauri::generate_context!());

    // ── Handle .run() returning Err (e.g. WebView2 init failure) ────────────
    // Some WebView2 failures surface as Err rather than a panic.
    if let Err(e) = result {
        let msg = format!("{}", e);
        eprintln!("[JDU] fatal: {}", msg);

        let lower = msg.to_lowercase();
        let _is_webview2 = lower.contains("webview") || lower.contains("edge")
            || lower.contains("0x80070002") || lower.contains("class not registered")
            || lower.contains("runtime");

        #[cfg(target_os = "windows")]
        if _is_webview2 {
            show_error_dialog(
                "JDU — Missing Runtime",
                "Microsoft WebView2 Runtime is not installed or is corrupted.\n\n\
                 JDU requires WebView2 to display web content.\n\n\
                 Please download and install it from:\n\
                 https://developer.microsoft.com/en-us/microsoft-edge/webview2/\n\n\
                 Choose the 'Evergreen Bootstrapper' or 'Standalone Installer'.",
            );
        } else {
            #[cfg(target_os = "windows")]
            show_error_dialog(
                "JDU — Startup Error",
                &format!("JDU failed to start.\n\nDetails:\n{}", msg),
            );
        }

        std::process::exit(1);
    }
}