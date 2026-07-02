use std::collections::{HashMap, HashSet};
use std::fmt;
use std::str::FromStr;

use serde::Deserialize;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};

use crate::settings::Settings;

// ── Errors ──

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotkeyParseError {
    TooFewParts,
    UnknownModifier(String),
    UnknownKey(String),
}

impl fmt::Display for HotkeyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewParts => write!(f, "热键格式无效，需要修饰键+按键"),
            Self::UnknownModifier(m) => write!(f, "未知修饰键: {}", m),
            Self::UnknownKey(k) => write!(f, "未知按键: {}", k),
        }
    }
}

impl std::error::Error for HotkeyParseError {}

// ── Hotkey newtype ──

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Hotkey {
    pub mods: Modifiers,
    pub code: Code,
}

impl Hotkey {
    pub fn shortcut(&self) -> Shortcut {
        Shortcut::new(Some(self.mods), self.code)
    }
}

impl FromStr for Hotkey {
    type Err = HotkeyParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('+').collect();
        if parts.len() < 2 {
            return Err(HotkeyParseError::TooFewParts);
        }
        let mut mods = Modifiers::empty();
        for p in &parts[..parts.len() - 1] {
            match p.to_lowercase().as_str() {
                "ctrl" | "control" => mods |= Modifiers::CONTROL,
                "alt" => mods |= Modifiers::ALT,
                "shift" => mods |= Modifiers::SHIFT,
                "win" | "windows" | "cmd" | "super" => mods |= Modifiers::SUPER,
                other => return Err(HotkeyParseError::UnknownModifier(other.into())),
            }
        }
        let key_str = parts.last().unwrap().to_uppercase();
        let code = parse_code(&key_str)?;
        Ok(Self { mods, code })
    }
}

fn parse_code(s: &str) -> Result<Code, HotkeyParseError> {
    match s {
        "A" => Ok(Code::KeyA), "B" => Ok(Code::KeyB), "C" => Ok(Code::KeyC),
        "D" => Ok(Code::KeyD), "E" => Ok(Code::KeyE), "F" => Ok(Code::KeyF),
        "G" => Ok(Code::KeyG), "H" => Ok(Code::KeyH), "I" => Ok(Code::KeyI),
        "J" => Ok(Code::KeyJ), "K" => Ok(Code::KeyK), "L" => Ok(Code::KeyL),
        "M" => Ok(Code::KeyM), "N" => Ok(Code::KeyN), "O" => Ok(Code::KeyO),
        "P" => Ok(Code::KeyP), "Q" => Ok(Code::KeyQ), "R" => Ok(Code::KeyR),
        "S" => Ok(Code::KeyS), "T" => Ok(Code::KeyT), "U" => Ok(Code::KeyU),
        "V" => Ok(Code::KeyV), "W" => Ok(Code::KeyW), "X" => Ok(Code::KeyX),
        "Y" => Ok(Code::KeyY), "Z" => Ok(Code::KeyZ),
        "0" => Ok(Code::Digit0), "1" => Ok(Code::Digit1), "2" => Ok(Code::Digit2),
        "3" => Ok(Code::Digit3), "4" => Ok(Code::Digit4), "5" => Ok(Code::Digit5),
        "6" => Ok(Code::Digit6), "7" => Ok(Code::Digit7), "8" => Ok(Code::Digit8),
        "9" => Ok(Code::Digit9),
        "F1" => Ok(Code::F1), "F2" => Ok(Code::F2), "F3" => Ok(Code::F3),
        "F4" => Ok(Code::F4), "F5" => Ok(Code::F5), "F6" => Ok(Code::F6),
        "F7" => Ok(Code::F7), "F8" => Ok(Code::F8), "F9" => Ok(Code::F9),
        "F10" => Ok(Code::F10), "F11" => Ok(Code::F11), "F12" => Ok(Code::F12),
        other => Err(HotkeyParseError::UnknownKey(other.into())),
    }
}

impl fmt::Display for Hotkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.mods.contains(Modifiers::CONTROL) {
            write!(f, "Ctrl+")?;
        }
        if self.mods.contains(Modifiers::ALT) {
            write!(f, "Alt+")?;
        }
        if self.mods.contains(Modifiers::SHIFT) {
            write!(f, "Shift+")?;
        }
        if self.mods.contains(Modifiers::SUPER) {
            write!(f, "Win+")?;
        }
        write!(f, "{}", code_to_str(&self.code))
    }
}

