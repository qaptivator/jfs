use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Locations under the jfs data directory.
///
/// Override the directory with `JFS_DIR`. Otherwise it is `%LOCALAPPDATA%\jfs`.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn resolve() -> Result<Self> {
        if let Some(dir) = env::var_os("JFS_DIR") {
            if !dir.is_empty() {
                let path = PathBuf::from(dir);
                let root = if path.is_absolute() {
                    path
                } else {
                    env::current_dir()?.join(path)
                };
                return Ok(Self { root });
            }
        }

        let local = env::var_os("LOCALAPPDATA").ok_or_else(|| {
            Error::new("LOCALAPPDATA is not set. jfs runs in a Windows user session.")
        })?;
        Ok(Self {
            root: PathBuf::from(local).join("jfs"),
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn current_junction(&self) -> PathBuf {
        self.root.join("current")
    }
}

/// Registry of local JDK directories.
///
/// `current` is the active version name. `versions` maps a name to an absolute JDK path.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct JfsConfig {
    #[serde(default)]
    pub current: String,
    #[serde(default)]
    pub versions: BTreeMap<String, String>,
}

impl JfsConfig {
    pub fn load(paths: &Paths) -> Result<Self> {
        let path = paths.config_file();
        match fs::read_to_string(&path) {
            Ok(raw) => {
                let raw = raw.trim_start_matches('\u{feff}');
                if raw.trim().is_empty() {
                    return Ok(Self::default());
                }
                Ok(serde_json::from_str(raw)?)
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err.into()),
        }
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        fs::create_dir_all(&paths.root)?;
        let data = serde_json::to_string_pretty(self)?;
        let dest = paths.config_file();
        let tmp = paths.root.join("config.json.tmp");
        fs::write(&tmp, format!("{data}\n"))?;
        replace_file(&tmp, &dest).map_err(|err| {
            let _ = fs::remove_file(&tmp);
            Error::new(format!("failed to save {}: {err}", dest.display()))
        })?;
        Ok(())
    }
}

/// Replace `to` with `from`, including when `to` already exists.
fn replace_file(from: &Path, to: &Path) -> io::Result<()> {
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
    }

    fn wide(path: &Path) -> Vec<u16> {
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }

    let from_wide = wide(from);
    let to_wide = wide(to);
    // SAFETY: both pointers refer to NUL-terminated UTF-16 strings that live
    // for the duration of the call.
    let ok = unsafe {
        MoveFileExW(
            from_wide.as_ptr(),
            to_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn roundtrip_config() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("jfs-config-{nanos}"));
        let paths = Paths { root: root.clone() };
        let mut config = JfsConfig {
            current: "21".to_string(),
            ..JfsConfig::default()
        };
        config
            .versions
            .insert("21".to_string(), r"C:\jdks\21".to_string());
        config
            .versions
            .insert("8".to_string(), r"C:\jdks\8".to_string());
        config.save(&paths).unwrap();

        let loaded = JfsConfig::load(&paths).unwrap();
        assert_eq!(loaded, config);

        let text = fs::read_to_string(paths.config_file()).unwrap();
        assert!(text.ends_with('\n'));
        assert!(text.contains("\"current\": \"21\""));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn missing_config_is_empty() {
        let paths = Paths {
            root: env::temp_dir().join("jfs-config-missing-does-not-exist"),
        };
        let config = JfsConfig::load(&paths).unwrap();
        assert!(config.current.is_empty());
        assert!(config.versions.is_empty());
    }
}
