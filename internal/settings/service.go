package settings

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"sync"
)

// Service 设置服务，管理设置读写和热键变更回调
type Service struct {
	mu             sync.RWMutex
	path           string
	data           Data
	onHotkeyChange func(old, new string) error
}

// NewService 创建设置服务。basePath 是存放 settings.json 的目录。
func NewService(basePath string) *Service {
	return &Service{
		path: filepath.Join(basePath, "settings.json"),
		data: defaults(),
	}
}

// OnHotkeyChange 注册热键变更回调。回调返回 error 则会拒绝此次修改。
func (s *Service) OnHotkeyChange(fn func(old, new string) error) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.onHotkeyChange = fn
}

// Load 从磁盘读取设置，失败时使用默认值
func (s *Service) Load() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	data, err := os.ReadFile(s.path)
	if err != nil {
		if os.IsNotExist(err) {
			s.data = defaults()
			return nil
		}
		return fmt.Errorf("read settings: %w", err)
	}

	loaded := defaults()
	if err := json.Unmarshal(data, &loaded); err != nil {
		return fmt.Errorf("parse settings: %w", err)
	}
	s.data = loaded
	return nil
}

// Save 将当前设置写入磁盘
func (s *Service) Save() error {
	s.mu.RLock()
	defer s.mu.RUnlock()

	data, err := json.MarshalIndent(s.data, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal settings: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(s.path), 0700); err != nil {
		return fmt.Errorf("create settings dir: %w", err)
	}
	return os.WriteFile(s.path, data, 0600)
}

// GetSettings 返回当前设置的副本（供前端调用）
func (s *Service) GetSettings() Data {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.data
}

// SaveSettings 更新并持久化设置。如果热键变更，先验证新热键有效性。
func (s *Service) SaveSettings(d Data) error {
	s.mu.Lock()
	oldHotkey := s.data.Hotkey
	hkCB := s.onHotkeyChange
	s.mu.Unlock()

	if d.Hotkey != oldHotkey && hkCB != nil {
		if err := hkCB(oldHotkey, d.Hotkey); err != nil {
			return err
		}
	}

	s.mu.Lock()
	s.data = d
	s.mu.Unlock()

	return s.Save()
}

func defaults() Data {
	return Data{
		Hotkey:    "Alt+E",
		AutoStart: false,
	}
}

// ConfigDir 返回设置文件所在目录（默认使用 os.UserConfigDir() 的子目录）
func ConfigDir() (string, error) {
	cfgDir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(cfgDir, "deepseek-pc"), nil
}