fn code_to_str(code: &Code) -> &str {
    match code {
        Code::KeyA => "A", Code::KeyB => "B", Code::KeyC => "C",
        Code::KeyD => "D", Code::KeyE => "E", Code::KeyF => "F",
        Code::KeyG => "G", Code::KeyH => "H", Code::KeyI => "I",
        Code::KeyJ => "J", Code::KeyK => "K", Code::KeyL => "L",
        Code::KeyM => "M", Code::KeyN => "N", Code::KeyO => "O",
        Code::KeyP => "P", Code::KeyQ => "Q", Code::KeyR => "R",
        Code::KeyS => "S", Code::KeyT => "T", Code::KeyU => "U",
        Code::KeyV => "V", Code::KeyW => "W", Code::KeyX => "X",
        Code::KeyY => "Y", Code::KeyZ => "Z",
        Code::Digit0 => "0", Code::Digit1 => "1", Code::Digit2 => "2",
        Code::Digit3 => "3", Code::Digit4 => "4", Code::Digit5 => "5",
        Code::Digit6 => "6", Code::Digit7 => "7", Code::Digit8 => "8",
        Code::Digit9 => "9",
        Code::F1 => "F1", Code::F2 => "F2", Code::F3 => "F3",
        Code::F4 => "F4", Code::F5 => "F5", Code::F6 => "F6",
        Code::F7 => "F7", Code::F8 => "F8", Code::F9 => "F9",
        Code::F10 => "F10", Code::F11 => "F11", Code::F12 => "F12",
        _ => "?",
    }
}

// ── Serde: Hotkey ↔ "Alt+E" ──

impl serde::Serialize for Hotkey {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

impl<'de> serde::Deserialize<'de> for Hotkey {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = <&str as Deserialize>::deserialize(d)?;
        Hotkey::from_str(s).map_err(serde::de::Error::custom)
    }
}

// ── Diff sync outcome ──

#[derive(Debug, Default)]
pub struct SyncOutcome {
    pub added: Vec<(String, String)>,
    pub removed: Vec<String>,
    pub failures: Vec<(String, String)>,
}

/// Diff-based 热键同步：只注销 removed、注册 added，保留交集不变。
pub fn sync_hotkeys(
    app: tauri::AppHandle,
    hotkey_map: &mut HashMap<String, String>,
    new: &Settings,
) -> SyncOutcome {
    let shortcut_manager = app.global_shortcut();

    let old_map: HashMap<String, String> = hotkey_map.clone();

    let new_map: HashMap<String, String> = new
        .entries
        .iter()
        .filter(|e| e.enabled)
        .filter_map(|e| e.hotkey.as_ref().map(|hk| (hk.to_string(), e.id.clone())))
        .collect();

    let old_keys: HashSet<&str> = old_map.keys().map(String::as_str).collect();
    let new_keys: HashSet<&str> = new_map.keys().map(String::as_str).collect();

    let mut outcome = SyncOutcome::default();

    // 注销差异（存在旧但不在新）
    for hk in old_keys.difference(&new_keys) {
        let hk = hk.to_string();
        if let Ok(k) = Hotkey::from_str(&hk) {
            if let Err(e) = shortcut_manager.unregister(k.shortcut()) {
                log::warn!("unregister failed for {}: {}", hk, e);
            } else {
                log::info!("hotkey unregistered: {}", hk);
                outcome.removed.push(hk.clone());
            }
        }
        hotkey_map.remove(&hk);
    }

    // 注册差异（存在新但不在旧）
    for hk in new_keys.difference(&old_keys) {
        let hk = hk.to_string();
        match Hotkey::from_str(&hk) {
            Ok(k) => {
                if let Err(e) = shortcut_manager.register(k.shortcut()) {
                    log::warn!("hotkey register failed: {} ({})", hk, e);
                    outcome.failures.push((hk, e.to_string()));
                } else {
                    let id = new_map.get(&hk).cloned().unwrap_or_default();
                    hotkey_map.insert(hk.clone(), id.clone());
                    log::info!("hotkey registered: {}", hk);
                    outcome.added.push((hk, id));
                }
            }
            Err(e) => {
                log::warn!("invalid hotkey {}: {}", hk, e);
                outcome.failures.push((hk, e.to_string()));
            }
        }
    }

    // 交集区域：更新 entry_id（url/name 可能变了）
    for hk in old_keys.intersection(&new_keys) {
        let hk = hk.to_string();
        if let Some(id) = new_map.get(&hk) {
            hotkey_map.insert(hk, id.clone());
        }
    }

    outcome
}

/// 检查某一个 Hotkey 是否可用（冲突检测 + 临时注册验证）
pub fn check_hotkey(
    app: tauri::AppHandle,
    hotkey_map: &HashMap<String, String>,
    hotkey: &Hotkey,
    editing_id: Option<&str>,
) -> Result<(), String> {
    let hk_str = hotkey.to_string();

    if let Some(entry_id) = hotkey_map.get(&hk_str) {
        return if editing_id == Some(entry_id.as_str()) {
            Ok(())
        } else {
            Err("已被其他站点占用".into())
        };
    }

    let shortcut_manager = app.global_shortcut();
    if let Err(e) = shortcut_manager.register(hotkey.shortcut()) {
        return Err(format!("无法注册: {}", e));
    }
    let _ = shortcut_manager.unregister(hotkey.shortcut());
    Ok(())
}
