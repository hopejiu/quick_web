package log

import (
	"fmt"
	"io"
	golog "log"
	"log/slog"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

var (
	logDir    string
	curFile   *os.File
	curName   string
	mu        sync.Mutex
	DevMode   bool // 开发模式下同时输出到 stderr
	retention = 72 * time.Hour
)

// Init 初始化日志系统：创建目录、打开日志文件、启动轮转/清理 goroutine。
func Init(appDataDir string) error {
	logDir = filepath.Join(appDataDir, "logs")
	if err := os.MkdirAll(logDir, 0700); err != nil {
		return fmt.Errorf("create log dir: %w", err)
	}
	if err := switchLog(); err != nil {
		return err
	}
	go func() {
		for range time.Tick(1 * time.Minute) {
			rotateIfNeeded()
			cleanOldLogs()
		}
	}()
	return nil
}

func switchLog() error {
	mu.Lock()
	defer mu.Unlock()

	name := logFileName()
	if curFile != nil && curName == name {
		return nil // 同一个小时，无需切换
	}

	if curFile != nil {
		curFile.Close()
	}

	f, err := os.OpenFile(filepath.Join(logDir, name), os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0600)
	if err != nil {
		return fmt.Errorf("open log file: %w", err)
	}

	curFile = f
	curName = name

	var w io.Writer = f
	if DevMode {
		w = io.MultiWriter(os.Stderr, f)
	}

	h := slog.NewTextHandler(w, &slog.HandlerOptions{Level: slog.LevelDebug})
	slog.SetDefault(slog.New(h))
	golog.SetOutput(w)

	return nil
}

func rotateIfNeeded() {
	mu.Lock()
	name := logFileName()
	needSwitch := curName != name
	mu.Unlock()

	if needSwitch {
		switchLog()
	}
}

func cleanOldLogs() {
	cutoff := time.Now().Add(-retention)
	entries, err := os.ReadDir(logDir)
	if err != nil {
		return
	}
	for _, e := range entries {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".log") {
			continue
		}
		info, err := e.Info()
		if err != nil {
			continue
		}
		if info.ModTime().Before(cutoff) {
			mu.Lock()
			skip := e.Name() == curName
			mu.Unlock()
			if !skip {
				os.Remove(filepath.Join(logDir, e.Name()))
			}
		}
	}
}

func logFileName() string {
	return fmt.Sprintf("deepseek-pc-%s.log", time.Now().Format("2006-01-02-15"))
}

// 便利函数

func Info(msg string, args ...any)  { slog.Info(msg, args...) }
func Warn(msg string, args ...any)  { slog.Warn(msg, args...) }
func Error(msg string, args ...any) { slog.Error(msg, args...) }
func Debug(msg string, args ...any) { slog.Debug(msg, args...) }
