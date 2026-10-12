#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::webview::{NewWindowFeatures, NewWindowResponse};
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

const JIRA_WINDOW_PREFIX: &str = "website-window-";
const NEW_TAB_MENU_ID: &str = "new-tab";

// WebKit check function for Linux - Version agnostic
#[cfg(target_os = "linux")]
fn check_webkit_available() -> bool {
    use std::process::Command;
    
    let version_patterns = ["webkit2gtk", "webkit2gtk-4", "webkit2gtk-4.0", "webkit2gtk-4.1", "webkit2gtk-5"];
    
    for pattern in &version_patterns {
        if Command::new("pkg-config")
            .args(["--exists", pattern])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    
    if let Ok(output) = Command::new("sh")
        .args(["-c", "find /usr/lib /usr/lib64 -name 'libwebkit2gtk*.so*' 2>/dev/null | head -1"])
        .output()
    {
        if !output.stdout.is_empty() {
            return true;
        }
    }
    
    if let Ok(output) = Command::new("ldconfig")
        .args(["-p"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("webkit") {
            return true;
        }
    }
    
    false
}

#[cfg(not(target_os = "linux"))]
fn check_webkit_available() -> bool {
    true
}

/// Links to the opener's host or to Atlassian open in a new tab. Other links open in the default browser.
fn opens_in_tab(url: &Url, opener_url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return true;
    }
    let Some(host) = url.host_str() else {
        return true;
    };
    let is_atlassian = ["atlassian.net", "atlassian.com"]
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    is_atlassian || opener_url.host_str() == Some(host)
}

/// Builds a Jira window. Pass `features` when the window answers a new window request from a page.
fn build_jira_window(
    app: &AppHandle,
    url: WebviewUrl,
    features: Option<NewWindowFeatures>,
) -> tauri::Result<WebviewWindow> {
    let label = format!("{JIRA_WINDOW_PREFIX}{}", Uuid::new_v4());
    let app_handle = app.clone();
    let opener_label = label.clone();

    let mut builder = WebviewWindowBuilder::new(app, &label, url)
        .title("Jira")
        .inner_size(1000.0, 700.0)
        .resizable(true)
        .on_document_title_changed(|window, title| {
            let _ = window.set_title(&format!("{title} - Jira"));
        })
        .on_new_window(move |url, features| {
            let Some(opener) = app_handle.get_webview_window(&opener_label) else {
                return NewWindowResponse::Deny;
            };
            let in_tab = opener.url().map_or(false, |opener_url| opens_in_tab(&url, &opener_url));
            if !in_tab {
                if let Err(e) = app_handle.opener().open_url(url.as_str(), None::<&str>) {
                    eprintln!("Failed to open {url} in browser: {e}");
                }
                return NewWindowResponse::Deny;
            }
            let blank = WebviewUrl::External("about:blank".parse().unwrap());
            match build_jira_window(&app_handle, blank, Some(features)) {
                Ok(window) => {
                    add_as_tab(&opener, &window);
                    NewWindowResponse::Create { window }
                }
                Err(e) => {
                    eprintln!("Failed to open new tab: {e}");
                    NewWindowResponse::Deny
                }
            }
        });

    if let Some(features) = features {
        builder = builder.window_features(features);
    }
    #[cfg(target_os = "macos")]
    {
        builder = builder.tabbing_identifier("jira");
    }
    builder.build()
}

/// Shows `tab` as a native tab of `opener`. Only macOS has native window tabs.
#[cfg(target_os = "macos")]
fn add_as_tab(opener: &WebviewWindow, tab: &WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowOrderingMode};

    let (opener, tab) = (opener.clone(), tab.clone());
    let main_thread_opener = opener.clone();
    let _ = main_thread_opener.run_on_main_thread(move || {
        let (Ok(opener_ptr), Ok(tab_ptr)) = (opener.ns_window(), tab.ns_window()) else {
            return;
        };
        // SAFETY: Tauri gives valid NSWindow pointers, and this closure runs on the main thread.
        let (opener, tab) = unsafe { (&*opener_ptr.cast::<NSWindow>(), &*tab_ptr.cast::<NSWindow>()) };
        opener.addTabbedWindow_ordered(tab, NSWindowOrderingMode::Above);
    });
}

#[cfg(not(target_os = "macos"))]
fn add_as_tab(_opener: &WebviewWindow, _tab: &WebviewWindow) {}

/// Opens the page of the focused Jira window in a new tab.
fn open_new_tab(app: &AppHandle) {
    let Some(current) = app
        .webview_windows()
        .into_values()
        .find(|w| w.label().starts_with(JIRA_WINDOW_PREFIX) && w.is_focused().unwrap_or(false))
    else {
        return;
    };
    let Ok(url) = current.url() else {
        return;
    };
    match build_jira_window(app, WebviewUrl::External(url), None) {
        Ok(tab) => add_as_tab(&current, &tab),
        Err(e) => eprintln!("Failed to open new tab: {e}"),
    }
}

