mod workflow;
use tauri::{AppHandle, Manager, PhysicalSize, PhysicalPosition, Window};
use serde::{Deserialize, Serialize};
use std::{fs, time::Duration};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::menu::{Menu, MenuItem};
use tauri_plugin_notification::NotificationExt;

#[tauri::command]
fn begin_drag(window: Window) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

#[derive(Serialize)]
struct WidgetOrigin { x: i32, y: i32, scale: f64 }

#[tauri::command]
fn widget_origin(window: Window) -> Result<WidgetOrigin, String> {
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    Ok(WidgetOrigin { x: pos.x, y: pos.y, scale })
}

fn clamp_to_work_area(window: &Window, x: i32, y: i32) -> Result<PhysicalPosition<i32>, String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let cx = x + size.width as i32 / 2;
    let cy = y + size.height as i32 / 2;
    let on_monitor = |m: &tauri::Monitor| {
        let p = m.position();
        let s = m.size();
        cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
    };
    let monitor = monitors.iter().find(|m| on_monitor(m)).or_else(|| monitors.iter().min_by_key(|m| {
        let p = m.position();
        let s = m.size();
        let dx = p.x + s.width as i32 / 2 - cx;
        let dy = p.y + s.height as i32 / 2 - cy;
        (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64)
    }));
    let Some(monitor) = monitor else { return Ok(PhysicalPosition::new(x, y)); };
    let area = monitor.work_area();
    let left = area.position.x;
    let top = area.position.y;
    let right = left + area.size.width as i32 - size.width as i32;
    let bottom = top + area.size.height as i32 - size.height as i32;
    Ok(PhysicalPosition::new(x.clamp(left.min(right), left.max(right)), y.clamp(top.min(bottom), top.max(bottom))))
}

#[tauri::command]
fn move_widget(window: Window, x: i32, y: i32) -> Result<(), String> {
    let pos = clamp_to_work_area(&window, x, y)?;
    window.set_position(pos).map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_panel(window: Window) -> Result<bool, String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let expanded = size.width > 200;
    let next: PhysicalSize<u32> = if expanded { PhysicalSize::new(78, 78) } else { PhysicalSize::new(390, 620) };
    let old = window.outer_position().map_err(|e| e.to_string())?;
    let right = old.x + size.width as i32;
    let bottom = old.y + size.height as i32;
    window.set_size(next).map_err(|e| e.to_string())?;
    window.set_position(PhysicalPosition::new((right - next.width as i32).max(0), (bottom - next.height as i32).max(0))).map_err(|e| e.to_string())?;
    Ok(!expanded)
}

#[tauri::command]
fn test_notification(app: AppHandle, body: String) -> Result<(), String> {
    app.notification().builder().title("Engineering Copilot").body(body).show().map_err(|e| e.to_string())
}



#[derive(Clone, Serialize, Deserialize)]
struct ProviderSettings {
    provider: String,
    model: String,
    #[serde(default)]
    endpoint: String,
}

#[derive(Serialize)]
struct ProviderStatus {
    settings: ProviderSettings,
    has_key: bool,
}

fn config_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("provider.json"))
}

fn read_settings(app: &AppHandle) -> ProviderSettings {
    config_path(app).ok()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(ProviderSettings { provider: "openai".into(), model: "gpt-5.1".into(), endpoint: String::new() })
}

fn key_entry(provider: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("engineering-copilot-widget", provider).map_err(|e| e.to_string())
}

fn check_settings(s: &ProviderSettings) -> Result<(), String> {
    if !matches!(s.provider.as_str(), "openai" | "anthropic" | "custom") { return Err("Invalid provider".into()); }
    if s.model.trim().is_empty() || s.model.len() > 160 { return Err("Enter a valid model identifier".into()); }
    if s.provider == "custom" {
        let url = reqwest::Url::parse(s.endpoint.trim()).map_err(|_| "Invalid custom endpoint URL")?;
        let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]" | "::1"));
        if url.scheme() != "https" && !(url.scheme() == "http" && local) { return Err("Custom endpoints must use HTTPS (HTTP is only allowed for localhost)".into()); }
        if url.username() != "" || url.password().is_some() || url.fragment().is_some() { return Err("Endpoint must not contain credentials or fragments".into()); }
    }
    Ok(())
}

#[tauri::command]
fn get_provider_status(app: AppHandle) -> Result<ProviderStatus, String> {
    let settings = read_settings(&app);
    let has_key = key_entry(&settings.provider)?.get_password().is_ok();
    Ok(ProviderStatus { settings, has_key })
}

