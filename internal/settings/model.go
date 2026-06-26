package settings

// SiteEntry 站点配置条目
type SiteEntry struct {
	ID         string `json:"id"`          // 唯一标识
	URL        string `json:"url"`         // 网址
	Name       string `json:"name"`        // 显示名称
	Hotkey     string `json:"hotkey"`      // 快捷键，如 "Alt+1"
	ScriptDir  string `json:"script_dir"`  // 脚本目录名（相对路径）
}

// Data 应用设置
type Data struct {
	Entries        []SiteEntry `json:"entries"`                   // 站点条目列表
	AutoStart      bool        `json:"auto_start"`                // 开机自启
	StartMinimized bool        `json:"start_minimized"`           // 启动时最小化
	ScriptsReleased bool       `json:"scripts_released,omitempty"` // 是否已释放内置脚本
	// 旧版兼容字段（迁移用，新设置不再写入）
	Hotkey string `json:"hotkey,omitempty"`
}
