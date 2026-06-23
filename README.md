# DeepSeek Desktop

将 [DeepSeek Chat](https://chat.deepseek.com) 封装为原生桌面应用，提供系统托盘、全局快捷键、批量删除对话等增强功能。

## 功能

- **内置 WebView** 加载 chat.deepseek.com，无浏览器工具栏
- **系统托盘** — 关闭/最小化窗口不退出，驻留托盘后台运行
- **定时销毁** — 隐藏后 5 分钟自动销毁 WebView，释放内存；再次唤起时重建
- **全局快捷键** — 默认 `Alt+E` 呼出/隐藏窗口（支持在设置中修改）
- **自动聚焦输入框** — 呼出窗口后自动定位到消息输入框
- **批量删除对话** — 侧边栏增加"批量删除"功能，支持勾选/反选/确认删除
- **开机自启** — 可选登录时自动启动
- **单例模式** — 只允许一个实例，重复点击 exe 唤出已有窗口
- **F12 开发者工具** — 按 F12 打开 WebView DevTools

## 使用方法

```bash
# 开发模式（热重载 + DevTools）
wails3 dev

# 构建可执行文件（含 DevTools）
wails3 build DEV=true

# 构建发布版（无 DevTools）
wails3 build
```

构建产物在 `bin/deepseek-pc.exe`。

## 快捷键设置

在设置窗口（托盘右键 → 设置）中可修改全局快捷键：

- **修饰键**：Alt（默认）/ Ctrl / Shift / Win
- **按键**：A-Z / 0-9 / F1-F12

## 应用图标

应用图标源文件为 `logo.png`。更换图标后重新生成：

```bash
copy logo.png build\appicon.png
wails3 generate icons -input build/appicon.png
wails3 build DEV=true
```

## 项目结构

```
deepseek-pc/
├── main.go                     # 应用入口（仅编排）
├── scripts/
│   ├── batch-delete.js         # 批量删除对话脚本（go:embed）
│   └── focus-textarea.js       # 输入框聚焦脚本（go:embed）
├── internal/
│   ├── webview/manager.go      # WebView 生命周期管理器
│   ├── injector/injector.go    # 脚本注入调度器
│   ├── hotkey/hotkey.go        # 系统级全局热键
│   └── settings/               # 设置数据模型 + JSON 持久化
├── frontend/                   # 设置页面（Vite + vanilla JS）
├── build/                      # 构建配置 + 图标源文件
├── docs/adr/                   # 架构决策记录
├── CONTEXT.md                  # 术语表
└── README.md
```

## 技术栈

- **框架**：Wails v3 (v3.0.0-alpha.97)
- **前端**：Vite 8 + 原生 JavaScript
- **热键**：`golang.design/x/hotkey`（系统级全局热键，窗口隐藏也可唤醒）
- **设置**：JSON 文件（`%AppData%/deepseek-pc/settings.json`）
- **编译嵌入**：Go `//go:embed` 将 JS 脚本编译进二进制
