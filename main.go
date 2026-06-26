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
)

//go:embed all:frontend/dist
var assets embed.FS

//go:embed scripts/*.js
var embeddedScripts embed.FS

func main() {
	// ---- 日志：按小时轮转 + 72 小时保留 ----
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
	slog.Info("settings loaded", "entries", len(initialSettings.Entries), "scripts_released", initialSettings.ScriptsReleased)
	for i, e := range initialSettings.Entries {
		slog.Info("entry", "index", i, "id", e.ID, "url", e.URL, "hotkey", e.Hotkey, "script_dir", e.ScriptDir)
	}
	if !initialSettings.ScriptsReleased && len(initialSettings.Entries) > 0 {
		firstDir := sett.ScriptDir(initialSettings.Entries[0].ScriptDir)
		slog.Info("releasing embedded scripts", "dir", firstDir)
		if err := scripts.ReleaseEmbedded(firstDir, embeddedScripts); err != nil {
			slog.Warn("release embedded scripts failed", "error", err)
		} else {
			_ = sett.MarkScriptsReleased()
			slog.Info("scripts released", "dir", firstDir)
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

	// ---- WebView 管理器 ----
	wm = webview.New(app)

	// ---- 热键服务 ----
	hkSvc := hotkey.NewService()

	// ---- 协调器 ----
	var coord *coordinator.Service
	focusJS := readFocusTextareaJS()
	onShow := func(entryID string, win application.Window) {
		scriptDir := sett.ScriptDir(
			getEntryScriptDir(initialSettings.Entries, entryID),
		)
		coord.InjectScripts(entryID, scriptDir, win)
		coord.InjectFocusScript(focusJS, win)
	}
	coord = coordinator.New(hkSvc, wm, sett, onShow)

	// 注册初始热键
	for _, entry := range initialSettings.Entries {
		if entry.ID == "default" {
			coord.RegisterEntry(entry, func() {
				if wm.IsVisible() {
					wm.Hide()
				} else {
					wm.Show()
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

	// ---- 主窗口 ----
	if !sett.GetSettings().StartMinimized {
		wm.Show()
		slog.Info("main window shown", "has_window", wm.Window() != nil)
		if len(initialSettings.Entries) > 0 {
			first := initialSettings.Entries[0]
			scriptDir := sett.ScriptDir(first.ScriptDir)
			slog.Info("injecting scripts into main window", "entry", first.ID, "dir", scriptDir)
			coord.InjectScripts(first.ID, scriptDir, wm.Window())
			coord.InjectFocusScript(focusJS, wm.Window())
		}
	}

	// ---- 设置窗口 ----
	settingsWindow := app.Window.NewWithOptions(application.WebviewWindowOptions{
		Name:      "settings",
		Title:     "设置",
		Width:     700,
		Height:    620,
		MinWidth:  600,
		MinHeight: 400,
		Hidden:    true,
		URL:       "/",
	})

	// ---- 系统托盘 ----
	tray := app.SystemTray.New()
	if iconBytes, err := os.ReadFile("build/appicon.png"); err == nil {
		tray.SetIcon(iconBytes)
	}
	tray.SetLabel("DeepSeek")

	trayMenu := application.NewContextMenu("tray-menu")
	trayMenu.Add("显示窗口").OnClick(func(_ *application.Context) { wm.Show() })
	trayMenu.Add("设置").OnClick(func(_ *application.Context) {
		settingsWindow.Show()
		settingsWindow.Focus()
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

func readFocusTextareaJS() string {
	data, err := embeddedScripts.ReadFile("scripts/focus-textarea.js")
	if err != nil {
		slog.Warn("read focus-textarea.js failed", "error", err)
		return ""
	}
	return string(data)
}

func getEntryScriptDir(entries []settings.SiteEntry, entryID string) string {
	for _, e := range entries {
		if e.ID == entryID {
			return e.ScriptDir
		}
	}
	return ""
}
