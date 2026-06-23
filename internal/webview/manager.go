package webview

import (
	"log"
	"sync/atomic"
	"time"

	"github.com/wailsapp/wails/v3/pkg/application"
	"github.com/wailsapp/wails/v3/pkg/events"
)

const destroyTimeout = 1 * time.Minute

// ToggleHandler 热键回调需要的窗口切换接口
type ToggleHandler interface {
	Show()
	Hide()
	IsVisible() bool
}

// Manager 管理主 WebView 窗口的生命周期
type Manager struct {
	win          application.Window
	destroyTimer *time.Timer
	forceClose   atomic.Bool
	app          *application.App
	afterCreate  []func(application.Window) // 每次新窗口创建后调用
}

// New 创建 Manager。窗口在首次调用 Show 时创建。
func New(app *application.App) *Manager {
	return &Manager{app: app}
}

// OnAfterCreate 注册新窗口创建后的回调（如脚本注入）
func (m *Manager) OnAfterCreate(fn func(application.Window)) {
	m.afterCreate = append(m.afterCreate, fn)
}

// Window 返回当前窗口实例（可能为 nil）
func (m *Manager) Window() application.Window {
	return m.win
}

// Show 显示窗口，必要时创建新窗口。强制带到最前。
func (m *Manager) Show() {
	if m.destroyTimer != nil {
		m.destroyTimer.Stop()
		m.destroyTimer = nil
	}
	if m.win == nil {
		m.win = m.newWindow()
	}
	m.win.Show()
	m.win.Focus()
	// 绕过 Windows 前台窗口限制：临时置顶再取消
	m.win.SetAlwaysOnTop(true)
	m.win.SetAlwaysOnTop(false)
	m.win.Focus()
}

// Hide 隐藏窗口，启动销毁定时器
func (m *Manager) Hide() {
	if m.win == nil {
		return
	}
	m.win.Hide()
	m.startDestroyTimer()
}

// IsVisible 返回窗口当前是否可见
func (m *Manager) IsVisible() bool {
	return m.win != nil && m.win.IsVisible()
}

// ForceClose 销毁旧窗口（用于定时器超时后的重建）
func (m *Manager) ForceClose() {
	if m.win == nil {
		return
	}
	m.forceClose.Store(true)
	m.win.Close()
	m.forceClose.Store(false)
	m.win = nil
}

func (m *Manager) startDestroyTimer() {
	if m.destroyTimer != nil {
		m.destroyTimer.Stop()
	}
	m.destroyTimer = time.AfterFunc(destroyTimeout, func() {
		log.Println("[webview] destroy timer expired, closing window")
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

func (m *Manager) newWindow() application.Window {
	w := m.app.Window.NewWithOptions(application.WebviewWindowOptions{
		Name:            "main",
		Title:           "DeepSeek",
		Width:           1200,
		Height:          800,
		MinWidth:        800,
		MinHeight:       600,
		URL:             "https://chat.deepseek.com",
		DevToolsEnabled: true,
		KeyBindings: map[string]func(window application.Window){
			"F12": func(w application.Window) {
				log.Println("[webview] F12 - opening DevTools")
				w.OpenDevTools()
			},
		},
	})
	w.HandleMessage("wails:runtime:ready")
	w.RegisterHook(events.Common.WindowClosing, func(e *application.WindowEvent) {
		if m.forceClose.Load() {
			return
		}
		m.Hide()
		e.Cancel()
	})
	w.OnWindowEvent(events.Common.WindowMinimise, func(e *application.WindowEvent) {
		m.Hide()
	})
	for _, fn := range m.afterCreate {
		fn(w)
	}
	return w
}
