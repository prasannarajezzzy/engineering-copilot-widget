mod github;
mod meeting;
mod workflow;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, Size, Window};
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

#[derive(Serialize)]
struct MenuPlace { corner: String }

#[tauri::command]
fn set_bubble_menu(window: Window, open: bool, corner: Option<String>) -> Result<MenuPlace, String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let x = pos.x as f64 / scale;
    let y = pos.y as f64 / scale;
    let w = size.width as f64 / scale;
    let h = size.height as f64 / scale;
    if w > 200.0 {
        return Ok(MenuPlace { corner: corner.unwrap_or_else(|| "br".into()) });
    }
    let chosen = if open {
        let monitors = window.available_monitors().map_err(|e| e.to_string())?;
        let cx = pos.x + size.width as i32 / 2;
        let cy = pos.y + size.height as i32 / 2;
        let monitor = monitors.iter().find(|m| {
            let p = m.position();
            let s = m.size();
            cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
        }).or_else(|| monitors.first());
        let (left, top, right, bottom) = if let Some(m) = monitor {
            let area = m.work_area();
            (
                area.position.x as f64 / scale,
                area.position.y as f64 / scale,
                (area.position.x as f64 + area.size.width as f64) / scale,
                (area.position.y as f64 + area.size.height as f64) / scale,
            )
        } else {
            (0.0, 0.0, x + w + 800.0, y + h + 800.0)
        };
        let extra_w = 168.0 - 78.0;
        let extra_h = 120.0 - 78.0;
        let grow_left = (x - left) >= extra_w || (x - left) >= (right - (x + w));
        let grow_up = (y - top) >= extra_h || (y - top) >= (bottom - (y + h));
        match (grow_up, grow_left) {
            (true, true) => "br",
            (true, false) => "bl",
            (false, true) => "tr",
            _ => "tl",
        }.to_string()
    } else {
        corner.unwrap_or_else(|| "br".into())
    };
    let (nx, ny, next_w, next_h) = if open {
        let (nx, ny) = match chosen.as_str() {
            "br" => (x + w - 168.0, y + h - 120.0),
            "bl" => (x, y + h - 120.0),
            "tr" => (x + w - 168.0, y),
            _ => (x, y),
        };
        (nx, ny, 168.0, 120.0)
    } else {
        let (nx, ny) = match chosen.as_str() {
            "br" => (x + w - 78.0, y + h - 78.0),
            "bl" => (x, y + h - 78.0),
            "tr" => (x + w - 78.0, y),
            _ => (x, y),
        };
        (nx, ny, 78.0, 78.0)
    };
    window.set_position(tauri::LogicalPosition::new(nx, ny)).map_err(|e| e.to_string())?;
    window.set_size(tauri::LogicalSize::new(next_w, next_h)).map_err(|e| e.to_string())?;
    Ok(MenuPlace { corner: chosen })
}

struct WidgetSpot { right: i32, bottom: i32 }
struct DeskSpot { x: f64, y: f64, w: f64, h: f64 }
static WIDGET_SPOT: Mutex<Option<WidgetSpot>> = Mutex::new(None);
static DESK_SPOT: Mutex<Option<DeskSpot>> = Mutex::new(None);

fn work_logical(window: &Window) -> Result<(f64, f64, f64, f64), String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let cx = pos.x + size.width as i32 / 2;
    let cy = pos.y + size.height as i32 / 2;
    let monitor = monitors.iter().find(|m| {
        let p = m.position();
        let s = m.size();
        cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
    }).or_else(|| monitors.first());
    let Some(monitor) = monitor else { return Ok((0.0, 0.0, 1280.0, 800.0)); };
    let area = monitor.work_area();
    Ok((
        area.position.x as f64 / scale,
        area.position.y as f64 / scale,
        area.size.width as f64 / scale,
        area.size.height as f64 / scale,
    ))
}

