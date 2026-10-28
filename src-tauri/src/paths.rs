use std::path::{Path, PathBuf};

/// Where Stingray keeps its files. All paths sit under the app data directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    pub data_dir: PathBuf,
}

impl AppPaths {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
        }
    }

    /// rclone's own config. This is the only place credentials are stored.
    pub fn rclone_config(&self) -> PathBuf {
        self.data_dir.join("rclone.conf")
    }

    pub fn settings(&self) -> PathBuf {
        self.data_dir.join("settings.json")
    }

    /// Saved NFS shares (server, export, and the folder each one is connected at). No secrets.
    pub fn nfs_shares(&self) -> PathBuf {
        self.data_dir.join("nfs.json")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    pub fn rclone_log(&self) -> PathBuf {
        self.logs_dir().join("rclone.log")
    }
}

/// Directory that holds the Stingray executable, used to find the bundled rclone.
pub fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_is_under_data_dir() {
        let paths = AppPaths::new("/data/stingray");
        assert_eq!(
            paths.rclone_config(),
            PathBuf::from("/data/stingray/rclone.conf")
        );
        assert_eq!(
            paths.rclone_log(),
            PathBuf::from("/data/stingray/logs/rclone.log")
        );
    }
}
