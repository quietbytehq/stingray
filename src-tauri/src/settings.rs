use crate::error::{AppError, AppResult};
use crate::text;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// User preferences. Holds no credentials.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub theme: Theme,
    /// Number of transfers that run at the same time (1..=8).
    pub transfer_concurrency: u8,
    /// Advanced: use this rclone binary instead of the bundled one.
    pub rclone_path: Option<PathBuf>,
    pub rclone_log_level: LogLevel,
    /// Which intro plays at launch. The first launch after an install or an update always plays
    /// the full one unless the splash is off.
    pub splash: SplashMode,
    /// App version that last played the full intro. `None` until the first launch.
    pub last_splash_version: Option<String>,
    /// Register Stingray to start when the user signs in. It then starts minimized to the tray.
    pub start_on_login: bool,
    /// Icon set for files and folders: "stingray", "fluent", "cupertino" or "outline".
    pub icon_theme: String,
    /// Icon size in the icon view, in pixels.
    pub icon_size: u16,
    /// Show pictures as thumbnails instead of file icons.
    pub show_thumbnails: bool,
    /// Your own label and badge colour for each account, keyed by connection name.
    pub accounts: std::collections::BTreeMap<String, AccountLook>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SplashMode {
    Full,
    Short,
    Off,
}

/// What the splash window should play this launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SplashPlan {
    Full,
    Short,
    Off,
}

impl Settings {
    /// Decides this launch's intro. The full one plays when the setting says so, or on the first
    /// launch of a new version (first install or an update); otherwise the short one, or none.
    pub fn splash_plan(&self, app_version: &str) -> SplashPlan {
        match self.splash {
            SplashMode::Off => SplashPlan::Off,
            SplashMode::Full => SplashPlan::Full,
            SplashMode::Short if self.last_splash_version.as_deref() != Some(app_version) => {
                SplashPlan::Full
            }
            SplashMode::Short => SplashPlan::Short,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Error,
    Notice,
    Info,
    Debug,
}

impl LogLevel {
    pub fn as_rclone_arg(self) -> &'static str {
        match self {
            LogLevel::Error => "ERROR",
            LogLevel::Notice => "NOTICE",
            LogLevel::Info => "INFO",
            LogLevel::Debug => "DEBUG",
        }
    }
}

/// How an account is shown in the sidebar. Only a label and a colour; never anything secret.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountLook {
    pub label: String,
    /// A colour like "#8a3ffc".
    pub color: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            transfer_concurrency: 3,
            rclone_path: None,
            rclone_log_level: LogLevel::Info,
            splash: SplashMode::Short,
            last_splash_version: None,
            start_on_login: false,
            icon_theme: "stingray".into(),
            icon_size: 56,
            show_thumbnails: true,
            accounts: Default::default(),
        }
    }
}

impl Settings {
    pub const MAX_CONCURRENCY: u8 = 8;

    /// Returns an error if a value is out of range.
    pub const MIN_ICON: u16 = 32;
    pub const MAX_ICON: u16 = 128;

    pub fn validate(&self) -> AppResult<()> {
        if self.transfer_concurrency == 0 || self.transfer_concurrency > Self::MAX_CONCURRENCY {
            return Err(AppError::invalid(text::CONCURRENCY_RANGE));
        }
        if !(Self::MIN_ICON..=Self::MAX_ICON).contains(&self.icon_size) {
            return Err(AppError::invalid("Icon size must be between 32 and 128."));
        }
        if !["stingray", "fluent", "cupertino", "outline"].contains(&self.icon_theme.as_str()) {
            return Err(AppError::invalid("That icon theme isn't available."));
        }
        Ok(())
    }
}

/// The settings in use, shared with commands and the rest of the app.
pub struct SettingsStore(pub Mutex<Settings>);

impl SettingsStore {
    pub fn new(settings: Settings) -> Self {
        Self(Mutex::new(settings))
    }

    pub fn get(&self) -> Settings {
        self.0.lock().expect("settings lock").clone()
    }

