package scripts

import (
	"embed"
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path/filepath"
	"strings"
)

// LoadFromDir 从指定目录读取所有 .js 文件内容。
func LoadFromDir(dir string) ([]string, error) {
	entries, err := os.ReadDir(dir)
	if err != nil {
		if os.IsNotExist(err) {
			slog.Warn("scripts dir not found", "dir", dir)
			return nil, nil
		}
		return nil, fmt.Errorf("read scripts dir: %w", err)
	}

	var scripts []string
	for _, e := range entries {
		if e.IsDir() || !strings.HasSuffix(e.Name(), ".js") {
			continue
		}
		data, err := os.ReadFile(filepath.Join(dir, e.Name()))
		if err != nil {
			return nil, fmt.Errorf("read script %s: %w", e.Name(), err)
		}
		slog.Info("script loaded", "file", e.Name(), "bytes", len(data))
		scripts = append(scripts, string(data))
	}
	slog.Info("LoadFromDir", "dir", dir, "count", len(scripts))
	return scripts, nil
}

// ReleaseEmbedded 首次启动时将 embed 脚本释放到 appdata 目录。
func ReleaseEmbedded(scriptsDir string, embedded embed.FS) error {
	if _, err := os.Stat(scriptsDir); err == nil {
		slog.Info("ReleaseEmbedded: dir exists, skip", "dir", scriptsDir)
		return nil
	}
	slog.Info("ReleaseEmbedded: creating dir", "dir", scriptsDir)
	if err := os.MkdirAll(scriptsDir, 0700); err != nil {
		return fmt.Errorf("create scripts dir: %w", err)
	}

	return fs.WalkDir(embedded, ".", func(path string, d fs.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() || !strings.HasSuffix(d.Name(), ".js") {
			return nil
		}
		data, err := embedded.ReadFile(path)
		if err != nil {
			return fmt.Errorf("read embedded %s: %w", path, err)
		}
		dest := filepath.Join(scriptsDir, d.Name())
		slog.Info("releasing script", "src", path, "dest", dest, "bytes", len(data))
		return os.WriteFile(dest, data, 0600)
	})
}
