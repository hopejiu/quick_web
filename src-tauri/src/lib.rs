mod logger;
mod settings;

use std::collections::HashMap;
use std::fs;
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use log::{info, warn, error};
use tauri::{
    AppHandle, Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    WindowEvent,
};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

// ── 全局状态 ──

struct AppState {
    /// 当前设置
    settings: Mutex<settings::Settings>,
    /// 热键字符串 → entry_id 映射
    hotkey_map: Mutex<HashMap<String, String>>,
}

// ── 热键字符串解析（与原 deepseek-pc 格式兼容） ──

fn parse_hotkey(s: &str) -> Result<(Modifiers, Code), String> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.len() < 2 {
        return Err("热键格式无效，需要修饰键+按键".into());
    }
    let mut mods = Modifiers::empty();
    for p in &parts[..parts.len() - 1] {
        match p.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "win" | "windows" | "cmd" | "super" => mods |= Modifiers::SUPER,
            _ => return Err(format!("未知修饰键: {}", p)),
        }
    }
    let key_str = parts.last().unwrap().to_uppercase();
    let code = match key_str.as_str() {
        "A" => Code::KeyA,
        "B" => Code::KeyB,
        "C" => Code::KeyC,
        "D" => Code::KeyD,
        "E" => Code::KeyE,
        "F" => Code::KeyF,
        "G" => Code::KeyG,
        "H" => Code::KeyH,
        "I" => Code::KeyI,
        "J" => Code::KeyJ,
        "K" => Code::KeyK,
        "L" => Code::KeyL,
        "M" => Code::KeyM,
        "N" => Code::KeyN,
        "O" => Code::KeyO,
        "P" => Code::KeyP,
        "Q" => Code::KeyQ,
        "R" => Code::KeyR,
        "S" => Code::KeyS,
        "T" => Code::KeyT,
        "U" => Code::KeyU,
        "V" => Code::KeyV,
        "W" => Code::KeyW,
        "X" => Code::KeyX,
        "Y" => Code::KeyY,
        "Z" => Code::KeyZ,
        "0" => Code::Digit0,
        "1" => Code::Digit1,
        "2" => Code::Digit2,
        "3" => Code::Digit3,
        "4" => Code::Digit4,
        "5" => Code::Digit5,
        "6" => Code::Digit6,
        "7" => Code::Digit7,
        "8" => Code::Digit8,
        "9" => Code::Digit9,
        "F1" => Code::F1,
        "F2" => Code::F2,
        "F3" => Code::F3,
        "F4" => Code::F4,
        "F5" => Code::F5,
        "F6" => Code::F6,
        "F7" => Code::F7,
        "F8" => Code::F8,
        "F9" => Code::F9,
        "F10" => Code::F10,
        "F11" => Code::F11,
        "F12" => Code::F12,
        _ => return Err(format!("未知按键: {}", key_str)),
    };
    Ok((mods, code))
}

fn shortcut_to_string(mods: Modifiers, code: &Code) -> String {
    let mut parts = Vec::new();
    if mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if mods.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if mods.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if mods.contains(Modifiers::SUPER) {
        parts.push("Win");
    }
    parts.push(match code {
        Code::KeyA => "A", Code::KeyB => "B", Code::KeyC => "C",
        Code::KeyD => "D", Code::KeyE => "E", Code::KeyF => "F",
        Code::KeyG => "G", Code::KeyH => "H", Code::KeyI => "I",
        Code::KeyJ => "J", Code::KeyK => "K", Code::KeyL => "L",
        Code::KeyM => "M", Code::KeyN => "N", Code::KeyO => "O",
        Code::KeyP => "P", Code::KeyQ => "Q", Code::KeyR => "R",
        Code::KeyS => "S", Code::KeyT => "T", Code::KeyU => "U",
        Code::KeyV => "V", Code::KeyW => "W", Code::KeyX => "X",
        Code::KeyY => "Y", Code::KeyZ => "Z",
        Code::Digit0 => "0", Code::Digit1 => "1", Code::Digit2 => "2",
        Code::Digit3 => "3", Code::Digit4 => "4", Code::Digit5 => "5",
        Code::Digit6 => "6", Code::Digit7 => "7", Code::Digit8 => "8",
        Code::Digit9 => "9",
        Code::F1 => "F1", Code::F2 => "F2", Code::F3 => "F3",
        Code::F4 => "F4", Code::F5 => "F5", Code::F6 => "F6",
        Code::F7 => "F7", Code::F8 => "F8", Code::F9 => "F9",
        Code::F10 => "F10", Code::F11 => "F11", Code::F12 => "F12",
        _ => "?",
    });
    parts.join("+")
}

