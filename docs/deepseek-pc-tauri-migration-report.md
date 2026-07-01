# deepseek-pc → Tauri 迁移技术报告

> 生成日期: 2026-07-01
> 源项目: `D:\code\deepseek-pc` (Go + Wails3)
> 目标: `D:\code\quick-web-v2` (Rust + Tauri 2 + Vite + TypeScript)
> Tauri 文档: `D:\wiki\tauri\zh-cn`

---

## 目录

1. [项目概述](#1-项目概述)
2. [原有功能清单](#2-原有功能清单)
3. [架构对比](#3-架构对比)
4. [Tauri API 映射分析](#4-tauri-api-映射分析)
5. [前端代码复用方案](#5-前端代码复用方案)
6. [详细模块实现方案](#6-详细模块实现方案)
7. [Cargo.toml 依赖清单](#7-cargotoml-依赖清单)
8. [实施步骤](#8-实施步骤)
9. [风险与注意事项](#9-风险与注意事项)

---

## 1. 项目概述

### 1.1 源项目 (deepseek-pc)

- **技术栈**: Go 1.25 + Wails3 (v3.0.0-alpha.97) + 原生 JavaScript + Vite
- **定位**: 多站点桌面浏览器壳，为每个站点提供独立窗口、全局快捷键、脚本注入
- **默认站点**: `https://chat.deepseek.com`
- **前端**: 纯 JavaScript（无框架），单页设置管理界面
- **后端模块**: 6 个 internal 包 (webview/hotkey/coordinator/settings/scripts/log)
- **构建**: Taskfile.yml + Wails3 CLI

### 1.2 目标项目 (quick-web-v2)

- **技术栈**: Rust (edition 2021) + Tauri 2 + TypeScript + Vite
- **当前状态**: Tauri 2 starter 模板（单个窗口、greet 命令）
- **前端**: TypeScript + Vite + Tauri API (`@tauri-apps/api` v2)
- **Rust 后端**: `src-tauri/src/lib.rs` + `main.rs`

---

## 2. 原有功能清单

### 2.1 核心功能

| # | 功能 | Go 实现 | 迁移必要性 |
|---|------|---------|-----------|
| 1 | **多站点 WebView 窗口** | `internal/webview/manager.go` — Manager 管理主窗口 + 多站点窗口的生命周期（创建/隐藏/销毁/定时器） | 核心功能 |
| 2 | **全局热键呼出/隐藏窗口** | `internal/hotkey/hotkey.go` — 包装 `golang.design/x/hotkey`，每个站点绑定独立热键，200ms 防抖 | 核心功能 |
| 3 | **设置管理** | `internal/settings/service.go` — JSON 持久化 (`os.UserConfigDir()/deepseek-pc/settings.json`)，CRUD 站点条目 | 核心功能 |
| 4 | **JS 脚本注入** | `internal/scripts/loader.go` — 首次启动 `go:embed` → appdata 目录，后续从文件系统加载 `.js` 文件注入 `ExecJS()` | 核心功能 |
| 5 | **协调器** | `internal/coordinator/coordinator.go` — 热键注册/注销、窗口同步、脚本注入调度 | 核心功能 |
| 6 | **系统托盘** | `main.go:137-157` — 托盘图标 + 菜单（显示窗口/设置/退出） | 核心功能 |
| 7 | **单实例** | `SingleInstanceOptions{UniqueID:"com.deepseek.desktop"}` | 核心功能 |
| 8 | **开机自启** | `app.Autostart.Enable()` | 核心功能 |
| 9 | **启动最小化** | `StartMinimized` 配置 + 条件 `wm.Show()` | 核心功能 |
| 10 | **文件日志** | `internal/log/log.go` — 按小时轮转，72h 保留，slog 输出 | 非必需可推迟 |

### 2.2 设置页面功能（前端）

| # | 功能 | 描述 |
|---|------|------|
| 1 | 站点列表 | 显示 name/url/hotkey/script_dir |
| 2 | 添加站点 | 弹窗表单：名称、网址（协议选择+域名）、快捷键选择器（修饰键按钮+按键下拉）、脚本目录（自动推导）、快捷冲突检测 |
| 3 | 编辑站点 | 同添加，预填充 |
| 4 | 删除站点 | 确认弹窗，默认站点不可删 (`default`) |
| 5 | 打开脚本目录 | 调用后端命令打开资源管理器 |
| 6 | 开机自启开关 | Toggle |
| 7 | 启动最小化开关 | Toggle |

### 2.3 注入脚本

| # | 脚本 | 功能 |
|---|------|------|
| 1 | `focus-textarea.js` | 自动聚焦搜索文本框 |
| 2 | `batch-delete.js` | DeepSeek Chat 批量删除对话（CSS 注入 + checkbox UI + fetch API 调用） |

---

## 3. 架构对比

### 3.1 进程模型

```
Wails3:                            Tauri:
┌──────────────┐                  ┌──────────────┐
│  Go Runtime  │                  │ Rust Core    │
│  (main)      │                  │  (lib.rs)    │
│              │                  │              │
│  ┌────────┐  │                  │  ┌────────┐  │
│  │WebView1│  │                  │  │WebView1│  │
│  │(设置页)│  │                  │  │(设置页)│  │
│  └────────┘  │                  │  └────────┘  │
│  ┌────────┐  │                  │  ┌────────┐  │
│  │WebView2│  │                  │  │WebView2│  │
│  │(站点1) │  │                  │  │(站点1) │  │
│  └────────┘  │                  │  └────────┘  │
│  ┌────────┐  │                  │  ┌────────┐  │
│  │WebView3│  │                  │  │WebView3│  │
│  │(站点2) │  │                  │  │(站点2) │  │
│  └────────┘  │                  │  └────────┘  │
└──────────────┘                  └──────────────┘
```

- Wails3: Go 主进程直接嵌入 WebView2，窗口创建后 Go 代码保持运行
- Tauri: Rust 核心进程 + WebView 子进程（由系统 WebView 库管理），IPC 通过 JSON-RPC

### 3.2 关键差异

| 维度 | Wails3 (Go) | Tauri (Rust) |
|------|-------------|--------------|
| 前端框架 | 任意（绑定由 Go 生成） | 任意（调用 `invoke`） |
| Rust/Go ↔ JS 通信 | Service 绑定生成前端代码 | `#[tauri::command]` + `invoke()` |
| 多 WebView | 原生支持 `app.Window.NewWithOptions()` | `WebviewWindowBuilder::new()` |
| 全局热键 | `golang.design/x/hotkey` | `tauri-plugin-global-shortcut` |
| 系统托盘 | 内置 `application.SystemTray` | 内置 `tray-icon` feature |
| 单实例 | 内置 `SingleInstanceOptions` | 内置 `app.set_single_instance()` |
| 开机自启 | 内置 `app.Autostart` | `tauri-plugin-autostart` |
| JS 注入 | `win.ExecJS(script)` | `webview.eval(script)` |
| 窗口隐藏/销毁 | 手动 `Hide()` + 定时器 `Close()` | `window.hide()` / 窗口关闭事件 |
| 日志 | 手动文件日志 | 可用 `log` crate + tracing |
| 构建 | Taskfile.yml + `wails3 build` | `cargo tauri build` |

---

## 4. Tauri API 映射分析

### 4.1 多窗口 (替代 `internal/webview/manager.go`)

Tauri 原生支持多 WebviewWindow：

```rust
// Rust 端创建站点窗口
use tauri::{WebviewUrl, WebviewWindowBuilder};

let site_window = WebviewWindowBuilder::new(app, "site-{entry_id}", WebviewUrl::External(url.parse().unwrap()))
    .title(&entry.name)
    .inner_size(1200.0, 800.0)
    .min_inner_size(800.0, 600.0)
    .build()?;
```

**窗口生命周期管理**:
- 隐藏: `window.hide()` (前端) 或 `window_handle.hide()` (Rust)
- 显示: `window.show()` + `window.set_focus()`
- 销毁定时器: 在 `on_window_event(WindowEvent::CloseRequested)` 中 `event.prevent_close()` 替代隐藏，启动 Rust 定时器 `tokio::spawn(async { sleep(DURATION).await; window.close() })`
- 需要 `window:allow-hide` `window:allow-show` `window:allow-close` 等权限

**关键差异**:
- Wails3 在窗口关闭时 `e.Cancel()` 然后 `Hide()`，Tauri 在 `CloseRequested` 事件中 `event.prevent_close()` 并调用 `hide()`
- Wails3 `destroyTimeout = 1 minute` 后用 `Close()` 真正销毁，Tauri 同理

### 4.2 全局热键 (替代 `internal/hotkey/hotkey.go`)

使用 `tauri-plugin-global-shortcut`:

```toml
# Cargo.toml
[dependencies]
tauri-plugin-global-shortcut = "2"
```

```rust
// 注册 - Rust 端
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyE);
app.global_shortcut().register(shortcut)?;
```

```javascript
// 注册 - 前端
import { register } from '@tauri-apps/plugin-global-shortcut';
await register('Alt+E', () => { /* toggle window */ });
```

**推荐方案**: 在 Rust 端统一管理所有热键（启动时注册所有条目的热键，设置变更时热重载），与 settings 模块联动

**前端快捷键选择器** 代码可以直接从 `frontend/src/main.js` 复制，仅需将 `SettingsService.SaveSettings(settings)` 替换为 `invoke('save_settings', { data: settings })`

### 4.3 系统托盘 (替代 `main.go:137-157`)

Tauri 内置支持，需启用 feature:

```toml
# Cargo.toml
tauri = { version = "2", features = ["tray-icon"] }
```

```rust
// Rust 端
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

let show = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
let settings = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
let menu = Menu::with_items(app, &[&show, &settings, &quit])?;

TrayIconBuilder::new()
    .icon(app.default_window_icon().unwrap().clone())
    .menu(&menu)
    .on_menu_event(|app, event| match event.id.as_ref() {
        "show" => { /* show main window */ }
        "settings" => { /* show settings window */ }
        "quit" => app.exit(0),
        _ => {}
    })
    .on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event {
            // 左键单击显示窗口
            if let Some(window) = tray.app_handle().get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    })
    .build(app)?;
```

**托盘图标**: 源项目用 `build/appicon.png`，Tauri 默认使用 `icons/` 目录的图标。
**注意**: 图标需要从 `D:\code\deepseek-pc\logo.png` 或 `build/appicon.png` 复制过来。

### 4.4 单实例 (替代 `SingleInstanceOptions`)

```rust
tauri::Builder::default()
    .setup(|app| {
        #[cfg(desktop)]
        let _ = app.set_single_instance_handler(|app, argv, cwd| {
            // 第二实例启动时，显示主窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        });
        Ok(())
    })
```

### 4.5 开机自启 (替代 `app.Autostart.Enable()`)

```toml
# Cargo.toml
tauri-plugin-autostart = "2"
```

```rust
use tauri_plugin_autostart::ManagerExt;

// 初始化
app.handle().plugin(tauri_plugin_autostart::init(
    tauri_plugin_autostart::MacosLauncher::LaunchAgent,
    None
))?;

// 启用/禁用
let autostart = app.handle().autolaunch();
autostart.enable()?;
autostart.disable()?;
println!("autostart enabled: {}", autostart.is_enabled()?);
```

```javascript
// 前端
import { enable, disable, isEnabled } from '@tauri-apps/plugin-autostart';
```

### 4.6 脚本注入 (替代 `internal/scripts/loader.go`)

```rust
// Rust 端
use tauri::Manager;
use std::fs;

#[tauri::command]
fn inject_scripts(app: tauri::AppHandle, entry_id: String, script_dir: String) -> Result<(), String> {
    let window = app.get_webview_window(&format!("site-{}", entry_id))
        .ok_or("window not found")?;

    let mut entries = fs::read_dir(&script_dir).map_err(|e| e.to_string())?;
    while let Some(Ok(entry)) = entries.next() {
        if entry.path().extension().map_or(false, |ext| ext == "js") {
            let script = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
            window.eval(&script).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
```

**注意**: 首次释放内置脚本的机制需要保留 — 首次启动时将 `scripts/` 下的 `.js` 复制到 appdata 目录。Tauri 没有 `go:embed`，但可以用 Rust 的 `include_str!` 编译时嵌入。

### 4.7 设置持久化 (替代 `internal/settings/service.go`)

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SiteEntry {
    pub id: String,
    pub url: String,
    pub name: String,
    pub hotkey: String,
    pub script_dir: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub entries: Vec<SiteEntry>,
    pub auto_start: bool,
    pub start_minimized: bool,
    pub scripts_released: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            entries: vec![SiteEntry {
                id: "default".into(),
                url: "https://chat.deepseek.com".into(),
                name: "DeepSeek Chat".into(),
                hotkey: "Alt+E".into(),
                script_dir: "chat.deepseek.com".into(),
            }],
            auto_start: false,
            start_minimized: false,
            scripts_released: false,
        }
    }
}

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("deepseek-pc")  // 复用原目录名以兼容旧配置
}

pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

pub fn script_dir(dir_name: &str) -> PathBuf {
    config_dir().join("scripts").join(dir_name)
}

#[tauri::command]
fn get_settings() -> Settings {
    let path = settings_path();
    if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Settings::default()
    }
}

#[tauri::command]
fn save_settings(data: Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
```

**兼容性**: 复用 `deepseek-pc` 配置目录名，旧版设置可无缝迁移。

### 4.8 文件日志 (替代 `internal/log/log.go`)

源项目的日志系统比较复杂（每小时轮转、72h 清理）。Tauri 生态推荐使用标准 Rust 日志库：

```toml
# Cargo.toml
log = "0.4"
env_logger = "0.11"
```

简化：启动时设置 `env_logger`，日志输出到 stderr。如需文件日志，可用 `log4rs` 或 `tracing-appender`（非核心功能，可推迟）。

### 4.9 窗口动画 bringToFront (替代 `manager.go:bringToFront`)

源项目用 `SetAlwaysOnTop(true)` → `SetAlwaysOnTop(false)` 的 hack 强制提前。Tauri 需要用平台特定方式：

```rust
use tauri::Manager;

fn bring_to_front(window: &tauri::WebviewWindow) {
    let _ = window.show();
    let _ = window.set_focus();
    // Windows 平台可能需要额外操作
    #[cfg(target_os = "windows")]
    {
        // 可以使用 tao 的 window 原始句柄操作
        // window.set_always_on_top(true)?;
        // window.set_always_on_top(false)?;
    }
}
```

---

## 5. 前端代码复用方案

### 5.1 可复用代码分析

源项目前端 `D:\code\deepseek-pc\frontend/` 结构：
```
frontend/
  src/main.js          # 主逻辑（站点 CRUD、快捷键选择器、开关）
  public/style.css     # 全部样式
  index.html           # HTML 模板（无框架）
  vite.config.js       # Vite 配置（含 Wails 插件）
```

**核心判断**: 源前端代码**几乎可以全部复用**，因为：
1. 无框架依赖（纯 JS + HTML + CSS）
2. 设置页面逻辑与框架无关
3. CSS 质量高、完整覆盖所有 UI 组件

### 5.2 复用时需修改

#### index.html → 复制到 `index.html`
- Wails 绑定导入 `import { Service as SettingsService } from "../bindings/..."` → Tauri `import { invoke } from '@tauri-apps/api/core'`
- 删除 `style.css` 引用 → 改为 Tauri 默认样式引用或直接在 HTML 中引入

#### src/main.js → 复制到 `src/main.ts`（改为 .ts 但有改动点）
函数调用替换表：

| 原调用 | 替换为 |
|--------|--------|
| `SettingsService.GetSettings()` | `invoke('get_settings')` |
| `SettingsService.SaveSettings(settings)` | `invoke('save_settings', { data: settings })` |
| `SettingsService.OpenScriptDir(dir)` | `invoke('open_script_dir', { dirName: dir })` |

修改示例：
```javascript
// 原代码 (Wails)
import { Service as SettingsService } from "../bindings/changeme/internal/settings";
settings = await SettingsService.GetSettings();

// 新代码 (Tauri)
import { invoke } from '@tauri-apps/api/core';
settings = await invoke('get_settings');
```

#### style.css → 复制到 `src/styles.css`
- 样式全部可用，无需修改
- 只需更新 `index.html` 中的引用路径
- 旧 project 用 `/style.css` (public 目录)，Tauri 用 `/src/styles.css` (src 目录) 或维持 public 引用

### 5.3 注入脚本复用

`scripts/batch-delete.js` 和 `scripts/focus-textarea.js` 是纯前端脚本，通过 `WebviewWindow.eval()` 注入到站点页面。

- 这两份脚本**完全无需修改**
- 存储路径从 `go:embed` 改为 Rust 编译时嵌入（用字符串常量）或首次启动写入 appdata

---

## 6. 详细模块实现方案

### 6.1 Rust 后端模块结构

```
src-tauri/src/
  lib.rs              # 主入口、所有命令注册
  main.rs             # #![windows_subsystem = "windows"] + run()
  settings.rs         # 设置数据模型、读写、默认值
  window_manager.rs   # 多窗口生命周期管理（创建/隐藏/销毁/定时器）
  script_injector.rs  # 脚本加载和注入
```

### 6.2 Rust → Tauri 命令清单

| 命令名 | 位置 | 功能 |
|--------|------|------|
| `get_settings` | `settings.rs` | 读取 settings.json |
| `save_settings` | `settings.rs` | 写入 settings.json + 触发回调 |
| `open_script_dir` | `settings.rs` | 用 OS 文件管理器打开脚本目录 |
| `release_scripts` | `script_injector.rs` | 首次启动释放内置脚本 |
| `inject_scripts` | `script_injector.rs` | 注入指定目录的 JS 到窗口 |
| `show_window` | `window_manager.rs` | 显示 / 创建站点窗口 |
| `hide_window` | `window_manager.rs` | 隐藏站点窗口（启动销毁定时器） |
| `toggle_window` | `window_manager.rs` | 切换站点窗口显示状态 |
| `show_settings` | `window_manager.rs` | 显示设置窗口 |

### 6.3 热键联动机制

源项目使用 `coordinator.go` 在设置保存后 diff 新旧 entries 并同步热键：

```
用户保存设置 → save_settings(data) → Rust 端 diff entries
  → 已删除/变更的条目：unregister hotkey + destroy window
  → 新增/变更的条目：register hotkey + create window (懒加载)
  → 自动启动：enable/disable autostart
```

在 Tauri 中实现：
```rust
// lib.rs 中维护全局状态
struct AppState {
    settings: Mutex<Settings>,
}

// save_settings 命令回调
fn on_settings_saved(app: &AppHandle, old: &Settings, new: &Settings) {
    // 1. diff hotkeys - 用 global-shortcut 插件
    // 2. diff autostart - 用 autostart 插件
    // 3. diff windows - 创建/销毁
}
```

### 6.4 窗口销毁定时器

```rust
use std::time::Duration;
use tauri::Manager;

// 启动销毁定时器
fn start_destroy_timer(app: &AppHandle, label: &str) {
    let app_handle = app.clone();
    let label = label.to_string();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(60));
        if let Some(window) = app_handle.get_webview_window(&label) {
            if !window.is_visible().unwrap_or(false) {
                let _ = window.close();
            }
        }
    });
}
```

**ponytail**: 仅基础 polling 定时器，未考虑 `tokio` 运行时。如果 app 用 tokio，可用 `tokio::spawn` + `tokio::time::sleep` 替代 std thread。

---

## 7. Cargo.toml 依赖清单

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-opener = "2"          # 已存在，维持
tauri-plugin-global-shortcut = "2" # 全局热键
tauri-plugin-autostart = "2"       # 开机自启
serde = { version = "1", features = ["derive"] }
serde_json = "1"
dirs = "6"                         # 跨平台配置目录
log = "0.4"                        # 日志
env_logger = "0.11"                # 日志输出
```

### 前端 npm 依赖

```json
{
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "@tauri-apps/plugin-global-shortcut": "^2",
    "@tauri-apps/plugin-autostart": "^2"
  }
}
```

### capabilities/default.json 权限

```json
{
  "identifier": "default",
  "windows": ["main", "settings"],
  "permissions": [
    "core:default",
    "opener:default",
    "global-shortcut:allow-register",
    "global-shortcut:allow-unregister",
    "global-shortcut:allow-is-registered",
    "autostart:allow-enable",
    "autostart:allow-disable",
    "autostart:allow-is-enabled",
    "core:window:default",
    "core:window:allow-create",
    "core:window:allow-close",
    "core:window:allow-hide",
    "core:window:allow-show",
    "core:window:allow-set-focus",
    "core:window:allow-minimize",
    "core:window:allow-toggle-maximize"
  ]
}
```

---

## 8. 实施步骤

### Phase 1: 基础框架 (0.5 天)

1. [ ] 在 `quick-web-v2` 添加 Tauri 依赖：`global-shortcut`, `autostart`, `dirs`, `log`, `env_logger`
2. [ ] 编写 `settings.rs` — 数据模型 + 读写 + 默认值
3. [ ] 编写 `lib.rs` — 注册 `get_settings` / `save_settings` / `open_script_dir` 三个命令
4. [ ] 前端：复制 `index.html` 和 `style.css` → 适配 Tauri (替换 `/style.css` 引用为 `/src/styles.css`)
5. [ ] 前端：将 `main.js` 的 `SettingsService.GetSettings/SaveSettings` 替换为 `invoke()`
6. [ ] 验证：`cargo tauri dev` → 设置页面加载、站点 CRUD 正常

### Phase 2: 多窗口 + 热键 (1 天)

7. [ ] 编写 `window_manager.rs` — 窗口创建/隐藏/销毁/定时器
8. [ ] 注册 `show_window` / `hide_window` / `toggle_window` 命令
9. [ ] 在 `lib.rs` 中集成 `global-shortcut` 插件
10. [ ] 实现热键注册/注销逻辑（启动时注册全部，save_settings 后 diff 同步）
11. [ ] 前端：将快捷键选择器的后端调用适配到 Tauri `invoke`
12. [ ] 验证：设置站点 → 快捷键呼出/隐藏站点窗口

### Phase 3: 脚本注入 (0.5 天)

13. [ ] 编写 `script_injector.rs` — 文件系统脚本加载 + `window.eval()`
14. [ ] 编译时嵌入默认脚本（`batch-delete.js` / `focus-textarea.js`）
15. [ ] 实现首次启动释放逻辑
16. [ ] 窗口创建时自动调度脚本注入

### Phase 4: 系统托盘 + 其他 (0.5 天)

17. [ ] 集成系统托盘 (Rust `TrayIconBuilder`)
18. [ ] 集成 `autostart` 插件
19. [ ] 集成单实例 `set_single_instance_handler`
20. [ ] 实现启动最小化逻辑
21. [ ] `bring_to_front` 窗口聚焦

### Phase 5: 适配与测试 (0.5 天)

22. [ ] 构建测试：`cargo tauri build`
23. [ ] Windows 托盘图标设置
24. [ ] F12 开发者工具快捷键
25. [ ] 最小窗口尺寸约束
26. [ ] 设置窗口关闭时隐藏而非退出

---

## 9. 风险与注意事项

### 9.1 已知风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| Tauri 多窗口的 `eval()` 在外部 URL 上可能受 CSP 限制 | 脚本注入失败 | 确认站点 CSP 策略。Tauri 2 `WebviewUrl::External` 页面默认有跨域限制，可能需要额外配置 |
| `global-shortcut` 快捷键格式可能与旧项目不同 | 热键不兼容 | `global-shortcut` 使用 `Code` 枚举（`Code::KeyE`），不是字符串格式。需在 Rust 端实现 `"Alt+E"` → `(Modifiers::ALT, Code::KeyE)` 解析 |
| 窗口销毁后重建可能丢失 WebView 状态 | 用户体验劣化 | 沿用旧项目的定期销毁机制，重建时重新注入脚本 |
| Tauri 2 `SingleInstance` 在不同平台表现不同 | 跨平台不一致 | Windows 使用 `set_single_instance_handler`，Linux/macOS 需额外测试 |
| `bring_to_front` hack (always-on-top toggle) 不可搬运 | 窗口可能落后其他窗口 | 尝试 `window.unminimize()` + `window.show()` + `window.set_focus()`，如仍无效再考虑平台特定方案 |
| 旧 settings.json 的 `snake_case` vs Tauri 默认 `camelCase` | 序列化不兼容 | 在 Rust 端使用 `#[serde(rename_all = "snake_case")]` 保持与旧 JSON 兼容 |

### 9.2 关键决策点

1. **配置目录**: 复用 `deepseek-pc` 还是改用新名称如 `quick-web-v2`？建议复用以兼容旧配置
2. **前端框架**: 保持纯 JS 复用，还是趁迁移升级到 React/Vue/Svelte？建议 YAGNI — 纯 JS 能用就不换框架
3. **脚本注入方式**: Rust 端 `eval()` 还是 Tauri 2 新引入的 `WebviewWindow::eval_on_webview()`？取决于 Tauri 2 API 版本
4. **日志**: 源项目 `internal/log` 是否必要？非必要可暂不实现，用 `env_logger` 输出 stderr 即可

### 9.3 ponytail 标记的简化

- **注入脚本冲突检测**: 旧项目用 `window.__batchDeleteInjected` guard。Tauri `eval` 每次注入独立执行上下文，但 inject 脚本的 `window.__xxxInjected` 仍然有效
- **防抖**: 旧项目 hotkey 有 200ms debounce。`global-shortcut` 插件内部已有防抖机制，Rust 端无需重复处理
- **`initDefaults` 迁移兼容**: 旧项目在 `Load()` 时自动迁移旧版单热键格式。Rust 端应保持相同逻辑，读取时检测 `entries` 是否为空并自动创建默认条目

---

## 附录 A：源项目关键文件索引

| 功能 | 源文件 (deepseek-pc) | 行数 | 迁移方案 |
|------|---------------------|------|----------|
| 主入口 | `main.go` | 180 | → `src-tauri/src/lib.rs` |
| 设置模型 | `internal/settings/model.go` | 21 | → `settings.rs` struct |
| 设置服务 | `internal/settings/service.go` | 185 | → `settings.rs` CRUD |
| WebView 管理 | `internal/webview/manager.go` | 212 | → `window_manager.rs` |
| 热键 | `internal/hotkey/hotkey.go` | 163 | → `tauri-plugin-global-shortcut` |
| 协调器 | `internal/coordinator/coordinator.go` | 123 | → `lib.rs` 热键同步逻辑 |
| 脚本加载 | `internal/scripts/loader.go` | 67 | → `script_injector.rs` |
| 日志 | `internal/log/log.go` | 121 | `env_logger`（简化）|
| 前端 HTML | `frontend/index.html` | 160 | 复制微调 |
| 前端 JS | `frontend/src/main.js` | 365 | 复制 + 替换 `invoke` |
| 前端 CSS | `frontend/public/style.css` | 249 | 复制 |
| 注入脚本 | `scripts/batch-delete.js` | 350 | 嵌入 Rust 字符串 |
| 注入脚本 | `scripts/focus-textarea.js` | 3 | 嵌入 Rust 字符串 |
| 托盘图标 | `build/appicon.png` | — | 复制到 `icons/` |

## 附录 B：Tauri 文档参考

| 主题 | 文档位置 |
|------|---------|
| 进程模型 | `D:\wiki\tauri\zh-cn\concept\process-model.md` |
| IPC 命令 | `D:\wiki\tauri\zh-cn\develop\calling-rust.mdx` |
| 事件系统 | `D:\wiki\tauri\zh-cn\develop\calling-frontend.mdx` |
| 全局快捷键 | `D:\wiki\tauri\zh-cn\plugin\global-shortcut.mdx` |
| 系统托盘 | `D:\wiki\tauri\zh-cn\learn\system-tray.mdx` |
| 自动启动 | `D:\wiki\tauri\zh-cn\plugin\autostart.mdx` |
| 窗口自定义 | `D:\wiki\tauri\zh-cn\learn\window-customization.mdx` |
| 窗口权限 | `D:\wiki\tauri\zh-cn\learn\Security\capabilities-for-windows-and-platforms.mdx` |
| 架构 | `D:\wiki\tauri\zh-cn\concept\architecture.mdx` |
| IPC 概述 | `D:\wiki\tauri\zh-cn\concept\Inter-Process Communication\index.mdx` |
