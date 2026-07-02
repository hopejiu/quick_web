use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use log::{error, info, warn};
use tauri::{
    ipc::InvokeError,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Listener, Manager, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::ShortcutState;

mod hotkey;
mod logger;
mod script_injector;
mod settings;
mod window_manager;

use hotkey::Hotkey;
use window_manager::WindowManager;

// ── 应用状态 ──

struct AppState {
    settings: Mutex<settings::Settings>,
    hotkey_map: Mutex<HashMap<String, String>>,
    window_manager: WindowManager,
    hide_timers: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

// ── 窗口工具 ──

fn bring_to_front(window: &tauri::WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// 启动窗口销毁定时器（20 秒后销毁隐藏的窗口）
/// 通过事件通知主线程执行 close，避免跨线程操作窗口
fn start_destroy_timer(app: &AppHandle, label: String) {
    let app = app.clone();
    std::thread::spawn(move || {
        info!("destroy timer started for {} (20s)", label);
        std::thread::sleep(Duration::from_secs(20));
        if app.get_webview_window(&label).is_some() {
            info!("destroy timer fired, emitting close-request for {}", label);
            if let Err(e) = app.emit("request-close-window", &label) {
                error!("failed to emit close-request: {}", e);
            }
        } else {
            info!("destroy timer: {} already gone", label);
        }
    });
}

/// 创建或显示站点窗口
fn show_site_window(app: &AppHandle, entry: &settings::SiteEntry) -> tauri::WebviewWindow {
    let label = format!("site-{}", entry.id);
    if let Some(win) = app.get_webview_window(&label) {
        bring_to_front(&win);
        return win;
    }

    info!("creating site window for {}", entry.name);
    let win = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(entry.url.parse().unwrap()),
    )
    .title(&entry.name)
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .build()
    .expect("failed to create site window");

    let win_clone = win.clone();
    let app_clone = app.clone();
    let label_clone = label.clone();
    win.on_window_event(move |event| {
        match event {
            WindowEvent::CloseRequested { api, .. } => {
                let should_close = app_clone
                    .state::<AppState>()
                    .window_manager
                    .take_closing(&label_clone);

                if should_close {
                    info!(
                        "window {} close requested by timer, allowing destroy",
                        label_clone
                    );
                } else {
                    info!(
                        "window {} close requested by user, hiding instead",
                        label_clone
                    );
                    api.prevent_close();
                    if let Err(e) = win_clone.hide() {
                        error!("failed to hide window {}: {}", label_clone, e);
                    } else {
                        start_destroy_timer(&app_clone, label_clone.clone());
                    }
                }
            }
            WindowEvent::Resized(size) if size.width == 0 && size.height == 0 => {
                info!("window {} minimized, hiding instead", label_clone);
                if let Err(e) = win_clone.hide() {
                    error!("failed to hide window {}: {}", label_clone, e);
                } else {
                    start_destroy_timer(&app_clone, label_clone.clone());
                }
            }
            WindowEvent::Focused(false) => {
                info!(
                    "window {} lost focus, starting 10s hide timer",
                    label_clone
                );
                // 取消该窗口之前的 pending 定时器
                if let Some(old_cancel) = app_clone
                    .state::<AppState>()
                    .hide_timers
                    .lock()
                    .unwrap()
                    .remove(&label_clone)
                {
                    old_cancel.store(true, Ordering::Relaxed);
                }
                let cancel = Arc::new(AtomicBool::new(false));
                app_clone
                    .state::<AppState>()
                    .hide_timers
                    .lock()
                    .unwrap()
                    .insert(label_clone.clone(), cancel.clone());
                let app = app_clone.clone();
                let label = label_clone.clone();
                let win = win_clone.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(10));
                    if !cancel.load(Ordering::Relaxed) {
                        info!("10s blur timer fired, hiding window {}", label);
                        if let Err(e) = win.hide() {
                            error!("failed to hide window {}: {}", label, e);
                        } else {
                            start_destroy_timer(&app, label);
                        }
                    }
                });
            }
            WindowEvent::Focused(true) => {
                // 窗口重新获得焦点，取消 pending 定时器
                if let Some(cancel) = app_clone
                    .state::<AppState>()
                    .hide_timers
                    .lock()
                    .unwrap()
                    .get(&label_clone)
                {
                    cancel.store(true, Ordering::Relaxed);
                }
            }
            _ => {}
        }
    });

    let _ = win.show();
    let _ = win.set_focus();

    script_injector::schedule_injection(app, entry);
    win
}

/// 隐藏站点窗口
fn hide_site_window(app: &AppHandle, entry_id: &str) {
    let label = format!("site-{}", entry_id);
    if let Some(win) = app.get_webview_window(&label) {
        info!("hiding window {}", label);
        if let Err(e) = win.hide() {
            error!("failed to hide window {}: {}", label, e);
        } else {
            start_destroy_timer(app, label);
        }
    }
}