#[tauri::command]
fn save_provider_settings(app: AppHandle, settings: ProviderSettings, api_key: Option<String>) -> Result<ProviderStatus, String> {
    check_settings(&settings)?;
    if let Some(key) = api_key {
        if !key.trim().is_empty() { key_entry(&settings.provider)?.set_password(key.trim()).map_err(|e| format!("Could not save key to OS credential store: {e}"))?; }
    }
    fs::write(config_path(&app)?, serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    get_provider_status(app)
}

#[tauri::command]
fn delete_provider_key(app: AppHandle, provider: String) -> Result<ProviderStatus, String> {
    if !matches!(provider.as_str(), "openai" | "anthropic" | "custom") { return Err("Invalid provider".into()); }
    // A missing key is equivalent to success.
    let entry = key_entry(&provider)?;
    if entry.get_password().is_ok() { entry.delete_credential().map_err(|e| e.to_string())?; }
    get_provider_status(app)
}

#[tauri::command]
async fn ask_ai(app: AppHandle, prompt: String) -> Result<String, String> {
    let settings = read_settings(&app);
    check_settings(&settings)?;
    if prompt.trim().is_empty() || prompt.len() > 20000 { return Err("Prompt must be between 1 and 20,000 characters".into()); }
    let key = key_entry(&settings.provider)?.get_password().map_err(|_| "No API key saved for selected provider")?;
    let client = reqwest::Client::builder().timeout(Duration::from_secs(45)).build().map_err(|e| e.to_string())?;
    let is_anthropic = settings.provider == "anthropic";
    let url = match settings.provider.as_str() {
        "openai" => "https://api.openai.com/v1/chat/completions".to_string(),
        "anthropic" => "https://api.anthropic.com/v1/messages".to_string(),
        _ => settings.endpoint.clone(),
    };
    let body = if is_anthropic {
        serde_json::json!({"model": settings.model, "max_tokens": 1024, "messages": [{"role": "user", "content": prompt}]})
    } else {
        serde_json::json!({"model": settings.model, "messages": [{"role": "user", "content": prompt}]})
    };
    let req = client.post(url).json(&body);
    let req = if is_anthropic { req.header("x-api-key", key).header("anthropic-version", "2023-06-01") }
              else if settings.provider == "custom" && key == "NO_KEY" { req }
              else { req.bearer_auth(key) };
    let resp = req.send().await.map_err(|e| format!("Connection failed: {e}"))?;
    let status = resp.status();
    if !status.is_success() { return Err(format!("Provider returned HTTP {}. Check model, key, endpoint and available credits.", status.as_u16())); }
    let v: serde_json::Value = resp.json().await.map_err(|_| "Provider returned an unexpected response format")?;
    let answer = if is_anthropic {
        v["content"].as_array().map(|items| items.iter().filter_map(|x| x["text"].as_str()).collect::<Vec<_>>().join("\n"))
    } else {
        v["choices"][0]["message"]["content"].as_str().map(str::to_string)
    };
    answer.filter(|x| !x.trim().is_empty()).ok_or_else(|| "Provider returned no text (model may not support this API format)".into())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "Show widget", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Engineering Copilot", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let mut tray = TrayIconBuilder::new().menu(&menu).show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() { tray = tray.icon(icon.clone()); }
            tray.on_menu_event(|app, event| {
                if event.id() == "show" { if let Some(win) = app.get_webview_window("main") { let _ = win.show(); let _ = win.set_focus(); } }
                else if event.id() == "quit" { app.exit(0); }
            }).on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                    if let Some(win) = tray.app_handle().get_webview_window("main") { let _ = win.show(); let _ = win.set_focus(); }
                }
            }).build(app)?;
            let _ = tray;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_background_color(Some(tauri::window::Color(0, 0, 0, 0)));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![begin_drag, widget_origin, move_widget, toggle_panel, test_notification, get_provider_status, save_provider_settings, delete_provider_key, ask_ai, workflow::jira_status, workflow::configure_jira, workflow::list_jira_tickets, workflow::import_jira_ticket, workflow::workflow_status, workflow::discuss_ticket, workflow::generate_ticket_plan, workflow::revise_ticket_plan, workflow::approve_ticket_plan, workflow::inspect_repo, workflow::start_ticket_work])
        .run(tauri::generate_context!())
        .expect("error while running Engineering Copilot");
}
