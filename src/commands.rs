use std::path::Path;

use colored::Colorize;

use crate::config::{JfsConfig, Paths};
use crate::error::{Error, Result};
use crate::fs_ops;

pub fn list(paths: &Paths) -> Result<()> {
    let config = JfsConfig::load(paths)?;
    if config.versions.is_empty() {
        println!("No JDK versions registered. Use 'jfs add <name> <path>' to get started.");
        return Ok(());
    }

    if !config.current.is_empty() && !config.versions.contains_key(&config.current) {
        eprintln!(
            "{} active version '{}' is not registered",
            "warning:".yellow().bold(),
            config.current
        );
    } else if !config.current.is_empty() {
        warn_if_junction_drifted(paths, &config)?;
    }

    let width = config.versions.keys().map(String::len).max().unwrap_or(0);
    let mut entries: Vec<_> = config.versions.iter().collect();
    entries.sort_by(|(left, _), (right, _)| version_sort_key(left).cmp(&version_sort_key(right)));

    for (name, path) in entries {
        let active = name == &config.current;
        let marker = if active { "*" } else { " " };
        let line = format!("{marker} {name:<width$} -> {path}");
        if active {
            println!("{}", line.green());
        } else {
            println!("{line}");
        }
    }
    Ok(())
}

pub fn add(paths: &Paths, name: &str, path: &str) -> Result<()> {
    let name = validate_name(name)?;
    let jdk = fs_ops::normalize_jdk_path(Path::new(path))?;
    fs_ops::ensure_outside_jfs(&jdk, paths)?;

    let mut config = JfsConfig::load(paths)?;
    let first = config.versions.is_empty();
    let was_current = config.current == name;
    let overwritten = config.versions.contains_key(&name);
    let display = jdk.display().to_string();
    config.versions.insert(name.clone(), display.clone());

    if (first && config.current.is_empty()) || was_current {
        activate(paths, &mut config, &name)?;
    } else {
        config.save(paths)?;
    }

    if overwritten {
        println!("Updated {name} -> {display}");
    } else {
        println!("Added {name} -> {display}");
    }
    Ok(())
}

pub fn use_version(paths: &Paths, version: &str) -> Result<()> {
    let version = validate_name(version)?;
    let mut config = JfsConfig::load(paths)?;
    if !config.versions.contains_key(&version) {
        return Err(Error::new(format!(
            "version '{version}' is not registered. Run 'jfs list' to see available versions."
        )));
    }
    activate(paths, &mut config, &version)
}

pub fn remove_version(paths: &Paths, version: &str) -> Result<()> {
    let version = validate_name(version)?;
    let mut config = JfsConfig::load(paths)?;
    if !config.versions.contains_key(&version) {
        return Err(Error::new(format!(
            "version '{version}' is not registered. Run 'jfs list' to see available versions."
        )));
    }
    if config.current == version {
        return Err(Error::new(
            "Cannot remove active version. Switch to another version first.",
        ));
    }
    config.versions.remove(&version);
    config.save(paths)?;
    println!("Removed {version}");
    Ok(())
}

pub fn current(paths: &Paths) -> Result<()> {
    let config = JfsConfig::load(paths)?;
    if config.current.is_empty() {
        return Err(Error::new("No active Java version."));
    }
    println!("{}", config.current);
    Ok(())
}

fn activate(paths: &Paths, config: &mut JfsConfig, version: &str) -> Result<()> {
    let target = config
        .versions
        .get(version)
        .cloned()
        .ok_or_else(|| Error::new(format!("version '{version}' is not registered")))?;
    let java = Path::new(&target).join("bin").join("java.exe");
    if !java.is_file() {
        return Err(Error::new(format!(
            "version '{version}' points to '{target}' which is not a JDK (missing bin\\java.exe)"
        )));
    }

    let previous = config.current.clone();
    fs_ops::retarget_current(paths, Path::new(&target))?;
    config.current = version.to_string();
    if let Err(err) = config.save(paths) {
        restore_previous(paths, config, &previous);
        return Err(Error::new(format!(
            "the junction was updated, but config.json could not be saved ({err}). Run 'jfs use {version}' to retry."
        )));
    }
    println!("Now using Java {version}");
    Ok(())
}

fn restore_previous(paths: &Paths, config: &JfsConfig, previous: &str) {
    if previous.is_empty() {
        return;
    }
    let Some(path) = config.versions.get(previous) else {
        return;
    };
    let _ = fs_ops::retarget_current(paths, Path::new(path));
}

fn validate_name(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::new("version name cannot be empty"));
    }
    if name.len() > 128 {
        return Err(Error::new("version name must be 128 characters or fewer"));
    }
    if name.contains(['\\', '/', ':', '*', '?', '"', '<', '>', '|']) {
        return Err(Error::new(
            "version name cannot contain \\ / : * ? \" < > |",
        ));
    }
    if name.chars().any(char::is_control) {
        return Err(Error::new("version name cannot contain control characters"));
    }
    Ok(name.to_string())
}

fn version_sort_key(name: &str) -> (Vec<u64>, String) {
    let numbers = name
        .split(|ch: char| !ch.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u64>().unwrap_or(0))
        .collect();
    (numbers, name.to_ascii_lowercase())
}

fn warn_if_junction_drifted(paths: &Paths, config: &JfsConfig) -> Result<()> {
    let Some(expected) = config.versions.get(&config.current) else {
        return Ok(());
    };
    match fs_ops::junction_target(&paths.current_junction())? {
        Some(actual) if fs_ops::same_path(&actual, Path::new(expected)) => Ok(()),
        Some(actual) => {
            eprintln!(
                "{} junction points at {} but '{}' is registered as {}. Run 'jfs use {}' to repair it.",
                "warning:".yellow().bold(),
                actual.display(),
                config.current,
                expected,
                config.current
            );
            Ok(())
        }
        None => {
            eprintln!(
                "{} {} is not a junction. Run 'jfs use {}' to restore it.",
                "warning:".yellow().bold(),
                paths.current_junction().display(),
                config.current
            );
            Ok(())
        }
    }
}