/// 切换站点窗口显示
fn toggle_site_window(app: &AppHandle, entry: &settings::SiteEntry) {
    let label = format!("site-{}", entry.id);
    if let Some(win) = app.get_webview_window(&label) {
        if win.is_visible().unwrap_or(false) {
            hide_site_window(app, &entry.id);
        } else {
            info!("showing hidden window {}", label);
            bring_to_front(&win);
        }
    } else {
        info!("no existing window {}, creating new", label);
        show_site_window(app, entry);
    }
}

// ── Raw shortcut → String （global-shortcut 处理器用） ──

fn raw_shortcut_to_string(shortcut: &tauri_plugin_global_shortcut::Shortcut) -> String {
    let hk = Hotkey {
        mods: shortcut.mods,
        code: shortcut.key,
    };
    hk.to_string()
}

// ── 结果类型（#2 职责分离） ──

#[derive(Debug)]
pub enum SaveError {
    Io(String),
    HotkeyFailures(Vec<(String, String)>),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "保存失败: {}", e),
            Self::HotkeyFailures(failures) => {
                let msg = failures
                    .iter()
                    .map(|(hk, e)| format!("{}: {}", hk, e))
                    .collect::<Vec<_>>()
                    .join("; ");
                write!(f, "设置已保存，但以下快捷键注册失败: {}", msg)
            }
        }
    }
}

impl From<SaveError> for InvokeError {
    fn from(e: SaveError) -> Self {
        InvokeError::from(e.to_string())
    }
}

// ── Tauri Commands ──

#[tauri::command]
fn check_hotkey(
    app: AppHandle,
    state: tauri::State<AppState>,
    hotkey_str: String,
    editing_id: Option<String>,
) -> Result<(), String> {
    let hk = Hotkey::from_str(&hotkey_str).map_err(|e| e.to_string())?;
    let map = state.hotkey_map.lock().unwrap();
    hotkey::check_hotkey(app.clone(), &map, &hk, editing_id.as_deref())
}

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: tauri::State<AppState>,
    data: settings::Settings,
) -> Result<(), SaveError> {
    let old = {
        let mut s = state.settings.lock().unwrap();
        let old = s.clone();
        *s = data.clone();
        old
    };
    settings::save(&data).map_err(SaveError::Io)?;

    // Autostart 同步
    let autostart = app.autolaunch();
    if data.auto_start {
        let _ = autostart.enable();
    } else {
        let _ = autostart.disable();
    }

    // 热键 diff 同步
    let mut hotkey_map = state.hotkey_map.lock().unwrap();
    let outcome = hotkey::sync_hotkeys(app.clone(), &mut hotkey_map, &data);
    let failures = outcome.failures;
    std::mem::drop(hotkey_map);

    // 销毁已删除条目的窗口
    for old_entry in &old.entries {
        if !data.entries.iter().any(|e| e.id == old_entry.id) {
            let label = format!("site-{}", old_entry.id);
            if let Some(win) = app.get_webview_window(&label) {
                let _ = win.close();
            }
        }
    }

    app.emit("settings-saved", &data).ok();
    info!("settings saved");

    if failures.is_empty() {
        Ok(())
    } else {
        Err(SaveError::HotkeyFailures(failures))
    }
}

#[tauri::command]
fn open_script_dir(dir_name: String) -> Result<(), String> {
    let dir = settings::script_dir_path(&dir_name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    std::process::Command::new("explorer")
        .arg(&dir)
        .spawn()
        .map_err(|e| format!("打开目录失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn toggle_window(
    app: AppHandle,
    state: tauri::State<AppState>,
    entry_id: String,
) -> Result<(), String> {
    let settings = state.settings.lock().unwrap();
    let entry = settings
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| format!("entry not found: {}", entry_id))?;
    std::mem::drop(settings);
    toggle_site_window(&app, &entry);
    Ok(())
}

#[tauri::command]
fn show_settings_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        bring_to_front(&win);
    }
}

#[tauri::command]
fn inject_scripts(app: AppHandle, entry_id: String) -> Result<(), String> {
    script_injector::inject_once(&app, &entry_id)
}

#[tauri::command]
fn release_embedded_scripts(app: AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    script_injector::release_embedded_scripts(&app, &state)
}