    pub fn set(&self, settings: Settings) {
        *self.0.lock().expect("settings lock") = settings;
    }
}

/// Loads settings from `path`. A missing file yields the defaults. Files written by v0.1 carry
/// `splashEnabled` instead of `splash`; `false` becomes `off`, `true` the default.
pub fn load(path: &Path) -> AppResult<Settings> {
    match fs::read_to_string(path) {
        Ok(text) => parse(&text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(err) => Err(err.into()),
    }
}

fn parse(text: &str) -> AppResult<Settings> {
    let value: serde_json::Value = serde_json::from_str(text)?;
    let legacy_off = value.get("splash").is_none()
        && value.get("splashEnabled").and_then(|v| v.as_bool()) == Some(false);
    let mut settings: Settings = serde_json::from_value(value)?;
    if legacy_off {
        settings.splash = SplashMode::Off;
    }
    Ok(settings)
}

/// Writes settings atomically: a temp file is written, then renamed over `path`.
pub fn save(path: &Path, settings: &Settings) -> AppResult<()> {
    settings.validate()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec_pretty(settings)?)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_gives_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load(&dir.path().join("settings.json")).unwrap();
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        let settings = Settings {
            theme: Theme::Dark,
            transfer_concurrency: 5,
            rclone_path: Some(PathBuf::from("/opt/rclone")),
            rclone_log_level: LogLevel::Debug,
            splash: SplashMode::Off,
            last_splash_version: Some("0.2.0".into()),
            start_on_login: true,
            icon_theme: "outline".into(),
            icon_size: 96,
            show_thumbnails: false,
            accounts: [(
                "Work".to_owned(),
                AccountLook { label: "Office".into(), color: "#ff9f0a".into() },
            )]
            .into(),
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap(), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn rejects_bad_icon_settings() {
        let mut settings = Settings::default();
        settings.icon_size = 8;
        assert!(settings.validate().is_err());
        settings.icon_size = 64;
        settings.icon_theme = "neon".into();
        assert!(settings.validate().is_err());
        settings.icon_theme = "fluent".into();
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn rejects_out_of_range_concurrency() {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            transfer_concurrency: 0,
            ..Settings::default()
        };
        assert!(save(&dir.path().join("s.json"), &settings).is_err());
    }

    #[test]
    fn partial_file_fills_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"theme":"dark"}"#).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.theme, Theme::Dark);
        assert_eq!(loaded.transfer_concurrency, 3);
    }

    #[test]
    fn v0_1_splash_flag_is_migrated() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, r#"{"splashEnabled":false}"#).unwrap();
        assert_eq!(load(&path).unwrap().splash, SplashMode::Off);
        fs::write(&path, r#"{"splashEnabled":true}"#).unwrap();
        assert_eq!(load(&path).unwrap().splash, SplashMode::Short);
        // A file that already has the new field wins over the old flag.
        fs::write(&path, r#"{"splashEnabled":false,"splash":"full"}"#).unwrap();
        assert_eq!(load(&path).unwrap().splash, SplashMode::Full);
    }

    #[test]
    fn full_intro_plays_on_first_launch_and_after_an_update() {
        let mut settings = Settings::default();
        assert_eq!(
            settings.splash_plan("0.2.0"),
            SplashPlan::Full,
            "first launch"
        );
        settings.last_splash_version = Some("0.2.0".into());
        assert_eq!(
            settings.splash_plan("0.2.0"),
            SplashPlan::Short,
            "ordinary launch"
        );
        assert_eq!(
            settings.splash_plan("0.3.0"),
            SplashPlan::Full,
            "after an update"
        );
        settings.splash = SplashMode::Full;
        assert_eq!(settings.splash_plan("0.2.0"), SplashPlan::Full);
        settings.splash = SplashMode::Off;
        settings.last_splash_version = None;
        assert_eq!(
            settings.splash_plan("0.2.0"),
            SplashPlan::Off,
            "off stays off"
        );
    }
}
