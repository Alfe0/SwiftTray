use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, io::{self, Write}, path::{Path, PathBuf}};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Shortcut {
    pub modifiers: u32,
    pub key: u32,
}
impl Shortcut {
    pub const fn new(modifiers: u32, key: u32) -> Self {
        Self { modifiers, key }
    }
    pub fn label(&self) -> String {
        let mut keys = Vec::new();
        if self.modifiers & 0x0008 != 0 { keys.push("Win".to_owned()); }
        if self.modifiers & 0x0002 != 0 { keys.push("Ctrl".to_owned()); }
        if self.modifiers & 0x0001 != 0 { keys.push("Alt".to_owned()); }
        if self.modifiers & 0x0004 != 0 { keys.push("Shift".to_owned()); }
        let key = if (0x41..=0x5a).contains(&self.key) || (0x30..=0x39).contains(&self.key) {
            char::from_u32(self.key).unwrap_or('?').to_string()
        } else if (0x70..=0x7b).contains(&self.key) {
            format!("F{}", self.key - 0x6f)
        } else {
            match self.key {
                0x0D => "Enter".to_owned(),
                0x09 => "Tab".to_owned(),
                0x20 => "Space".to_owned(),
                0x13 => "Pause".to_owned(),
                0x2C => "Print Screen".to_owned(),
                0xAD => "Volume Mute".to_owned(),
                0xAE => "Volume Down".to_owned(),
                0xAF => "Volume Up".to_owned(),
                0xB0 => "Next Track".to_owned(),
                0xB1 => "Previous Track".to_owned(),
                0xB2 => "Stop Media".to_owned(),
                0xB3 => "Play / Pause".to_owned(),
                0x5D => "Menu".to_owned(),
                0x2D => "Insert".to_owned(),
                0x2E => "Delete".to_owned(),
                0x24 => "Home".to_owned(),
                0x23 => "End".to_owned(),
                0x21 => "Page Up".to_owned(),
                0x22 => "Page Down".to_owned(),
                0x26 => "Up".to_owned(),
                0x28 => "Down".to_owned(),
                0x25 => "Left".to_owned(),
                0x27 => "Right".to_owned(),
                0x60..=0x69 => format!("Num {}",self.key-0x60),
                0x7C..=0x87 => format!("F{}", self.key-0x6F),
                _=>format!("0x{:X}", self.key),
            }
        };
        keys.push(key);
        keys.join(" + ")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    // Endpoint IDs are stored instead of transient device indexes.
    pub enabled_devices: HashMap<String, bool>,
    pub aliases: HashMap<String, String>,
    // One of: speaker, headphones, monitor, bluetooth.
    pub device_icons: HashMap<String, String>,
    pub popup: Shortcut,
    pub switch: Shortcut,
    pub snip: Shortcut,
    // false: SwiftTray mint-to-teal brand colors; true: Windows accent color.
    pub use_windows_accent: bool,
    pub startup_prompt_seen: bool,
    pub toast_enabled: bool,
    pub toast_top_right: bool, // Retained for compatibility with existing settings.
    pub toast_position: Option<u8>, // 0: top right, 1: top left, 2: above flyout.
}
impl Default for Config {
    fn default() -> Self {
        Self {
            enabled_devices: HashMap::new(),
            aliases: HashMap::new(),
            device_icons: HashMap::new(),
            // Two-key, easy-to-remember defaults. RegisterHotKey reports
            // conflicts; existing saved shortcuts always take precedence.
            popup: Shortcut::new(0x0001, b'M' as u32), // Alt + M
            switch: Shortcut::new(0x0001, b'O' as u32), // Alt + O
            snip: Shortcut::new(0x0001, b'S' as u32),   // Alt + S
            use_windows_accent: false,
            startup_prompt_seen: false,
            toast_enabled: true,
            toast_top_right: true,
            toast_position: None,
        }
    }
}
impl Config {
    pub fn file() -> PathBuf {
        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        root.join("SwiftTray").join("settings.json")
    }
    fn read_file(path: &Path) -> io::Result<Self> {
        let contents = fs::read(path)?;
        serde_json::from_slice(&contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    pub fn load() -> Self {
        let path = Self::file();
        // A power interruption or a previous release can leave damaged JSON.
        // Recover the last valid user configuration rather than resetting all
        // hotkeys, output aliases and preferences without warning.
        Self::read_file(&path)
            .or_else(|_| Self::read_file(&path.with_extension("json.bak")))
            .unwrap_or_default()
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::file())
    }

    fn save_to(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() { fs::create_dir_all(parent)?; }
        let bytes = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        let temporary = path.with_extension("json.tmp");
        let backup = path.with_extension("json.bak");

        // Flush the new value before replacing the existing file. An ordinary
        // rename replaces the destination on Windows without writing partial
        // JSON into the active settings file.
        let mut output = fs::File::create(&temporary)?;
        output.write_all(&bytes)?;
        output.sync_all()?;
        drop(output);

        // Do not overwrite a good backup with an already-corrupt active file.
        // The backup represents the preceding successfully parsed settings.
        if Self::read_file(path).is_ok() {
            fs::copy(path, &backup)?;
        }
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(())
    }
    pub fn popup_position(&self) -> u8 {
        self.toast_position.unwrap_or(if self.toast_top_right { 0 } else { 1 }).min(2)
    }
    pub fn device_enabled(&self, id: &str) -> bool {
        self.enabled_devices.get(id).copied().unwrap_or(true)
    }
    pub fn display_name<'a>(&'a self, id: &str, original: &'a str) -> &'a str {
        self.aliases.get(id).filter(|x| !x.trim().is_empty())
            .map(String::as_str).unwrap_or(original)
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::Config;

    #[test]
    fn older_settings_default_to_swifttray_colors() {
        let config:Config=serde_json::from_str(r#"{"toast_enabled":false}"#).unwrap();
        assert!(!config.use_windows_accent);
        assert!(!config.toast_enabled);
    }

    #[test]
    fn windows_color_preference_round_trips() {
        let mut config=Config::default();
        config.use_windows_accent=true;
        let json=serde_json::to_string(&config).unwrap();
        let restored:Config=serde_json::from_str(&json).unwrap();
        assert!(restored.use_windows_accent);
    }
}

#[cfg(test)]
mod recovery_tests {
    use super::Config;
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn restores_valid_backup_instead_of_resetting_preferences() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let directory = std::env::temp_dir().join(format!("swifttray-settings-test-{}-{timestamp}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let settings = directory.join("settings.json");
        let mut chosen = Config::default();
        chosen.use_windows_accent = true;
        chosen.toast_enabled = false;
        chosen.save_to(&settings).unwrap();
        chosen.toast_enabled = true;
        chosen.save_to(&settings).unwrap();
        fs::write(&settings, b"{incomplete JSON").unwrap();
        let recovered = Config::read_file(&settings)
            .or_else(|_| Config::read_file(&settings.with_extension("json.bak")))
            .unwrap();
        assert!(recovered.use_windows_accent);
        assert!(!recovered.toast_enabled);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn saving_settings_never_leaves_a_temporary_file() {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let directory = std::env::temp_dir().join(format!("swifttray-save-test-{}-{timestamp}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("settings.json");
        Config::default().save_to(&path).unwrap();
        assert!(path.is_file());
        assert!(!path.with_extension("json.tmp").exists());
        fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
mod default_hotkeys_tests {
    use super::{Config, Shortcut};

    #[test]
    fn first_run_uses_two_key_defaults() {
        let defaults=Config::default();
        assert_eq!(defaults.popup,Shortcut::new(0x0001,b'M' as u32));
        assert_eq!(defaults.switch,Shortcut::new(0x0001,b'O' as u32));
        assert_eq!(defaults.snip,Shortcut::new(0x0001,b'S' as u32));
        assert_eq!(defaults.popup.label(),"Alt + M");
    }

    #[test]
    fn existing_custom_shortcuts_survive_deserialization() {
        let saved=r#"{"popup":{"modifiers":0,"key":34},"switch":{"modifiers":0,"key":35},"snip":{"modifiers":0,"key":33}}"#;
        let actual:Config=serde_json::from_str(saved).unwrap();
        assert_eq!(actual.popup,Shortcut::new(0,34));
        assert_eq!(actual.switch,Shortcut::new(0,35));
        assert_eq!(actual.snip,Shortcut::new(0,33));
    }
}
