package coordinator

import (
	"log/slog"
	"time"

	"changeme/internal/hotkey"
	"changeme/internal/scripts"
	"changeme/internal/settings"
	"changeme/internal/webview"

	"github.com/wailsapp/wails/v3/pkg/application"
)

// OnShowCallback 窗口显示后的脚本注入回调
type OnShowCallback func(entryID string, win application.Window)

// ToggleFunc 自定义显示/隐藏逻辑（用于主窗口等特殊窗口）
type ToggleFunc func()

// Service 协调站点生命周期：热键注册 + 窗口管理 + 脚本注入
type Service struct {
	hkSvc   *hotkey.Service
	wm      *webview.Manager
	sett    *settings.Service
	onShow  OnShowCallback
	toggles map[string]ToggleFunc
}

// New 创建协调器
func New(hkSvc *hotkey.Service, wm *webview.Manager, sett *settings.Service, onShow OnShowCallback) *Service {
	return &Service{hkSvc: hkSvc, wm: wm, sett: sett, onShow: onShow, toggles: make(map[string]ToggleFunc)}
}

// RegisterEntry 为单个站点条目注册热键
func (c *Service) RegisterEntry(entry settings.SiteEntry, toggle ToggleFunc) {
	if entry.Hotkey == "" {
		return
	}
	if toggle != nil {
		c.toggles[entry.ID] = toggle
	}
	err := c.hkSvc.Register(entry.Hotkey, func() {
		if fn, ok := c.toggles[entry.ID]; ok && fn != nil {
			fn()
			return
		}
		if c.wm.SiteIsVisible(entry.ID) {
			c.wm.SiteHide(entry.ID)
			return
		}
		created := c.wm.SiteShow(entry.ID, entry.URL, entry.Name)
		// 仅新创建的窗口需要注入脚本
		if created && c.onShow != nil {
			c.onShow(entry.ID, c.wm.SiteWindow(entry.ID))
		}
	})
	if err != nil {
		slog.Warn("hotkey register failed", "entry", entry.Name, "hotkey", entry.Hotkey, "error", err)
	}
}

// SyncEntries diff 新旧 entries，注销/重注册热键，销毁已删窗口
func (c *Service) SyncEntries(old, new settings.Data) {
	oldMap := make(map[string]settings.SiteEntry)
	for _, e := range old.Entries {
		oldMap[e.ID] = e
	}
	newMap := make(map[string]settings.SiteEntry)
	for _, e := range new.Entries {
		newMap[e.ID] = e
	}
	for id, oe := range oldMap {
		ne, exists := newMap[id]
		if !exists || ne.Hotkey != oe.Hotkey {
			c.hkSvc.Unregister(oe.Hotkey)
		}
	}
	for id, ne := range newMap {
		oe, existed := oldMap[id]
		if !existed || ne.Hotkey != oe.Hotkey {
			c.RegisterEntry(ne, c.toggles[id])
		}
	}
	for id := range oldMap {
		if _, exists := newMap[id]; !exists {
			c.wm.DestroySiteWindow(id)
		}
	}
}

// InjectScripts 将站点脚本注入到窗口（带重试，等待页面加载）
func (c *Service) InjectScripts(entryID, scriptDir string, win application.Window) {
	if win == nil {
		slog.Warn("InjectScripts: win is nil", "entry", entryID)
		return
	}
	slog.Info("InjectScripts scheduled", "entry", entryID, "dir", scriptDir)
	go func() {
		for i := 0; i < 3; i++ {
			time.Sleep(1 * time.Second)
			jsList, err := scripts.LoadFromDir(scriptDir)
			if err != nil {
				slog.Warn("InjectScripts load failed", "entry", entryID, "attempt", i+1, "error", err)
				continue
			}
			if len(jsList) == 0 {
				slog.Warn("InjectScripts no scripts found", "entry", entryID, "attempt", i+1, "dir", scriptDir)
				continue
			}
			slog.Info("InjectScripts injecting", "entry", entryID, "attempt", i+1, "count", len(jsList))
			for _, js := range jsList {
				win.ExecJS(js)
			}
			slog.Info("InjectScripts done", "entry", entryID)
			return
		}
		slog.Error("InjectScripts all attempts failed", "entry", entryID, "dir", scriptDir)
	}()
}