// ── 窗口工具 ──

fn bring_to_front(window: &tauri::WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

/// 启动窗口销毁定时器（1 分钟后销毁隐藏的窗口）
fn start_destroy_timer(app: &AppHandle, label: String) {
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(60));
        if let Some(win) = app.get_webview_window(&label) {
            if !win.is_visible().unwrap_or(false) {
                info!("destroy timer: closing window {}", label);
                let _ = win.close();
            }
        }
    });
}

/// 创建或显示站点窗口
fn show_site_window(app: &AppHandle, entry: &settings::SiteEntry) -> tauri::WebviewWindow {
    let label = format!("site-{}", entry.id);
    // 已有窗口 → 置前
    if let Some(win) = app.get_webview_window(&label) {
        bring_to_front(&win);
        return win;
    }
    // 新建
    info!("creating site window");
    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::External(entry.url.parse().unwrap()))
        .title(&entry.name)
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .build()
        .expect("failed to create site window");

    let win_clone = win.clone();
    let app_clone = app.clone();
    win.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = win_clone.hide();
            start_destroy_timer(&app_clone, label.clone());
        }
    });

    let _ = win.show();
    let _ = win.set_focus();

    // 调度脚本注入（原 Go 版 goroutine + 3 次重试）
    schedule_injection(app, entry);

    win
}

/// 延迟注入脚本（等待页面加载，最多 3 次重试）
fn schedule_injection(app: &AppHandle, entry: &settings::SiteEntry) {
    let label = format!("site-{}", entry.id);
    let script_dir = settings::script_dir_path(&entry.script_dir);
    let app = app.clone();

    std::thread::spawn(move || {
        for i in 0..3 {
            std::thread::sleep(Duration::from_secs(1));
            let Some(win) = app.get_webview_window(&label) else {
                warn!("injection: window {} not found (attempt {})", label, i + 1);
                continue;
            };
            win.eval(FOCUS_TEXTAREA_JS).ok();
            win.eval(BATCH_DELETE_JS).ok();
            info!("injected built-in scripts (attempt {})", i + 1);
            if let Ok(entries) = fs::read_dir(&script_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "js") {
                        if let Ok(content) = fs::read_to_string(&path) {
                            win.eval(&content).ok();
                            info!("injected custom script: {}", path.display());
                        }
                    }
                }
            }
            info!("script injection done for {}", label);
            return;
        }
        error!("injection: all 3 attempts failed for {}", label);
    });
}

/// 隐藏站点窗口
fn hide_site_window(app: &AppHandle, entry_id: &str) {
    let label = format!("site-{}", entry_id);
    if let Some(win) = app.get_webview_window(&label) {
        let _ = win.hide();
        start_destroy_timer(app, label);
    }
}

/// 切换站点窗口显示
fn toggle_site_window(app: &AppHandle, entry: &settings::SiteEntry) {
    let label = format!("site-{}", entry.id);
    if let Some(win) = app.get_webview_window(&label) {
        if win.is_visible().unwrap_or(false) {
            hide_site_window(app, &entry.id);
        } else {
            bring_to_front(&win);
        }
    } else {
        show_site_window(app, entry);
    }
}

// ── 热键同步 ──

