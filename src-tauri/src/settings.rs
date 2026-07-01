use std::str::FromStr;
use std::fs;
use std::path::PathBuf;

use crate::hotkey::Hotkey;

/// 站点配置条目
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SiteEntry {
    pub id: String,
    pub url: String,
    pub name: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "hotkey_serde"
    )]
    pub hotkey: Option<Hotkey>,
    pub script_dir: String,
}

/// 自定义 Hotkey 序列化：None → 省略字段；Some(hk) → "Alt+E" 字符串
mod hotkey_serde {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::str::FromStr;
    use crate::hotkey::Hotkey;

    pub fn serialize<S: Serializer>(hk: &Option<Hotkey>, s: S) -> Result<S::Ok, S::Error> {
        match hk {
            Some(h) => s.serialize_str(&h.to_string()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Hotkey>, D::Error> {
        let s: Option<String> = Option::deserialize(d)?;
        match s {
            Some(s) if !s.is_empty() => Hotkey::from_str(&s).map(Some).map_err(serde::de::Error::custom),
            _ => Ok(None),
        }
    }
}

/// 应用设置，JSON 字段与原 deepseek-pc 兼容（snake_case）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Settings {
    pub entries: Vec<SiteEntry>,
    pub auto_start: bool,
    pub start_minimized: bool,
    #[serde(default)]
    pub scripts_released: bool,
    /// 旧版兼容（迁移用，新设置不再写入）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotkey: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            entries: vec![SiteEntry {
                id: "default".into(),
                url: "https://chat.deepseek.com".into(),
                name: "DeepSeek Chat".into(),
                hotkey: Some(Hotkey::from_str("Alt+E").expect("default hotkey is valid")),
                script_dir: "chat.deepseek.com".into(),
            }],
            auto_start: false,
            start_minimized: false,
            scripts_released: false,
            hotkey: None,
        }
    }
}

/// 配置目录：%APPDATA%/deepseek-pc（与原项目兼容）
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("deepseek-pc")
}

/// settings.json 完整路径
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.json")
}

/// 指定条目的脚本目录绝对路径
pub fn script_dir_path(dir_name: &str) -> PathBuf {
    config_dir().join("scripts").join(dir_name)
}



/// 读取设置，不存在时返回默认值
pub fn load() -> Settings {
    let path = settings_path();
    match fs::read_to_string(&path) {
        Ok(content) => {
            let mut s: Settings = serde_json::from_str(&content).unwrap_or_default();
            // initDefaults：空 entries 时创建默认条目（兼容旧版迁移）
            if s.entries.is_empty() {
                let hk_opt = s.hotkey.take()
                    .and_then(|h| Hotkey::from_str(&h).ok())
                        .or_else(|| Hotkey::from_str("Alt+E").ok());
                s.entries.push(SiteEntry {
                    id: "default".into(),
                    url: "https://chat.deepseek.com".into(),
                    name: "DeepSeek Chat".into(),
                    hotkey: hk_opt,
                    script_dir: "chat.deepseek.com".into(),
                });
                s.scripts_released = false;
            }
            s
        }
        Err(_) => Settings::default(),
    }
}

/// 保存设置到磁盘
pub fn save(data: &Settings) -> Result<(), String> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}
