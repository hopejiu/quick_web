# DeepSeek Desktop (deepseek-pc)

## 项目定位
将 DeepSeek Chat（https://chat.deepseek.com）封装为原生桌面应用，提供系统托盘、全局快捷键、WebView 生命周期管理、批量删除对话等增强功能。

## 术语表

| 术语 | 定义 |
|------|------|
| 主窗口 | 加载 `https://chat.deepseek.com` 的 WebviewWindow |
| 设置窗口 | 加载本地前端页面的 WebviewWindow，用于配置应用设置 |
| 系统托盘 | 操作系统通知区域的图标，应用关闭窗口后驻留于此 |
| 全局快捷键 | 系统级热键（`golang.design/x/hotkey`），用于呼出/隐藏主窗口 |
| WebView 管理器 | `internal/webview.Manager` — 封装窗口创建、隐藏、定时器销毁、按需重建 |
| 脚本注入器 | `internal/injector.Service` — 管理 JS 脚本的注入时机和重试 |
| ToggleHandler | WebView 管理器暴露的接口（Show/Hide/IsVisible），供热键回调调用 |
| 销毁定时器 | 窗口隐藏后 5 分钟自动销毁 WebView 释放内存 |

## 模块结构
- `internal/webview/` — WebView 生命周期管理器
- `internal/injector/` — 脚本注入调度器
- `internal/hotkey/` — 系统级全局热键
- `internal/settings/` — 设置数据模型 + JSON 持久化
- `scripts/` — 注入到主窗口的 JS 脚本（通过 `go:embed` 嵌入）

## 已知约定
- **设置持久化**: JSON 文件，存放于 `os.UserConfigDir()/deepseek-pc/settings.json`
- **模块名**: `changeme`
- **框架**: Wails v3（v3.0.0-alpha.97）+ 原生 JavaScript + Vite
- **托盘图标**: `build/appicon.png`
- **定时器超时**: 5 分钟（`internal/webview/manager.go` 中的 `destroyTimeout`）