/// 重新注册所有热键
fn sync_hotkeys(app: &AppHandle, state: &AppState, new: &settings::Settings) {
    let mut map = state.hotkey_map.lock().unwrap();
    let shortcut_manager = app.global_shortcut();

    // 注销所有旧热键
    for hk in map.keys() {
        if let Ok((mods, code)) = parse_hotkey(hk) {
            let _ = shortcut_manager.unregister(Shortcut::new(Some(mods), code.clone()));
        }
    }
    map.clear();

    // 注册所有新热键
    for entry in &new.entries {
        if entry.hotkey.is_empty() {
            continue;
        }
        match parse_hotkey(&entry.hotkey) {
            Ok((mods, code)) => {
                let shortcut = Shortcut::new(Some(mods), code);
                if let Err(e) = shortcut_manager.register(shortcut) {
                    warn!("hotkey register failed: {} ({})", entry.hotkey, e);
                } else {
                    map.insert(entry.hotkey.clone(), entry.id.clone());
                    info!("hotkey registered: {} → {}", entry.hotkey, entry.name);
                }
            }
            Err(e) => warn!("invalid hotkey {}: {}", entry.hotkey, e),
        }
    }
}

// ── Tauri Commands ──

#[tauri::command]
fn get_settings(state: tauri::State<AppState>) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: tauri::State<AppState>,
    data: settings::Settings,
) -> Result<(), String> {
    let old = {
        let mut s = state.settings.lock().unwrap();
        let old = s.clone();
        *s = data.clone();
        old
    };
    settings::save(&data)?;

    // Autostart 同步
    let autostart = app.autolaunch();
    if data.auto_start {
        let _ = autostart.enable();
    } else {
        let _ = autostart.disable();
    }

    // 热键同步（diff 变更的条目）
    sync_hotkeys(&app, &state, &data);

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
    Ok(())
}

#[tauri::command]
fn open_script_dir(dir_name: String) -> Result<(), String> {
    let dir = settings::script_dir_path(&dir_name);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Command::new("explorer").arg(&dir).spawn().map_err(|e| format!("打开目录失败: {}", e))?;
    Ok(())
}

#[tauri::command]
fn toggle_window(app: AppHandle, state: tauri::State<AppState>, entry_id: String) -> Result<(), String> {
    let settings = state.settings.lock().unwrap();
    let entry = settings
        .entries
        .iter()
        .find(|e| e.id == entry_id)
        .cloned()
        .ok_or_else(|| format!("entry not found: {}", entry_id))?;
    drop(settings);
    toggle_site_window(&app, &entry);
    Ok(())
}

#[tauri::command]
fn show_settings_window(app: AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        bring_to_front(&win);
    }
}

// ── 注入脚本 ──

/// 编译时嵌入的内置脚本
const FOCUS_TEXTAREA_JS: &str = r#"console.log('[inject] focus-textarea.js loaded');var el=document.querySelector('textarea[name="search"]');if(el)el.focus();"#;
const BATCH_DELETE_JS: &str = include_str!("../../scripts/batch-delete.js");

#[tauri::command]
fn inject_scripts(app: AppHandle, entry_id: String) -> Result<(), String> {
    let label = format!("site-{}", entry_id);
    let win = app
        .get_webview_window(&label)
        .ok_or("window not found")?;

    // 先注入 focus-textarea 脚本
    let _ = win.eval(FOCUS_TEXTAREA_JS);

    // 从磁盘加载自定义脚本
    let script_dir = {
        let settings = app.state::<AppState>();
        let s = settings.settings.lock().unwrap();
        let entry = s
            .entries
            .iter()
            .find(|e| e.id == entry_id)
            .cloned();
        entry.map(|e| settings::script_dir_path(&e.script_dir))
    };
    if let Some(dir) = script_dir {
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "js") {
                    if let Ok(content) = fs::read_to_string(&path) {
                        let _ = win.eval(&content);
                        info!("injected script: {}", path.display());
                    }
                }
            }
        }
    }
    Ok(())
}

#[tauri::command]
fn release_embedded_scripts(_app: AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    let scripts_dir = settings::config_dir().join("scripts").join("chat.deepseek.com");
    if scripts_dir.exists() {
        return Ok(());
    }
    fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;
    // 写入 batch-delete.js
    fs::write(scripts_dir.join("batch-delete.js"), BATCH_DELETE_JS).map_err(|e| e.to_string())?;
    fs::write(scripts_dir.join("focus-textarea.js"), FOCUS_TEXTAREA_JS).map_err(|e| e.to_string())?;

    let mut s = state.settings.lock().unwrap();
    s.scripts_released = true;
    settings::save(&s)?;
    info!("embedded scripts released");
    Ok(())
}

