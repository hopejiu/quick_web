use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use log::{info, warn, error, debug};
use tauri::{AppHandle, Manager};

use crate::settings;

/// 编译时嵌入的内置脚本
const FOCUS_TEXTAREA_JS: &str = r#"console.log('[inject] focus-textarea.js loaded');var el=document.querySelector('textarea[name="search"]');if(el)el.focus();"#;
const BATCH_DELETE_JS: &str = include_str!("../../scripts/batch-delete.js");

/// 延迟注入脚本（窗口创建后调用，等待页面加载）
pub fn schedule_injection(app: &AppHandle, entry: &settings::SiteEntry) {
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
            eval_scripts(&win, &script_dir);
            info!("script injection done for {}", label);
            return;
        }
        error!("injection: all 3 attempts failed for {}", label);
    });
}

/// 一次性注入（命令调用，手动重注入）
pub fn inject_once(app: &AppHandle, entry_id: &str) -> Result<(), String> {
    let label = format!("site-{}", entry_id);
    let win = app.get_webview_window(&label).ok_or("window not found")?;

    let script_dir = {
        use crate::AppState;
        let settings = app.state::<AppState>();
        let s = settings.settings.lock().unwrap();
        s.entries.iter().find(|e| e.id == entry_id)
            .map(|e| settings::script_dir_path(&e.script_dir))
    };

    if let Some(dir) = script_dir {
        eval_scripts(&win, &dir);
    }
    Ok(())
}

/// 内部：eval 内置脚本 + 脚本目录所有 .js 文件
fn eval_scripts(win: &tauri::WebviewWindow, script_dir: &PathBuf) {
    if let Err(e) = win.eval(FOCUS_TEXTAREA_JS) {
        debug!("eval focus-textarea failed: {}", e);
    }
    if let Err(e) = win.eval(BATCH_DELETE_JS) {
        debug!("eval batch-delete failed: {}", e);
    }
    if let Ok(entries) = fs::read_dir(script_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "js") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Err(e) = win.eval(&content) {
                        debug!("eval custom script {} failed: {}", path.display(), e);
                    } else {
                        info!("injected custom script: {}", path.display());
                    }
                }
            }
        }
    }
}

/// 首次启动：释放嵌入脚本到磁盘
pub fn release_embedded_scripts(_app: &AppHandle, state: &crate::AppState) -> Result<(), String> {
    let scripts_dir = settings::config_dir().join("scripts").join("chat.deepseek.com");
    if scripts_dir.exists() {
        return Ok(());
    }
    fs::create_dir_all(&scripts_dir).map_err(|e| e.to_string())?;
    fs::write(scripts_dir.join("batch-delete.js"), BATCH_DELETE_JS).map_err(|e| e.to_string())?;
    fs::write(scripts_dir.join("focus-textarea.js"), FOCUS_TEXTAREA_JS).map_err(|e| e.to_string())?;

    let mut s = state.settings.lock().unwrap();
    s.scripts_released = true;
    settings::save(&s)?;
    info!("embedded scripts released");
    Ok(())
}
