#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use tauri::{Manager, WebviewUrl};
use uuid::Uuid;
use std::thread;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::Mutex;
use once_cell::sync::Lazy;

// Store last titles to prevent duplicate updates
static LAST_TITLES: Lazy<Mutex<HashMap<String, String>>> = Lazy::new(|| Mutex::new(HashMap::new()));

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

#[tauri::command]
fn update_window_title(window: tauri::Window, title: String) {
    let window_label = window.label().to_string();
    
    // Check if title has changed to prevent spam
    let mut last_titles = LAST_TITLES.lock().unwrap();
    if let Some(last_title) = last_titles.get(&window_label) {
        if last_title == &title {
            return; // Skip duplicate update
        }
    }
    
    // Store new title
    last_titles.insert(window_label.clone(), title.clone());
    
    println!("📝 Updating title for window '{}' to: {}", window_label, title);
    if !title.is_empty() && title != "null" && title != "undefined" {
        let _ = window.set_title(&format!("{} - Jira Desktop", title));
    }
}

#[tauri::command]
async fn open_website_window(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let hostname = url.replace("https://", "").replace("http://", "").split('/').next().unwrap_or(&url).to_string();
    let window_id = format!("website-window-{}", Uuid::new_v4());

    // Hide the main window
    if let Some(main_window) = app.get_webview_window("main") {
        let _ = main_window.hide();
    }

    let parsed_url = url.parse().map_err(|e| format!("Invalid URL: {}", e))?;
    
    let builder = tauri::WebviewWindowBuilder::new(
        &app,
        &window_id,
        WebviewUrl::External(parsed_url),
    )
    .title(&format!("🔄 Loading {}...", hostname))
    .inner_size(1000.0, 700.0)
    .resizable(true)
    .visible(true)
    .decorations(true);

    match builder.build() {
        Ok(new_window) => {
            let app_handle = app.clone();
            let window_label = new_window.label().to_string();
            
            // Clean up stored title when window closes
            let window_label_clone = window_label.clone();
            let new_window_for_close = new_window.clone();
            new_window_for_close.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { .. } = event {
                    let mut last_titles = LAST_TITLES.lock().unwrap();
                    last_titles.remove(&window_label_clone);
                    
                    if let Some(main_window) = app_handle.get_webview_window("main") {
                        let _ = main_window.show();
                    }
                }
            });
            
            // Improved JavaScript with debouncing to prevent spam
            let js_code = r#"
                (function() {
                    console.log('Title updater script injected');
                    
                    let lastSentTitle = '';
                    let debounceTimer = null;
                    
                    function sendTitleToTauri() {
                        if (window.__TAURI__) {
                            let title = document.title;
                            if (title && title !== 'null' && title !== 'undefined' && title !== '') {
                                title = title.trim();
                                
                                // Don't send if title hasn't changed
                                if (title === lastSentTitle) {
                                    console.log('Title unchanged, skipping:', title);
                                    return;
                                }
                                
                                // Clear previous debounce timer
                                if (debounceTimer) {
                                    clearTimeout(debounceTimer);
                                }
                                
                                // Debounce: wait 100ms before sending
                                debounceTimer = setTimeout(() => {
                                    lastSentTitle = title;
                                    console.log('📄 Sending title to Tauri:', title);
                                    window.__TAURI__.core.invoke('update_window_title', { title }).catch(err => {
                                        console.error('Failed to send title:', err);
                                    });
                                }, 100);
                            }
                        } else {
                            console.log('Not in Tauri environment');
                        }
                    }
                    
                    // Send title on page load
                    if (document.readyState === 'complete') {
                        sendTitleToTauri();
                    } else {
                        document.addEventListener('DOMContentLoaded', sendTitleToTauri);
                        window.addEventListener('load', sendTitleToTauri);
                    }
                    
                    // Send only once after initial load (not multiple times)
                    setTimeout(sendTitleToTauri, 500);
                    
                    // Observe title changes with debouncing
                    const observer = new MutationObserver(() => {
                        console.log('Title mutation detected');
                        sendTitleToTauri();
                    });
                    
                    const titleElement = document.querySelector('title');
                    if (titleElement) {
                        observer.observe(titleElement, { 
                            subtree: true, 
                            characterData: true, 
                            childList: true 
                        });
                        console.log('Observing title element');
                    }
                    
                    // Watch for navigation (SPA) with debouncing
                    let lastUrl = location.href;
                    const urlObserver = new MutationObserver(() => {
                        const url = location.href;
                        if (url !== lastUrl) {
                            lastUrl = url;
                            console.log('URL changed, updating title');
                            sendTitleToTauri();
                        }
                    });
                    
                    // Only observe if document.body exists
                    if (document.body) {
                        urlObserver.observe(document.body, { subtree: true, childList: true });
                    }
                    
                    // Initial send
                    sendTitleToTauri();
                })();
            "#;
            
            // Inject script once (not multiple times)
            let new_window_clone = new_window.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(800));
                let _ = new_window_clone.eval(js_code);
            });
            
            Ok(())
        }
        Err(e) => Err(format!("Window creation failed: {}", e)),
    }
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
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_website_window,
            update_window_title  
        ])
        .setup(|app| {
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.set_title("Jira Desktop")?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri app");
}