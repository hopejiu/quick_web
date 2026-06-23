# ADR 0001: Settings & WebView Architecture

## 状态
已更新 (2026-06-23) — 反映了第三次迭代后的实际架构

## 背景
为 DeepSeek 桌面端实现设置管理、热键配置、WebView 生命周期管理等功能。

## 决策

### 设置持久化
- 使用 `encoding/json` 将设置写入 JSON 文件
- 存储路径: `os.UserConfigDir()/deepseek-pc/settings.json`

### 热键管理
- 使用 `golang.design/x/hotkey` 实现系统级全局热键（即使窗口隐藏也能响应）
- 热键变更采用 `RegisterAndSwap` 模式：先注册新热键，成功后再注销旧热键
- 格式校验在 Go 端完成，失败时返回 error 给前端

### WebView 生命周期
- **定时器销毁**: 窗口隐藏后启动 5 分钟定时器，超时自动销毁 WebView 释放内存
- **按需重建**: 再次唤起时（Alt+E / 托盘点击）若窗口已销毁则创建全新的
- **关闭但不退出**: 点击关闭按钮 → 隐藏窗口 → 启动销毁定时器
- 使用 `forceClose atomic.Bool` 控制真正的关闭 vs 隐藏
- 热键回调通过 `ToggleHandler` 接口与具体窗口实现解耦

### 脚本注入
- 脚本存储在 `scripts/` 目录的 `.js` 文件中，通过 `go:embed` 编译进二进制
- 通过 `WebviewWindow.ExecJS()` 运行时注入
- 每次新窗口创建时启动注入调度器（3 次重试 × 3 秒间隔）
- ExecJS 依赖 `runtimeLoaded` 标志，通过 `HandleMessage("wails:runtime:ready")` 手动触发

### 窗口架构
- **完全分离的前端**：主窗口加载外部 URL，设置窗口加载本地资产
- **标准窗口**（非 Frameless）：保留标题栏和关闭/最小化/最大化按钮
- **单例**：Wails v3 内置 `SingleInstanceOptions`，UniqueID=com.deepseek.desktop

### 架构模块化
- `internal/webview/` — WebView 生命周期管理器（窗口创建/销毁/定时器）
- `internal/injector/` — 脚本注入调度器（重试/时机管理）
- `internal/hotkey/` — 系统级全局热键（`golang.design/x/hotkey`）
- `internal/settings/` — 设置数据模型 + JSON 持久化
- `main.go` — 仅编排，不包含业务逻辑实现

### 快捷键 UI
- 修饰键下拉框（单选: Alt/Ctrl/Shift/Win）
- 按键选择器（A-Z, 0-9, F1-F12）

## 历史变更
| 日期 | 变更 |
|------|------|
| 2026-06-23 | 初始创建：Wails KeyBinding + Frameless + 用户脚本管理 |
| 2026-06-23 | 改用 `golang.design/x/hotkey` 系统级热键；移除 Frameless |
| 2026-06-23 | 移除自定义脚本功能；硬编码注入批量删除脚本 |
| 2026-06-23 | 添加 5 分钟定时器销毁机制 |
| 2026-06-23 | 架构重构：提取 webview/injector 包，ToggleHandler 接口 |