// ── 应用入口 ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // ---- 日志初始化 ----
    let cfg_dir = settings::config_dir();
    let log_dir = cfg_dir.join("logs");
    let dev_mode = cfg!(debug_assertions);
    logger::init(log_dir, dev_mode).ok();
    info!("app started, config dir: {:?}", cfg_dir);

    // ---- 加载设置 ----
    let initial_settings = settings::load();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let hk_str = shortcut_to_string(
                        shortcut.mods,
                        &shortcut.key,
                    );
                    if let Some(state) = app.try_state::<AppState>() {
                        let map = state.hotkey_map.lock().unwrap();
                        if let Some(entry_id) = map.get(&hk_str) {
                            let entry_id = entry_id.clone();
                            drop(map);
                            let settings = state.settings.lock().unwrap();
                            let entry = settings.entries.iter().find(|e| e.id == entry_id).cloned();
                            drop(settings);
                            if let Some(e) = entry {
                                toggle_site_window(app, &e);
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {
            // ---- Autostart ----
            app.handle().plugin(
                tauri_plugin_autostart::init(
                    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
                    None,
                ),
            )?;

            // ---- 状态 ----
            let hotkey_map = {
                let mut map = HashMap::new();
                for e in &initial_settings.entries {
                    if !e.hotkey.is_empty() {
                        map.insert(e.hotkey.clone(), e.id.clone());
                    }
                }
                map
            };
            let state = AppState {
                settings: Mutex::new(initial_settings),
                hotkey_map: Mutex::new(hotkey_map),
            };
            app.manage(state);

            // ---- 设置窗口（始终隐藏，通过托盘菜单唤起） ----
            let settings_win = WebviewWindowBuilder::new(
                app,
                "settings",
                WebviewUrl::App("index.html".into()),
            )
            .title("设置")
            .inner_size(700.0, 620.0)
            .min_inner_size(600.0, 400.0)
            .visible(false)
            .build()?;
            // 窗口关闭 → 隐藏而非退出
            let sw = settings_win.clone();
            settings_win.on_window_event(move |event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = sw.hide();
                }
            });

            // ---- 注册热键 ----
            let state_ref = app.state::<AppState>();
            sync_hotkeys(app.handle(), state_ref.inner(), &state_ref.settings.lock().unwrap().clone());

            // ---- 首次释放脚本 ----
            if !state_ref.settings.lock().unwrap().scripts_released {
                let scripts_dir = settings::config_dir().join("scripts").join("chat.deepseek.com");
                fs::create_dir_all(&scripts_dir).ok();
                fs::write(scripts_dir.join("batch-delete.js"), BATCH_DELETE_JS).ok();
                fs::write(scripts_dir.join("focus-textarea.js"), FOCUS_TEXTAREA_JS).ok();
                state_ref.settings.lock().unwrap().scripts_released = true;
                settings::save(&state_ref.settings.lock().unwrap()).ok();
                info!("embedded scripts released on first launch");
            }

            // ---- 启动最小化判断 ----
            let start_minimized = state_ref.settings.lock().unwrap().start_minimized;

            // ---- 系统托盘 ----
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
                        // 显示默认站点窗口
                        let state = app.state::<AppState>();
                        let s = state.settings.lock().unwrap();
                        if let Some(first) = s.entries.first().cloned() {
                            drop(s);
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
                            drop(s);
                            toggle_site_window(&app, &first);
                        }
                    }
                })
                .build(app)?;

            // ---- 单实例 ----
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                info!("second instance detected, bringing windows to front");
                if let Some(win) = app.get_webview_window("settings") {
                    let _ = win.unminimize();
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }))?;

            // ---- 非最小化：显示第一个站点窗口 ----
            if !start_minimized {
                let s = app.state::<AppState>().settings.lock().unwrap().clone();
                if let Some(first) = s.entries.first().cloned() {
                    drop(s);
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
