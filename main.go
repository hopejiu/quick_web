package main

import (
	"embed"
	"log/slog"
	"os"

	applog "changeme/internal/log"
	"changeme/internal/coordinator"
	"changeme/internal/hotkey"
	"changeme/internal/scripts"
	"changeme/internal/settings"
	"changeme/internal/webview"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

//go:embed all:frontend/dist
var assets embed.FS

//go:embed scripts/*.js
var embeddedScripts embed.FS

func main() {
	// ---- 日志 ----
	cfgDir, _ := settings.ConfigDir()
	if cfgDir == "" {
		cfgDir, _ = os.Getwd()
	}
	if err := applog.Init(cfgDir); err != nil {
		slog.Error("init logging failed", "error", err)
	}
	slog.Info("app started", "cfg_dir", cfgDir)

	// ---- 设置 ----
	sett := settings.NewService(cfgDir)
	if err := sett.Load(); err != nil {
		slog.Warn("load settings failed", "error", err)
	}

	// ---- 首次释放内置脚本 ----
	initialSettings := sett.GetSettings()
	slog.Info("settings loaded", "entries", len(initialSettings.Entries), "start_minimized", initialSettings.StartMinimized)
	for i, e := range initialSettings.Entries {
		slog.Info("entry", "index", i, "id", e.ID, "url", e.URL, "hotkey", e.Hotkey, "script_dir", e.ScriptDir)
	}
	if !initialSettings.ScriptsReleased && len(initialSettings.Entries) > 0 {
		firstDir := sett.ScriptDir(initialSettings.Entries[0].ScriptDir)
		if err := scripts.ReleaseEmbedded(firstDir, embeddedScripts); err != nil {
			slog.Warn("release embedded scripts failed", "error", err)
		} else {
			_ = sett.MarkScriptsReleased()
		}
	}

	// ---- 应用 ----
	var wm *webview.Manager

	app := application.New(application.Options{
		Name: "DeepSeek",
		Assets: application.AssetOptions{
			Handler: application.AssetFileServerFS(assets),
		},
		Services: []application.Service{
			application.NewService(sett),
		},
		SingleInstance: &application.SingleInstanceOptions{
			UniqueID: "com.deepseek.desktop",
			OnSecondInstanceLaunch: func(_ application.SecondInstanceData) {
				wm.Show()
			},
		},
	})

	// ---- 主窗口：设置窗口（始终存在，Hidden，保证应用不退出） ----
	settingsWindow := app.Window.NewWithOptions(application.WebviewWindowOptions{
		Name:            "settings",
		Title:           "设置",
		Width:           700,
		Height:          620,
		MinWidth:        600,
		MinHeight:       400,
		URL:             "http://wails.localhost/",
		DevToolsEnabled: true,
		Hidden:          true,
	})
	settingsWindow.RegisterHook(events.Common.WindowClosing, func(e *application.WindowEvent) {
		settingsWindow.Hide()
		e.Cancel()
	})

	// ---- 从窗口管理器（DeepSeek + 其他站点窗口，可被销毁） ----
	wm = webview.New(app)

	// ---- 热键服务 ----
	hkSvc := hotkey.NewService()

	// ---- 协调器 ----
	var coord *coordinator.Service
	onShow := func(entryID string, win application.Window) {
		scriptDir := sett.ScriptDir(getEntryScriptDir(initialSettings.Entries, entryID))
		coord.InjectScripts(entryID, scriptDir, win)
	}
	coord = coordinator.New(hkSvc, wm, sett, onShow)

	// 注册初始热键（默认条目 toggle 从窗口）
	for _, entry := range initialSettings.Entries {
		if entry.ID == "default" {
			entry := entry
			coord.RegisterEntry(entry, func() {
				if wm.IsVisible() {
					wm.Hide()
				} else if wm.Show() {
					// 窗口首次创建，InjectScripts 会从目录加载所有 .js
					coord.InjectScripts(entry.ID, sett.ScriptDir(entry.ScriptDir), wm.Window())
				}
			})
		} else {
			coord.RegisterEntry(entry, nil)
		}
	}

	// SaveSettings 后同步热键和窗口
	sett.OnSave(func(old, new settings.Data) {
		coord.SyncEntries(old, new)
	})

	// ---- 启动时显示从窗口（如果未设置最小化） ----
	if !initialSettings.StartMinimized {
		if wm.Show() && len(initialSettings.Entries) > 0 {
			first := initialSettings.Entries[0]
			coord.InjectScripts(first.ID, sett.ScriptDir(first.ScriptDir), wm.Window())
		}
	}

	// ---- 系统托盘 ----
	tray := app.SystemTray.New()
	if iconBytes, err := os.ReadFile("build/appicon.png"); err == nil {
		tray.SetIcon(iconBytes)
	}
	tray.SetLabel("DeepSeek")

	trayMenu := application.NewContextMenu("tray-menu")
	trayMenu.Add("显示窗口").OnClick(func(_ *application.Context) { wm.Show() })
	trayMenu.Add("设置").OnClick(func(_ *application.Context) {
		if settingsWindow.IsVisible() {
			settingsWindow.Focus()
		} else {
			settingsWindow.Show()
			settingsWindow.Focus()
		}
	})
	trayMenu.AddSeparator()
	trayMenu.Add("退出").OnClick(func(_ *application.Context) { app.Quit() })
	tray.SetMenu(trayMenu.Menu)
	tray.OnClick(func() { wm.Show() })

	// ---- 开机自启 ----
	if sett.GetSettings().AutoStart {
		if err := app.Autostart.Enable(); err != nil {
			slog.Warn("autostart enable failed", "error", err)
		}
	}

	if err := app.Run(); err != nil {
		slog.Error("app run failed", "error", err)
		os.Exit(1)
	}
}

func getEntryScriptDir(entries []settings.SiteEntry, entryID string) string {
	for _, e := range entries {
		if e.ID == entryID {
			return e.ScriptDir
		}
	}
	return ""
}
