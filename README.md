# DeepSeek PC (Tauri)

Go + Wails3 → Rust + Tauri 2 迁移版。

多站点桌面浏览器壳，为每个站点提供独立窗口、全局快捷键、脚本注入。

## 快速开始

```bash
# 安装依赖
pnpm install

# 开发模式
pnpm tauri dev

# 生产构建
pnpm tauri build
```

构建产物在 `src-tauri/target/release/bundle/` 下。

## 功能

- **多站点窗口**：加载外部 URL 的独立 WebView 窗口
- **全局热键**：每个站点可绑定 `Alt/Ctrl/Shift/Win + A-Z/0-9/F1-F12`
- **JS 脚本注入**：自动将 `scripts/{hostname}/*.js` 注入到对应站点
- **系统托盘**：驻留系统通知区，支持快速切换
- **单实例**：双击 exe 只启动一个实例
- **开机自启**：登录时自动启动
- **启动最小化**：启动后隐藏到系统托盘
- **文件日志**：按小时轮转，保留 72 小时

## 项目结构

```
src-tauri/
  src/
    lib.rs        # 主入口、热键、托盘、窗口管理、命令
    settings.rs   # 设置数据模型 + JSON 持久化
    logger.rs     # 文件日志（轮转 + 清理）
scripts/
  batch-delete.js    # DeepSeek Chat 批量删除对话
  focus-textarea.js  # 自动聚焦搜索框
src/
  main.js       # 设置页面逻辑（纯 JS）
  styles.css    # 设置页面样式
```
