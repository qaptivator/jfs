# [jfs](https://jfs.acdev.app/)

<p align="center">
  <img src="web/assets/logo.png" alt="jfs logo, a blue coffee cup with orange steam" width="160">
</p>

**Windows Java version manager for Java fans.**

jfs (Java Fucking Sucks) is a Windows Java version manager. Point it at JDKs you already have, then switch with one command. Your `JAVA_HOME` never changes. Behind the scenes, a directory junction moves. No shims, no shell hooks, no background service.

```powershell
# install
irm https://jfs.acdev.app/install.ps1 | iex
# then
jfs add 21 "C:\path\to\jdk-21"
jfs use 21
```

Git Bash and cmd install one-liners are on the [landing page](https://jfs.acdev.app/).

## Example

Register a few JDKs once, then switch in the same terminal. `java -version` follows immediately; `JAVA_HOME` stays `%LOCALAPPDATA%\jfs\current`.

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

## How it works

`JAVA_HOME` points at `%LOCALAPPDATA%\jfs\current`, which is an NTFS directory junction. `jfs use` retargets that junction, so every new `java` process follows the JDK you picked. Terminals that are already open don't need a restart.

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

Host the `web/` directory at `jfs.acdev.app`. It is the landing page, `install.ps1`, `install.bat`, `favicon.ico`, `assets/logo.png`, `sitemap.xml`, `robots.txt`, and `site.webmanifest`.

## License

MIT. Copyright (c) 2026 acdev. See [LICENSE](LICENSE).
