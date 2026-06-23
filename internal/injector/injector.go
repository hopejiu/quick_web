package injector

import (
	"log"
	"time"

	"github.com/wailsapp/wails/v3/pkg/application"
)

// Script 待注入的脚本
type Script struct {
	Content string // JS 代码
	Name    string // 仅用于日志
}

// Service 管理脚本注入到 WebView 窗口
type Service struct {
	scripts []Script
	retries int
	delay   time.Duration
}

// New 创建脚本注入服务
// retries: 重试次数, delay: 每次重试间隔
func New(retries int, delay time.Duration) *Service {
	return &Service{
		retries: retries,
		delay:   delay,
	}
}

// Register 注册需要注入的脚本
func (s *Service) Register(scripts ...Script) {
	s.scripts = append(s.scripts, scripts...)
}

// InjectInto 将注册的脚本注入到指定窗口
func (s *Service) InjectInto(win application.Window) {
	for _, script := range s.scripts {
		script := script
		go func() {
			for i := 1; i <= s.retries; i++ {
				time.Sleep(s.delay)
				log.Printf("[injector] injecting %q (attempt %d/%d)", script.Name, i, s.retries)
				win.ExecJS(script.Content)
			}
		}()
	}
}
