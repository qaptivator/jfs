use std::fs;
use std::io;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::config::Paths;
use crate::error::{Error, Result};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;

/// What currently occupies a path we may replace.
enum Slot {
    Absent,
    /// NTFS junction or another reparse point. Safe to remove with `remove_dir`.
    Link,
    EmptyDir,
    /// A real directory that contains files. Never delete this.
    RealDir,
    Other,
}

/// Point `%LOCALAPPDATA%\jfs\current` at `target`.
///
/// The published name is swapped by rename: the new junction is created beside
/// `current`, the old link is moved aside, then the new link takes its place.
/// `remove_dir` deletes a junction entry and does not follow it into the JDK.
pub fn retarget_current(paths: &Paths, target: &Path) -> Result<()> {
    if is_unc(target) {
        return Err(Error::new(
            "junctions cannot target a network path. Register a JDK on a local NTFS disk.",
        ));
    }
    if !target.is_dir() {
        return Err(Error::new(format!(
            "JDK directory does not exist: {}",
            target.display()
        )));
    }

    fs::create_dir_all(&paths.root)?;
    let link = paths.current_junction();
    let staging = paths.root.join("current.new");
    let backup = paths.root.join("current.bak");

    discard_slot(&staging)?;
    discard_slot(&backup)?;

    if let Err(err) = junction::create(target, &staging) {
        let _ = fs::remove_dir(&staging);
        return Err(Error::new(format!(
            "failed to create junction for {}: {err}",
            target.display()
        )));
    }

    match classify(&link)? {
        Slot::Absent => {}
        Slot::Link | Slot::EmptyDir => {
            if let Err(err) = fs::rename(&link, &backup) {
                let _ = discard_slot(&staging);
                return Err(Error::new(format!(
                    "failed to move aside {}: {err}",
                    link.display()
                )));
            }
        }
        Slot::RealDir => {
            let _ = discard_slot(&staging);
            return Err(Error::new(format!(
                "{} is a real directory with files in it. jfs will not delete it. Move it aside, then retry.",
                link.display()
            )));
        }
        Slot::Other => {
            let _ = discard_slot(&staging);
            return Err(Error::new(format!(
                "{} exists and is not a directory junction. Move it aside, then retry.",
                link.display()
            )));
        }
    }

    if let Err(err) = fs::rename(&staging, &link) {
        if matches!(classify(&backup), Ok(Slot::Link | Slot::EmptyDir)) {
            let _ = fs::rename(&backup, &link);
        }
        let _ = discard_slot(&staging);
        return Err(Error::new(format!(
            "failed to publish {}: {err}",
            link.display()
        )));
    }

    discard_slot(&backup)?;
    Ok(())
}

/// Target of the `current` junction, if it is a junction.
pub fn junction_target(path: &Path) -> Result<Option<PathBuf>> {
    match classify(path)? {
        Slot::Link => {
            let target = junction::get_target(path)?;
            Ok(Some(strip_verbatim(target)))
        }
        _ => Ok(None),
    }
}

pub fn same_path(left: &Path, right: &Path) -> bool {
    fn norm(path: &Path) -> String {
        path.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    }
    norm(left) == norm(right)
}

/// Canonical absolute JDK path, with the Win32 `\\?\` prefix removed.
pub fn normalize_jdk_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        return Err(Error::new("path cannot be empty"));
    }
    if !path.exists() {
        return Err(Error::new(format!(
            "path does not exist: {}",
            path.display()
        )));
    }

    let canonical = fs::canonicalize(path)?;
    let stripped = strip_verbatim(canonical);
    if is_unc(&stripped) {
        return Err(Error::new(
            "junctions cannot target a network path. Register a JDK on a local NTFS disk.",
        ));
    }

    let java = stripped.join("bin").join("java.exe");
    if !java.is_file() {
        return Err(Error::new(format!(
            "not a JDK: {} is missing bin\\java.exe",
            stripped.display()
        )));
    }
    Ok(stripped)
}

pub fn ensure_outside_jfs(jdk: &Path, paths: &Paths) -> Result<()> {
    let jdk_text = jdk.to_string_lossy().to_ascii_lowercase();
    let root_text = paths
        .root
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    if jdk_text == root_text || jdk_text.starts_with(&(root_text + "\\")) {
        return Err(Error::new(
            "that path is inside the jfs data directory. Register the real JDK directory, not the current junction.",
        ));
    }
    Ok(())
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\??\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    if let Some(rest) = text.strip_prefix(r"\??\") {
        return PathBuf::from(rest);
    }
    path
}

fn is_unc(path: &Path) -> bool {
    path.to_string_lossy().starts_with(r"\\")
}

fn is_reparse(meta: &fs::Metadata) -> bool {
    meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

fn classify(path: &Path) -> io::Result<Slot> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Slot::Absent),
        Err(err) => return Err(err),
    };
    if is_reparse(&meta) {
        return Ok(Slot::Link);
    }
    if meta.is_dir() {
        let mut entries = fs::read_dir(path)?;
        return if entries.next().is_none() {
            Ok(Slot::EmptyDir)
        } else {
            Ok(Slot::RealDir)
        };
    }
    Ok(Slot::Other)
}

fn discard_slot(path: &Path) -> Result<()> {
    match classify(path)? {
        Slot::Absent => Ok(()),
        Slot::Link | Slot::EmptyDir => fs::remove_dir(path).map_err(Error::from),
        Slot::RealDir => Err(Error::new(format!(
            "{} is a real directory with files in it. jfs will not delete it.",
            path.display()
        ))),
        Slot::Other => Err(Error::new(format!(
            "{} exists and is not a directory junction. jfs will not delete it.",
            path.display()
        ))),
    }
}
