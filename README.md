# [jfs](https://jfs.acdev.app/)

<p align="center">
  <img src="web/assets/logo.png" alt="jfs logo, a blue coffee cup with orange steam" width="160">
</p>

**Stop editing `JAVA_HOME`.**

jfs (Java Fucking Sucks) is a Windows Java version manager. Point it at JDKs you already have, then switch with one command. Your actual `JAVA_HOME` environment variable stays unchanged. Behind the scenes, a directory junction moves. No shims, no shell hooks, no background service.

```powershell
irm https://jfs.acdev.app/install.ps1 | iex
```

Git Bash:

```bash
curl -fsSL https://jfs.acdev.app/install.ps1 | powershell.exe -NoProfile -ExecutionPolicy Bypass -Command -
```

cmd:

```bat
powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://jfs.acdev.app/install.ps1 | iex"
```

Or run [install.bat](https://jfs.acdev.app/install.bat).

```powershell
PS C:\Users\you> java -version
openjdk version "25.0.4.1" 2026-08-18 LTS
OpenJDK Runtime Environment Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS)
OpenJDK 64-Bit Server VM Temurin-25.0.4.1+1 (build 25.0.4.1+1-LTS, mixed mode, sharing)

PS C:\Users\you> jfs list
  8  -> C:\Users\you\.gradle\jdks\temurin-8-amd64-windows\jdk8u442-b06
  21 -> C:\Users\you\.gradle\jdks\eclipse_adoptium-21-amd64-windows.2
* 25 -> C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot

PS C:\Users\you> jfs use 21
Now using Java 21

PS C:\Users\you> java -version
openjdk version "21.0.11" 2026-04-21 LTS
OpenJDK Runtime Environment Temurin-21.0.11+10 (build 21.0.11+10-LTS)
OpenJDK 64-Bit Server VM Temurin-21.0.11+10 (build 21.0.11+10-LTS, mixed mode, sharing)

PS C:\Users\you> jfs use 8
Now using Java 8

PS C:\Users\you> java -version
openjdk version "1.8.0_442"
OpenJDK Runtime Environment (Temurin)(build 1.8.0_442-b06)
OpenJDK 64-Bit Server VM (Temurin)(build 25.442-b06, mixed mode)
```

1. **Install once.** The script puts `jfs.exe` in `%LOCALAPPDATA%\jfs\bin`, sets your user `JAVA_HOME` to `%LOCALAPPDATA%\jfs\current`, and adds that `bin` plus `%JAVA_HOME%\bin` to your user PATH. Open a new terminal after installing; `jfs use` after that does not need another one.
2. **Register local JDKs.** jfs does not download Java. `jfs add 21 "C:\path\to\jdk"` checks for `bin\java.exe` and records the path. The first JDK you add becomes active.
3. **Switch.** `jfs use 21` retargets the `current` junction. New `java` processes follow it. Open terminals do not need a hook.

Windows searches the system `PATH` before the user `PATH`. If a system directory contains its own `java.exe`, that copy wins. The installer prints those directories when it finds them. An entry that is literally `%JAVA_HOME%\bin` is fine: it follows jfs.

## Commands

| Command | What it does |
| --- | --- |
| `jfs list` | Show registered JDKs. The active one is marked with `*`. |
| `jfs add <name> <path>` | Register a local JDK directory. |
| `jfs use <name>` | Point the current junction at that JDK. |
| `jfs current` | Print the active version name. |
| `jfs rm <name>` | Forget a version. Files on disk stay. The active version cannot be removed. |

## Where state lives

```text
%LOCALAPPDATA%\jfs\
  config.json     registry of names and paths
  current         NTFS junction to the active JDK
  bin\jfs.exe     the program
```

Set `JFS_DIR` to store that directory somewhere else. Junctions must target a local NTFS path, not a network share.

## Uninstall

`Remove-Item -Recurse` follows the `current` junction and can delete the JDK it points at. Remove the junction first:

```powershell
cmd /c rmdir "%LOCALAPPDATA%\jfs\current"
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\jfs"
```

Then delete the user `JAVA_HOME` value and the jfs entries from the user `PATH` (`HKCU\Environment`).

## Build from source

```powershell
cargo build --release
powershell -ExecutionPolicy Bypass -File web\install.ps1 -LocalExe target\release\jfs.exe
```

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Rust 1.85 or newer. Windows-only: switching is an NTFS junction (`mklink /J`), which does not need administrator rights or Developer Mode.

[![CI](https://github.com/qaptivator/jfs/actions/workflows/ci.yml/badge.svg)](https://github.com/qaptivator/jfs/actions/workflows/ci.yml)

## License

MIT. Copyright (c) 2026 acdev. See [LICENSE](LICENSE).
