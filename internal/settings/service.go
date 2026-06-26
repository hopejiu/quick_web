package settings

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
)

// Service 设置服务，管理设置读写
type Service struct {
	mu             sync.RWMutex
	path           string
	data           Data
	cfgDir         string
	onSave         func(old, new Data) // SaveSettings 保存后的回调
}

// NewService 创建设置服务。cfgDir 是存放 settings.json 的目录。
func NewService(cfgDir string) *Service {
	return &Service{
		path:   filepath.Join(cfgDir, "settings.json"),
		data:   defaults(),
		cfgDir: cfgDir,
	}
}

// OnSave 注册保存后的回调（用于热键重新注册）
func (s *Service) OnSave(fn func(old, new Data)) {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.onSave = fn
}

// Load 从磁盘读取设置，失败时使用默认值；首次加载会初始化默认站点条目
func (s *Service) Load() error {
	s.mu.Lock()
	defer s.mu.Unlock()

	data, err := os.ReadFile(s.path)
	if err != nil {
		if os.IsNotExist(err) {
			s.data = defaults()
			s.initDefaults()
			return s.saveLocked()
		}
		return fmt.Errorf("read settings: %w", err)
	}

	loaded := defaults()
	if err := json.Unmarshal(data, &loaded); err != nil {
		return fmt.Errorf("parse settings: %w", err)
	}
	s.data = loaded
	s.initDefaults()
	return nil
}

// initDefaults 首次初始化：创建默认 DeepSeek 站点条目
func (s *Service) initDefaults() {
	if len(s.data.Entries) > 0 {
		return
	}
	// 迁移旧版单热键
	hk := s.data.Hotkey
	if hk == "" {
		hk = "Alt+E"
	}
	s.data.Hotkey = ""
	s.data.Entries = []SiteEntry{
		{
			ID:        "default",
			URL:       "https://chat.deepseek.com",
			Name:      "DeepSeek Chat",
			Hotkey:    hk,
			ScriptDir: "chat.deepseek.com",
		},
	}
	s.data.ScriptsReleased = false
}

// Save 将当前设置写入磁盘。
// 使用 RLock 因为只读取 s.data（json.MarshalIndent 是纯读操作），
// 磁盘写入（os.WriteFile）不涉及共享内存，不需要写锁。
func (s *Service) Save() error {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.saveLocked()
}

// saveLocked 内部保存（调用者需持有锁）
func (s *Service) saveLocked() error {
	data, err := json.MarshalIndent(s.data, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal settings: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(s.path), 0700); err != nil {
		return fmt.Errorf("create settings dir: %w", err)
	}
	return os.WriteFile(s.path, data, 0600)
}

// GetSettings 返回当前设置的副本
func (s *Service) GetSettings() Data {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.data
}

// SaveSettings 更新并持久化设置，保存后触发回调
func (s *Service) SaveSettings(d Data) error {
	s.mu.Lock()
	old := s.data
	cb := s.onSave
	s.data = d
	s.mu.Unlock()

	if err := s.Save(); err != nil {
		return err
	}
	if cb != nil {
		cb(old, d)
	}
	return nil
}

// ScriptDir 返回指定条目的脚本绝对路径
func (s *Service) ScriptDir(dirName string) string {
	return filepath.Join(s.cfgDir, "scripts", dirName)
}

// OpenScriptDir 用资源管理器打开脚本目录
func (s *Service) OpenScriptDir(dirName string) error {
	dir := filepath.Join(s.cfgDir, "scripts", dirName)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return err
	}
	return exec.Command("explorer", dir).Start()
}

// MarkScriptsReleased 标记内置脚本已释放
func (s *Service) MarkScriptsReleased() error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.data.ScriptsReleased = true
	return s.saveLocked()
}

func defaults() Data {
	return Data{
		Entries:        nil,
		AutoStart:      false,
		StartMinimized: false,
		ScriptsReleased: false,
	}
}

// ConfigDir 返回设置文件所在目录
func ConfigDir() (string, error) {
	cfgDir, err := os.UserConfigDir()
	if err != nil {
		return "", err
	}
	return filepath.Join(cfgDir, "deepseek-pc"), nil
}

// DirNameFromURL 从 URL 提取 hostname 作为脚本目录名
func DirNameFromURL(url string) string {
	host := url
	if i := strings.Index(host, "://"); i >= 0 {
		host = host[i+3:]
	}
	if i := strings.Index(host, "/"); i >= 0 {
		host = host[:i]
	}
	host = strings.TrimPrefix(host, "www.")
	if host == "" {
		host = "unknown"
	}
	return host
}
