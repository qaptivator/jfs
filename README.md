# jfs

Java Fucking Sucks. A `JAVA_HOME` manager for Java fans on Windows.

`JAVA_HOME` stays pointed at `%LOCALAPPDATA%\jfs\current`. That path is an NTFS directory junction. `jfs use` retargets the junction, so every new `java` process follows the JDK you picked. There is no shim, no shell hook, and no background service.

jfs does not download JDKs. You register directories that are already on disk.

[![CI](https://github.com/qaptivator/jfs/actions/workflows/ci.yml/badge.svg)](https://github.com/qaptivator/jfs/actions/workflows/ci.yml)

## Install

In PowerShell:

```powershell
irm https://jfs.acdev.app/install.ps1 | iex
```

Or download [install.bat](https://jfs.acdev.app/install.bat) and run it. The batch file fetches the PowerShell installer.

The installer:

1. Downloads the latest `jfs.exe` from [GitHub releases](https://github.com/qaptivator/jfs/releases) into `%LOCALAPPDATA%\jfs\bin`.
2. Sets the user `JAVA_HOME` to `%LOCALAPPDATA%\jfs\current`.
3. Adds `%LOCALAPPDATA%\jfs\bin` and `%JAVA_HOME%\bin` to the user `PATH` if they are missing.

Open a new terminal after installing. `jfs use` after that does not require another new terminal.

Windows searches the system `PATH` before the user `PATH`. If a system directory contains its own `java.exe`, that copy wins. The installer prints those directories when it finds them. An entry that is literally `%JAVA_HOME%\bin` is fine: it follows jfs.

### Build from source

```powershell
cargo build --release
powershell -ExecutionPolicy Bypass -File web\install.ps1 -LocalExe target\release\jfs.exe
```

The release workflow builds `jfs-x86_64.exe` on `windows-latest` and `jfs-aarch64.exe` on `windows-11-arm` when that runner exists. Tag `v0.1.0` to publish them.

## Usage

```powershell
jfs add 8 "C:\Users\ac\.gradle\jdks\temurin-8-amd64-windows\jdk8u442-b06"
jfs add 21 "C:\Users\ac\.gradle\jdks\eclipse_adoptium-21-amd64-windows.2"
jfs add 25 "C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot"
jfs list
jfs use 21
java -version
jfs current
jfs rm 8
```

`jfs add` requires `bin\java.exe` inside the directory. The first JDK you add becomes active. Adding a JDK that is already active retargets the junction. `jfs rm` only edits `config.json`. It refuses to remove the active version.

```text
%LOCALAPPDATA%\jfs\
  config.json
  current          junction
  bin\jfs.exe
```

`config.json`:

```json
{
  "current": "25",
  "versions": {
    "8": "C:\\Users\\ac\\.gradle\\jdks\\temurin-8-amd64-windows\\jdk8u442-b06",
    "21": "C:\\Users\\ac\\.gradle\\jdks\\eclipse_adoptium-21-amd64-windows.2",
    "25": "C:\\Program Files\\Eclipse Adoptium\\jdk-25.0.4.101-hotspot"
  }
}
```

Set `JFS_DIR` to store that directory somewhere else. Junctions must target a local NTFS path, not a network share.

## Uninstall

`Remove-Item -Recurse` follows the `current` junction and can delete the JDK it points at. Remove the junction first:

```powershell
cmd /c rmdir "%LOCALAPPDATA%\jfs\current"
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\jfs"
```

Then delete the user `JAVA_HOME` value and the jfs entries from the user `PATH` (`HKCU\Environment`).

## Contributing

Repository: https://github.com/qaptivator/jfs

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Rust 1.85 or newer. Pull requests against `main`. The tool is Windows-only because version switching is an NTFS junction (`mklink /J`), which does not need administrator rights or Developer Mode.

Host the `web/` directory at `jfs.acdev.app`. It is the landing page plus `install.ps1` and `install.bat`.

## License

MIT. Copyright (c) 2026 acdev. See [LICENSE](LICENSE).
