package hotkey

import (
	"errors"
	"fmt"
	"log"
	"strings"

	"golang.design/x/hotkey"
)

// Service 管理系统级全局热键
type Service struct {
	hk *hotkey.Hotkey
}

// NewService 创建热键服务
func NewService() *Service {
	return &Service{}
}

// Register 注册系统级全局热键，格式: "Alt+E", "Ctrl+Shift+A"
func (s *Service) Register(keystr string, callback func()) error {
	mods, key, err := parse(keystr)
	if err != nil {
		return fmt.Errorf("快捷键格式无效: %w", err)
	}

	log.Printf("[hotkey] Register %q (mods=%v key=%v)", keystr, mods, key)
	nhk := hotkey.New(mods, key)
	if err := nhk.Register(); err != nil {
		msg := err.Error()
		if strings.Contains(msg, "already registered") {
			return fmt.Errorf("热键 %s 已被其他程序占用", keystr)
		}
		return fmt.Errorf("热键 %s 注册失败: %s", keystr, msg)
	}

	s.hk = nhk
	go func() {
		for range s.hk.Keydown() {
			log.Println("[hotkey] Keydown triggered")
			callback()
		}
	}()
	return nil
}

// Unregister 注销当前热键
func (s *Service) Unregister() {
	if s.hk != nil {
		s.hk.Unregister()
		s.hk = nil
	}
}

// RegisterAndSwap 安全替换热键：先注册新热键，成功后再注销旧热键
func (s *Service) RegisterAndSwap(keystr string, callback func()) error {
	mods, key, err := parse(keystr)
	if err != nil {
		return fmt.Errorf("快捷键格式无效")
	}

	log.Printf("[hotkey] RegisterAndSwap trying %q (mods=%v key=%v)", keystr, mods, key)
	nhk := hotkey.New(mods, key)
	if err := nhk.Register(); err != nil {
		msg := err.Error()
		if strings.Contains(msg, "already registered") {
			return fmt.Errorf("热键 %s 已被其他程序占用", keystr)
		}
		return fmt.Errorf("热键 %s 注册失败: %s", keystr, msg)
	}

	// 新热键注册成功，注销旧的
	if s.hk != nil {
		s.hk.Unregister()
	}
	s.hk = nhk

	go func() {
		for range s.hk.Keydown() {
			log.Println("[hotkey] Keydown triggered")
			callback()
		}
	}()
	return nil
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
