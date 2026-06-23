package main

import (
	"embed"
	"log"
	"os"
	"time"

	"changeme/internal/hotkey"
	"changeme/internal/injector"
	"changeme/internal/settings"
	"changeme/internal/webview"

	"github.com/wailsapp/wails/v3/pkg/application"
)

//go:embed all:frontend/dist
var assets embed.FS

//go:embed scripts/batch-delete.js
var batchDeleteJS string

//go:embed scripts/focus-textarea.js
var focusTextareaJS string

func init() {
	application.RegisterEvent[string]("app:event")
}

func main() {
	// ---- 设置 ----
	cfgDir, err := settings.ConfigDir()
	if err != nil {
		log.Fatal(err)
	}
	sett := settings.NewService(cfgDir)
	if err := sett.Load(); err != nil {
		log.Printf("warn: load settings: %v", err)
	}

	// ---- 应用 ----
	var wm *webview.Manager // 单例回调中要用到，预先声明

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
				wm.Show() // 会重建窗口 + 强制前置
			},
		},
	})

	// ---- WebView 管理器 ----
	wm = webview.New(app)

	// 注册脚本注入：每次新窗口创建后定时注入
	scriptInjector := injector.New(5, time.Second)
	scriptInjector.Register(injector.Script{Name: "batch-delete", Content: batchDeleteJS})
	wm.OnAfterCreate(func(win application.Window) {
		scriptInjector.InjectInto(win)
	})

	wm.Show()

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

	// ---- 全局热键 ----
	hkSvc := hotkey.NewService()
	setupHotkey(hkSvc, wm, sett)

	// ---- 开机自启 ----
	if sett.GetSettings().AutoStart {
		if err := app.Autostart.Enable(); err != nil {
			log.Printf("warn: autostart enable failed: %v", err)
		}
	}

	if err := app.Run(); err != nil {
		log.Fatal(err)
	}
}

func setupHotkey(hkSvc *hotkey.Service, handler webview.ToggleHandler, sett *settings.Service) {
	toggle := func() {
		if handler.IsVisible() {
			handler.Hide()
			return
		}
		handler.Show()
		time.Sleep(500 * time.Millisecond)
		if wm, ok := handler.(*webview.Manager); ok {
			if win := wm.Window(); win != nil {
				win.ExecJS(focusTextareaJS)
			}
		}
	}

	hk := sett.GetSettings().Hotkey
	if hk != "" {
		if err := hkSvc.Register(hk, toggle); err != nil {
			log.Printf("warn: hotkey register failed: %v", err)
		}
	}

	sett.OnHotkeyChange(func(old, new string) error {
		if new == "" {
			hkSvc.Unregister()
			return nil
		}
		return hkSvc.RegisterAndSwap(new, toggle)
	})
}
