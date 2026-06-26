# DeepSeek Desktop (deepseek-pc)

## 项目定位
多站点桌面浏览器壳，为每个站点提供独立窗口、全局快捷键、脚本注入。默认站点 DeepSeek Chat，用户可自由添加更多站点。

## 术语表

| 术语 | 定义 |
|------|------|
| 站点条目 (SiteEntry) | 一条 URL + 快捷键 + 脚本目录的配置记录 |
| 主窗口 | 加载某站点 URL 的 WebviewWindow，每个站点独立窗口 |
| 设置窗口 | 加载本地前端页面的 WebviewWindow，用于配置应用设置 |
| 系统托盘 | 操作系统通知区域的图标，应用关闭窗口后驻留于此 |
| 全局快捷键 | 系统级热键（`golang.design/x/hotkey`），每个站点条目可绑定一个，用于呼出/隐藏对应窗口 |
| WebView 管理器 | `internal/webview.Manager` — 封装窗口创建、隐藏、定时器销毁、按需重建，支持多站点窗口 |
| 脚本注入器 | `internal/injector.Service` — 从文件系统加载 JS 脚本并注入到窗口 |
| 脚本释放 | 首次启动时将内置脚本从 `go:embed` 写入 appdata 目录，后续不再使用 embed |
| 销毁定时器 | 窗口隐藏后 1 分钟自动销毁 WebView 释放内存 |

## 模块结构
- `internal/webview/` — WebView 生命周期管理器（多站点窗口）
- `internal/injector/` — 脚本注入调度器
- `internal/hotkey/` — 多热键注册服务
- `internal/settings/` — 设置数据模型 + JSON 持久化 + 站点条目 CRUD
- `scripts/` — 脚本释放目录（`go:embed` 仅用于首次释放）

## 已知约定
- **设置持久化**: JSON 文件，存放于 `os.UserConfigDir()/deepseek-pc/settings.json`
- **脚本目录**: `os.UserConfigDir()/deepseek-pc/scripts/{dirName}/`，dirName 默认取 URL hostname
- **模块名**: `changeme`
- **框架**: Wails v3（v3.0.0-alpha.97）+ 原生 JavaScript + Vite
- **托盘图标**: `build/appicon.png`
- **定时器超时**: 1 分钟（`internal/webview/manager.go` 中的 `destroyTimeout`）
