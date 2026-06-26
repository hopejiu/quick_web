package webview

import (
	"log/slog"
	"sync/atomic"
	"time"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

const destroyTimeout = 1 * time.Minute

// Manager 管理 WebView 窗口的生命周期
type Manager struct {
	win          application.Window
	destroyTimer *time.Timer
	forceClose   atomic.Bool
	app          *application.App

	siteWindows map[string]application.Window
	siteTimers  map[string]*time.Timer
}

// New 创建 Manager
func New(app *application.App) *Manager {
	return &Manager{
		app:         app,
		siteWindows: make(map[string]application.Window),
		siteTimers:  make(map[string]*time.Timer),
	}
}

// Window 返回当前主窗口实例
func (m *Manager) Window() application.Window {
	return m.win
}

// Show 显示主窗口
func (m *Manager) Show() {
	if m.destroyTimer != nil {
		m.destroyTimer.Stop()
		m.destroyTimer = nil
	}
	if m.win == nil {
		slog.Info("creating main window")
		m.win = m.createWindow("main", "https://chat.deepseek.com", "DeepSeek", 1200, 800,
			func() { m.Hide() },
		)
	}
	m.bringToFront(m.win)
}

// Hide 隐藏主窗口
func (m *Manager) Hide() {
	if m.win == nil {
		return
	}
	m.win.Hide()
	m.startDestroyTimer()
}

// IsVisible 返回主窗口是否可见
func (m *Manager) IsVisible() bool {
	return m.win != nil && m.win.IsVisible()
}

// SiteShow 显示站点窗口
func (m *Manager) SiteShow(entryID, url, title string) {
	if t, ok := m.siteTimers[entryID]; ok {
		t.Stop()
		delete(m.siteTimers, entryID)
	}
	if _, ok := m.siteWindows[entryID]; !ok {
		slog.Info("creating site window", "entry", entryID, "url", url)
		m.siteWindows[entryID] = m.createWindow("site-"+entryID, url, title, 1200, 800,
			func() { m.SiteHide(entryID) },
		)
	}
	m.bringToFront(m.siteWindows[entryID])
}

// SiteHide 隐藏站点窗口
func (m *Manager) SiteHide(entryID string) {
	w, ok := m.siteWindows[entryID]
	if !ok {
		return
	}
	w.Hide()
	m.startSiteDestroyTimer(entryID)
}

// SiteIsVisible 站点窗口是否可见
func (m *Manager) SiteIsVisible(entryID string) bool {
	w, ok := m.siteWindows[entryID]
	return ok && w != nil && w.IsVisible()
}

// SiteWindow 返回站点窗口
func (m *Manager) SiteWindow(entryID string) application.Window {
	return m.siteWindows[entryID]
}

// DestroySiteWindow 销毁站点窗口
func (m *Manager) DestroySiteWindow(entryID string) {
	if t, ok := m.siteTimers[entryID]; ok {
		t.Stop()
		delete(m.siteTimers, entryID)
	}
	if w, ok := m.siteWindows[entryID]; ok {
		m.forceClose.Store(true)
		w.Close()
		m.forceClose.Store(false)
		delete(m.siteWindows, entryID)
	}
}

func (m *Manager) createWindow(name, url, title string, width, height int, onHide func()) application.Window {
	w := m.app.Window.NewWithOptions(application.WebviewWindowOptions{
		Name:            name,
		Title:           title,
		Width:           width,
		Height:          height,
		MinWidth:        800,
		MinHeight:       600,
		URL:             url,
		DevToolsEnabled: true,
		KeyBindings: map[string]func(window application.Window){
			"F12": func(w application.Window) {
				slog.Info("DevTools opened", "window", name)
				w.OpenDevTools()
			},
		},
	})
	w.HandleMessage("wails:runtime:ready")
	w.RegisterHook(events.Common.WindowClosing, func(e *application.WindowEvent) {
		if m.forceClose.Load() {
			return
		}
		onHide()
		e.Cancel()
	})
	w.OnWindowEvent(events.Common.WindowMinimise, func(e *application.WindowEvent) {
		onHide()
	})
	return w
}

func (m *Manager) bringToFront(w application.Window) {
	w.Show()
	w.Focus()
	w.SetAlwaysOnTop(true)
	w.SetAlwaysOnTop(false)
	w.Focus()
}

func (m *Manager) startDestroyTimer() {
	if m.destroyTimer != nil {
		m.destroyTimer.Stop()
	}
	m.destroyTimer = time.AfterFunc(destroyTimeout, func() {
		slog.Info("destroy timer expired, closing main window")
		m.forceClose.Store(true)
		application.InvokeSync(func() {
			if m.win != nil {
				m.win.Close()
				m.win = nil
			}
		})
		m.forceClose.Store(false)
	})
}

func (m *Manager) startSiteDestroyTimer(entryID string) {
	if t, ok := m.siteTimers[entryID]; ok {
		t.Stop()
	}
	m.siteTimers[entryID] = time.AfterFunc(destroyTimeout, func() {
		slog.Info("destroy timer expired, closing site window", "entry", entryID)
		m.forceClose.Store(true)
		application.InvokeSync(func() {
			if w, ok := m.siteWindows[entryID]; ok {
				w.Close()
				delete(m.siteWindows, entryID)
			}
		})
		m.forceClose.Store(false)
		delete(m.siteTimers, entryID)
	})
}