fn work_physical(window: &Window) -> Result<(i32, i32, i32, i32), String> {
    let pos = window.outer_position().map_err(|e| e.to_string())?;
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let monitors = window.available_monitors().map_err(|e| e.to_string())?;
    let cx = pos.x + size.width as i32 / 2;
    let cy = pos.y + size.height as i32 / 2;
    let monitor = monitors.iter().find(|m| {
        let p = m.position();
        let s = m.size();
        cx >= p.x && cy >= p.y && cx < p.x + s.width as i32 && cy < p.y + s.height as i32
    }).or_else(|| monitors.first());
    let Some(monitor) = monitor else { return Ok((0, 0, 1280, 800)); };
    let area = monitor.work_area();
    Ok((area.position.x, area.position.y, area.size.width as i32, area.size.height as i32))
}

fn remember_spots(window: &Window) {
    if window.is_fullscreen().unwrap_or(false) { return; }
    let Ok(pos) = window.outer_position() else { return };
    let Ok(size) = window.outer_size() else { return };
    if window.is_resizable().unwrap_or(false) {
        let Ok(scale) = window.scale_factor() else { return };
        if let Ok(mut spot) = DESK_SPOT.lock() {
            *spot = Some(DeskSpot {
                x: pos.x as f64 / scale,
                y: pos.y as f64 / scale,
                w: size.width as f64 / scale,
                h: size.height as f64 / scale,
            });
        }
    } else if let Ok(mut spot) = WIDGET_SPOT.lock() {
        *spot = Some(WidgetSpot { right: pos.x + size.width as i32, bottom: pos.y + size.height as i32 });
    }
}

fn place_floating(window: &Window, width: u32, height: u32) -> Result<(), String> {
    let (left, top, area_w, area_h) = work_physical(window)?;
    let (right, bottom) = WIDGET_SPOT.lock().ok().and_then(|spot| spot.as_ref().map(|s| (s.right, s.bottom)))
        .unwrap_or((left + area_w - 16, top + area_h - 16));
    let max_x = left + area_w - width as i32;
    let max_y = top + area_h - height as i32;
    let x = (right - width as i32).clamp(left.min(max_x), left.max(max_x));
    let y = (bottom - height as i32).clamp(top.min(max_y), top.max(max_y));
    window.set_size(PhysicalSize::new(width, height)).map_err(|e| e.to_string())?;
    window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())
}

fn place_desk(window: &Window) -> Result<(), String> {
    let (ax, ay, aw, ah) = work_logical(window)?;
    let saved = DESK_SPOT.lock().ok().and_then(|spot| spot.as_ref().map(|s| (s.x, s.y, s.w, s.h)));
    let (x, y, w, h) = if let Some((x, y, w, h)) = saved {
        let w = w.clamp(420.0, aw.max(420.0));
        let h = h.clamp(480.0, ah.max(480.0));
        let x = x.clamp(ax, (ax + aw - w).max(ax));
        let y = y.clamp(ay, (ay + ah - h).max(ay));
        (x, y, w, h)
    } else {
        let margin = 16.0;
        let w = (aw * 0.42).clamp(420.0, (aw - margin * 2.0).max(420.0));
        let h = (ah - margin * 2.0).clamp(480.0, ah.max(480.0));
        (ax + aw - w - margin, ay + margin, w, h)
    };
    window.set_min_size(Some(Size::Logical(tauri::LogicalSize::new(420.0, 480.0)))).map_err(|e| e.to_string())?;
    window.set_size(tauri::LogicalSize::new(w, h)).map_err(|e| e.to_string())?;
    window.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| e.to_string())
}