// ── 应用入口 ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cfg_dir = settings::config_dir();
    let log_dir = cfg_dir.join("logs");
    let dev_mode = cfg!(debug_assertions);
    logger::init(log_dir, dev_mode).ok();
    info!("app started, config dir: {:?}", cfg_dir);

    let initial_settings = settings::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let hk_str = raw_shortcut_to_string(shortcut);
                    if let Some(state) = app.try_state::<AppState>() {
                        let map = state.hotkey_map.lock().unwrap();
                        if let Some(entry_id) = map.get(&hk_str) {
                            let entry_id = entry_id.clone();
                            std::mem::drop(map);
                            let settings = state.settings.lock().unwrap();
                            let entry = settings.entries.iter().find(|e| e.id == entry_id).cloned();
                            std::mem::drop(settings);
                            if let Some(e) = entry {
                                toggle_site_window(app, &e);
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            // 窗口销毁事件监听（定时器到时间后发事件，主线程执行 close）
            let handle = app.handle().clone();
            app.listen("request-close-window", move |event| {
                let label = event.payload().trim_matches('"');
                info!("received request-close-window for {}", label);
                if let Some(win) = handle.get_webview_window(label) {
                    if !win.is_visible().unwrap_or(false) {
                        info!("marking {} for timer-triggered close", label);
                        handle
                            .state::<AppState>()
                            .window_manager
                            .mark_closing(label);
                        info!("closing window {} (destroy timer)", label);
                        if let Err(e) = win.close() {
                            error!("failed to close window {}: {}", label, e);
                        }
                    } else {
                        info!("window {} is now visible, skip close", label);
                    }
                } else {
                    info!("window {} not found, already closed?", label);
                }
            });

            // Autostart
            app.handle().plugin(tauri_plugin_autostart::init(
                tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                None,
            ))?;

            // 初始热键注册
            let mut hotkey_map = HashMap::<String, String>::new();
            let registration_outcome =
                { hotkey::sync_hotkeys(app.handle().clone(), &mut hotkey_map, &initial_settings) };
            if !registration_outcome.failures.is_empty() {
                warn!(
                    "initial hotkey registration failures: {:?}",
                    registration_outcome.failures
                );
            }

            // 状态
            let initial_settings_for_state = initial_settings.clone();
            app.manage(AppState {
                settings: Mutex::new(initial_settings_for_state),
                hotkey_map: Mutex::new(hotkey_map),
                window_manager: WindowManager::new(),
                hide_timers: Mutex::new(HashMap::new()),
            });

            // 设置窗口
            let settings_win =
                WebviewWindowBuilder::new(app, "settings", WebviewUrl::App("index.html".into()))
                    .title("设置")
                    .inner_size(700.0, 620.0)
                    .min_inner_size(600.0, 400.0)
                    .visible(false)
                    .build()?;
            let sw = settings_win.clone();
            settings_win.on_window_event(move |event| match event {
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = sw.hide();
                }
                WindowEvent::Resized(size) if size.width == 0 && size.height == 0 => {
                    info!("settings window minimized, hiding instead");
                    let _ = sw.hide();
                }
                _ => {}
            });

            // 首次释放脚本（flag 提前读取，避免 closure 生命周期问题）
            let need_release_scripts = !initial_settings.scripts_released;
            if need_release_scripts {
                let state_ref = app.state::<AppState>();
                script_injector::release_embedded_scripts(app.handle(), &state_ref).ok();
                state_ref.settings.lock().unwrap().scripts_released = true;
                settings::save(&state_ref.settings.lock().unwrap()).ok();
                info!("embedded scripts released on first launch");
            }

            // 托盘
            let _app_handle = app.handle().clone();
            let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &settings_item, &quit])?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "show" => {
                        let state = app.state::<AppState>();
                        let s = state.settings.lock().unwrap();
                        if let Some(first) = s.entries.first().cloned() {
                            std::mem::drop(s);
                            toggle_site_window(app, &first);
                        }
                    }
                    "settings" => {
                        if let Some(win) = app.get_webview_window("settings") {
                            bring_to_front(&win);
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let state = app.state::<AppState>();
                        let s = state.settings.lock().unwrap();
                        if let Some(first) = s.entries.first().cloned() {
                            std::mem::drop(s);
                            toggle_site_window(&app, &first);
                        }
                    }
                })
                .build(app)?;

            // 单实例
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                    info!("second instance detected, bringing windows to front");
                    if let Some(win) = app.get_webview_window("settings") {
                        let _ = win.unminimize();
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }))?;

            // 非最小化：显示第一个站点窗口
            let start_minimized = {
                let s = app.state::<AppState>();
                let val = s.settings.lock().unwrap().start_minimized;
                val
            };
            if !start_minimized {
                let s = app.state::<AppState>().settings.lock().unwrap().clone();
                if let Some(first) = s.entries.first().cloned() {
                    std::mem::drop(s);
                    show_site_window(app.handle(), &first);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            open_script_dir,
            toggle_window,
            show_settings_window,
            inject_scripts,
            release_embedded_scripts,
            check_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
