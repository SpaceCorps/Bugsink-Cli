//! Non-secret account metadata in a readable YAML file. The API token itself never lands here -
//! that goes to [`crate::secrets`].

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct AccountConfig {
    /// Base URL of the Bugsink instance (e.g. https://bugsink.example.com)
    pub endpoint: String,
    /// Whatever identity or description was given when added
    pub identity: String,
    pub added_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub version: u32,
    pub accounts: IndexMap<String, AccountConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Config { version: 1, accounts: IndexMap::new() }
    }
}

impl Config {
    /// Account lookup is case-insensitive; returns the name as it is stored.
    pub fn find(&self, name: &str) -> Option<(&String, &AccountConfig)> {
        self.accounts.iter().find(|(k, _)| k.eq_ignore_ascii_case(name))
    }

    pub fn sorted(&self) -> Vec<(&String, &AccountConfig)> {
        let mut v: Vec<_> = self.accounts.iter().collect();
        v.sort_by_key(|(k, _)| k.to_lowercase());
        v
    }
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("BUGSINK_CONFIG_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }

    #[cfg(windows)]
    let dir = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default().join("bugsink-cli");

    #[cfg(not(windows))]
    let dir = {
        let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
        if cfg!(target_os = "macos") {
            home.join("Library").join("Application Support").join("bugsink-cli")
        } else {
            std::env::var_os("XDG_CONFIG_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"))
                .join("bugsink-cli")
        }
    };

    dir
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.yaml")
}

pub fn ensure_dir() -> Result<()> {
    let dir = config_dir();
    if dir.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(&dir)?;
    restrict_to_owner(&dir);
    Ok(())
}

pub fn load() -> Result<Config> {
    let path = config_path();
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(e) => return Err(e.into()),
    };
    if text.trim().is_empty() {
        return Ok(Config::default());
    }
    serde_norway::from_str::<Option<Config>>(&text).map(Option::unwrap_or_default).map_err(|e| {
        Error::other(format!("{} is not valid.", path.display()))
            .detail(e.to_string())
            .fix(format!("Fix or delete {}, then run: bugsink accounts add <name>", path.display()))
    })
}

pub fn save(config: &Config) -> Result<()> {
    ensure_dir()?;
    let yaml = serde_norway::to_string(config).map_err(|e| Error::other(e.to_string()))?;
    atomic_write(&config_path(), yaml.as_bytes())
}

/// Write to a temp file in the same directory, then rename over - never a partial file.
pub fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    fs::write(&tmp, contents)?;
    restrict_to_owner(&tmp);
    fs::rename(&tmp, path)?;
    Ok(())
}

/// 0600 (0700 for directories) on Unix. On Windows the DPAPI blob is already user-scoped.
pub fn restrict_to_owner(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = if path.is_dir() { 0o700 } else { 0o600 };
        // Best effort - a filesystem that cannot express permissions is not a reason to fail.
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
    }
    #[cfg(not(unix))]
    let _ = path;
}

/// Cross-process exclusive lock, held while config and secrets are written together.
pub struct Lock(#[allow(dead_code)] File);

pub fn lock() -> Result<Lock> {
    ensure_dir()?;
    let path = config_dir().join(".lock");
    let file = OpenOptions::new().create(true).truncate(false).read(true).write(true).open(&path)?;

    let deadline = Instant::now() + Duration::from_secs(15);
    let mut delay = Duration::from_millis(25);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Lock(file)),
            Err(fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                std::thread::sleep(delay);
                delay = (delay * 2).min(Duration::from_millis(400));
            }
            Err(fs::TryLockError::WouldBlock) => {
                return Err(Error::other(format!(
                    "Timed out waiting for the lock at {}. Another bugsink process may be stuck.",
                    path.display()
                )));
            }
            Err(fs::TryLockError::Error(e)) => return Err(e.into()),
        }
    }
}

/// `yyyy-MM-ddTHH:mm:ssZ`, UTC.
pub fn now_utc() -> String {
    let secs =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    format_utc(secs)
}

fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(1_790_000_000), "2026-09-21T14:13:20Z");
    }

    #[test]
    fn loads_empty_config() {
        let c = Config::default();
        assert_eq!(c.version, 1);
        assert!(c.accounts.is_empty());
    }
}