#[tauri::command]
fn place_window(window: Window, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let (ax, ay, aw, ah) = work_logical(&window)?;
    let width = width.clamp(420.0, aw.max(420.0));
    let height = height.clamp(480.0, ah.max(480.0));
    let x = x.clamp(ax, (ax + aw - width).max(ax));
    let y = y.clamp(ay, (ay + ah - height).max(ay));
    window.set_position(tauri::LogicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    window.set_size(tauri::LogicalSize::new(width, height)).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_window_mode(window: Window, mode: String) -> Result<String, String> {
    if !matches!(mode.as_str(), "bubble" | "widget" | "desk" | "full") {
        return Err("Unknown window mode.".into());
    }
    remember_spots(&window);
    if window.is_fullscreen().unwrap_or(false) {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
    }
    match mode.as_str() {
        "bubble" => {
            window.set_min_size(Some(Size::Logical(tauri::LogicalSize::new(78.0, 78.0)))).map_err(|e| e.to_string())?;
            window.set_resizable(false).map_err(|e| e.to_string())?;
            window.set_always_on_top(true).map_err(|e| e.to_string())?;
            window.set_skip_taskbar(true).map_err(|e| e.to_string())?;
            place_floating(&window, 78, 78)?;
        }
        "widget" => {
            window.set_min_size(Some(Size::Logical(tauri::LogicalSize::new(78.0, 78.0)))).map_err(|e| e.to_string())?;
            window.set_resizable(false).map_err(|e| e.to_string())?;
            window.set_always_on_top(true).map_err(|e| e.to_string())?;
            window.set_skip_taskbar(true).map_err(|e| e.to_string())?;
            place_floating(&window, 390, 620)?;
        }
        "desk" => {
            window.set_resizable(true).map_err(|e| e.to_string())?;
            window.set_always_on_top(false).map_err(|e| e.to_string())?;
            window.set_skip_taskbar(false).map_err(|e| e.to_string())?;
            place_desk(&window)?;
        }
        "full" => {
            window.set_resizable(true).map_err(|e| e.to_string())?;
            window.set_always_on_top(false).map_err(|e| e.to_string())?;
            window.set_skip_taskbar(false).map_err(|e| e.to_string())?;
            window.set_fullscreen(true).map_err(|e| e.to_string())?;
        }
        _ => {}
    }
    let _ = window.set_focus();
    Ok(mode)
}

#[tauri::command]
fn toggle_panel(window: Window) -> Result<bool, String> {
    if window.is_fullscreen().unwrap_or(false) {
        window.set_fullscreen(false).map_err(|e| e.to_string())?;
    }
    if window.is_resizable().unwrap_or(false) {
        window.set_resizable(false).map_err(|e| e.to_string())?;
        window.set_always_on_top(true).map_err(|e| e.to_string())?;
        window.set_skip_taskbar(true).map_err(|e| e.to_string())?;
        window.set_min_size(Some(Size::Logical(tauri::LogicalSize::new(78.0, 78.0)))).map_err(|e| e.to_string())?;
    }
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let expanded = size.width as f64 / scale > 200.0;
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

pub(crate) fn key_entry(provider: &str) -> Result<keyring::Entry, String> {
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

pub(crate) async fn complete_prompt(app: &AppHandle, prompt: &str, max_chars: usize) -> Result<String, String> {
    let settings = read_settings(app);
    check_settings(&settings)?;
    if prompt.trim().is_empty() || prompt.len() > max_chars { return Err(format!("Prompt must be between 1 and {max_chars} characters")); }
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

#[tauri::command]
async fn ask_ai(app: AppHandle, prompt: String) -> Result<String, String> {
    complete_prompt(&app, &prompt, 20_000).await
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
        .invoke_handler(tauri::generate_handler![begin_drag, widget_origin, move_widget, toggle_panel, test_notification, get_provider_status, save_provider_settings, delete_provider_key, ask_ai, workflow::jira_status, workflow::configure_jira, workflow::list_jira_tickets, workflow::import_jira_ticket, workflow::workflow_status, workflow::discuss_ticket, workflow::generate_ticket_plan, workflow::revise_ticket_plan, workflow::approve_ticket_plan, workflow::inspect_repo, workflow::start_ticket_work, github::github_status, github::save_github_token, github::clear_github_token, github::list_github_pulls, meeting::start_meeting, meeting::stop_meeting, meeting::list_meetings, meeting::meeting_listening, set_bubble_menu, set_window_mode, place_window])
        .run(tauri::generate_context!())
        .expect("error while running Engineering Copilot");
}
