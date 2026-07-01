package hotkey

import (
	"context"
	"errors"
	"fmt"
	"log/slog"
	"strings"
	"time"

	"golang.design/x/hotkey"
)

const debounceInterval = 200 * time.Millisecond

type entry struct {
	hk   *hotkey.Hotkey
	stop context.CancelFunc
}

// Service 管理系统级全局热键（支持多个同时注册）
type Service struct {
	entries map[string]*entry
}

// NewService 创建热键服务
func NewService() *Service {
	return &Service{entries: make(map[string]*entry)}
}

// Register 注册系统级全局热键
func (s *Service) Register(keystr string, callback func()) error {
	if keystr == "" {
		return nil
	}
	mods, key, err := parse(keystr)
	if err != nil {
		return fmt.Errorf("快捷键格式无效: %w", err)
	}

	if e, ok := s.entries[keystr]; ok {
		e.stop()
		e.hk.Unregister()
		delete(s.entries, keystr)
	}

	slog.Info("hotkey register", "key", keystr, "mods", mods, "keycode", key)
	nhk := hotkey.New(mods, key)
	if err := nhk.Register(); err != nil {
		msg := err.Error()
		if strings.Contains(msg, "already registered") {
			return fmt.Errorf("热键 %s 已被其他程序占用", keystr)
		}
		return fmt.Errorf("热键 %s 注册失败: %s", keystr, msg)
	}

	ctx, cancel := context.WithCancel(context.Background())
	s.entries[keystr] = &entry{hk: nhk, stop: cancel}
	go func() {
		var lastFire time.Time
		for {
			select {
			case <-ctx.Done():
				return
			case <-nhk.Keydown():
				now := time.Now()
				if now.Sub(lastFire) < debounceInterval {
					continue // ponytail: 底层库 PeekMessage+10ms 轮询在特定时序下同一按键触发两次 keydown, 用简单去重兜底
				}
				lastFire = now
				slog.Info("hotkey triggered", "key", keystr)
				callback()
			}
		}
	}()
	return nil
}

// Unregister 注销指定热键
func (s *Service) Unregister(keystr string) {
	if e, ok := s.entries[keystr]; ok {
		e.stop()
		e.hk.Unregister()
		delete(s.entries, keystr)
	}
}

// UnregisterAll 注销所有热键
func (s *Service) UnregisterAll() {
	for k, e := range s.entries {
		e.stop()
		e.hk.Unregister()
		delete(s.entries, k)
	}
}

func parse(s string) ([]hotkey.Modifier, hotkey.Key, error) {
	var mods []hotkey.Modifier
	var key hotkey.Key

	parts := strings.Split(s, "+")
	for _, p := range parts {
		p = strings.TrimSpace(p)
		switch strings.ToLower(p) {
		case "ctrl", "control":
			mods = append(mods, hotkey.ModCtrl)
		case "alt":
			mods = append(mods, hotkey.ModAlt)
		case "shift":
			mods = append(mods, hotkey.ModShift)
		case "win", "windows", "cmd":
			mods = append(mods, hotkey.ModWin)
		default:
			key = parseKey(p)
		}
	}

	if key == 0 {
		return nil, 0, errors.New("hotkey: invalid key")
	}
	return mods, key, nil
}

func parseKey(s string) hotkey.Key {
	s = strings.ToUpper(s)
	if len(s) == 1 {
		ch := s[0]
		if ch >= 'A' && ch <= 'Z' {
			return hotkey.Key(ch)
		}
		if ch >= '0' && ch <= '9' {
			return hotkey.Key(ch)
		}
	}
	switch s {
	case "F1":
		return hotkey.KeyF1
	case "F2":
		return hotkey.KeyF2
	case "F3":
		return hotkey.KeyF3
	case "F4":
		return hotkey.KeyF4
	case "F5":
		return hotkey.KeyF5
	case "F6":
		return hotkey.KeyF6
	case "F7":
		return hotkey.KeyF7
	case "F8":
		return hotkey.KeyF8
	case "F9":
		return hotkey.KeyF9
	case "F10":
		return hotkey.KeyF10
	case "F11":
		return hotkey.KeyF11
	case "F12":
		return hotkey.KeyF12
	}
	return 0
}