#[tauri::command]
async fn open_website_window(app: AppHandle, url: String) -> Result<(), String> {
    let parsed_url = url.parse().map_err(|e| format!("Invalid URL: {}", e))?;
    build_jira_window(&app, WebviewUrl::External(parsed_url), None)
        .map_err(|e| format!("Window creation failed: {}", e))?;

    // Hide the launcher while Jira windows are open
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.hide();
    }
    Ok(())
}

fn main() {
    #[cfg(target_os = "linux")]
    {
        if !check_webkit_available() {
            eprintln!("ERROR: WebKitGTK not found!");
            eprintln!("This application requires WebKitGTK to display web content.");
            eprintln!("");
            eprintln!("Please install with one of these commands:");
            eprintln!("");
            eprintln!("  Ubuntu/Debian:");
            eprintln!("    sudo apt install libwebkit2gtk-4.0-37");
            eprintln!("");
            eprintln!("  Fedora/RHEL:");
            eprintln!("    sudo dnf install webkit2gtk4.0");
            eprintln!("");
            eprintln!("  Arch Linux:");
            eprintln!("    sudo pacman -S webkit2gtk");
            eprintln!("");
            std::process::exit(1);
        }
    }

    tauri::Builder::default()
        // The default link script cancels target="_blank" clicks and calls IPC, which Jira pages cannot use.
        // Without it, the links get to on_new_window.
        .plugin(
            tauri_plugin_opener::Builder::new()
                .open_js_links_on_click(false)
                .build(),
        )
        .invoke_handler(tauri::generate_handler![open_website_window])
        .on_menu_event(|app, event| {
            if event.id() == NEW_TAB_MENU_ID {
                open_new_tab(app);
            }
        })
        .on_window_event(|window, event| {
            // Show the launcher again when the last Jira window closes
            if let tauri::WindowEvent::Destroyed = event {
                let label = window.label();
                if !label.starts_with(JIRA_WINDOW_PREFIX) {
                    return;
                }
                let app = window.app_handle();
                let jira_window_open = app
                    .webview_windows()
                    .keys()
                    .any(|other| other.starts_with(JIRA_WINDOW_PREFIX) && other != label);
                if !jira_window_open {
                    if let Some(main_window) = app.get_webview_window("main") {
                        let _ = main_window.show();
                    }
                }
            }
        })
        .setup(|app| {
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.set_title("Jira")?;
            }
            // Add "New Tab" (⌘T) to the File menu. ⌘W in the default menu closes a tab.
            #[cfg(target_os = "macos")]
            if let Some(menu) = app.menu() {
                let file_menu = menu.items()?.into_iter().find_map(|item| {
                    item.as_submenu()
                        .filter(|submenu| submenu.text().is_ok_and(|text| text == "File"))
                        .cloned()
                });
                if let Some(file_menu) = file_menu {
                    let new_tab = tauri::menu::MenuItem::with_id(
                        app,
                        NEW_TAB_MENU_ID,
                        "New Tab",
                        true,
                        Some("CmdOrCtrl+T"),
                    )?;
                    file_menu.prepend(&new_tab)?;
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}

#[cfg(test)]
mod tests {
    use super::opens_in_tab;

    fn check(url: &str, opener: &str) -> bool {
        opens_in_tab(&url.parse().unwrap(), &opener.parse().unwrap())
    }

    #[test]
    fn jira_ticket_link_opens_in_tab() {
        assert!(check(
            "https://acme.atlassian.net/browse/ABC-1",
            "https://acme.atlassian.net/jira/software/projects/ABC/boards/1"
        ));
    }

    #[test]
    fn same_host_on_self_hosted_jira_opens_in_tab() {
        assert!(check("https://jira.acme.com/browse/ABC-1", "https://jira.acme.com/secure/Dashboard.jspa"));
    }

    #[test]
    fn other_atlassian_hosts_open_in_tab() {
        let opener = "https://acme.atlassian.net/browse/ABC-1";
        assert!(check("https://id.atlassian.com/manage-profile", opener));
        assert!(check("https://other.atlassian.net/wiki", opener));
    }

    #[test]
    fn blank_page_opens_in_tab() {
        assert!(check("about:blank", "https://acme.atlassian.net/browse/ABC-1"));
    }

    #[test]
    fn external_link_opens_in_browser() {
        let opener = "https://acme.atlassian.net/browse/ABC-1";
        assert!(!check("https://github.com/acme/repo/pull/1", opener));
        assert!(!check("https://evil-atlassian.net/", opener));
    }
}
