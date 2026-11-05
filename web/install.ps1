# jfs installer for Windows.
# https://jfs.acdev.app/install.ps1
#Requires -Version 5.1
param(
    [string]$LocalExe,
    [string]$Repo = "qaptivator/jfs"
)

$ErrorActionPreference = "Stop"

function Get-CpuArch {
    $arch = $env:PROCESSOR_ARCHITECTURE
    if ($arch -eq "x86" -and $env:PROCESSOR_ARCHITEW6432) {
        $arch = $env:PROCESSOR_ARCHITEW6432
    }
    switch ($arch) {
        "AMD64" { "jfs-x86_64.exe" }
        "ARM64" { "jfs-aarch64.exe" }
        default { throw "jfs supports 64-bit x64 and ARM64 Windows. This process is $arch." }
    }
}

function Get-UserPathState {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $false)
    if (-not $key) {
        return @{ Kind = "ExpandString"; Value = ""; Name = "Path" }
    }
    try {
        $pathName = @($key.GetValueNames()) | Where-Object { $_ -ieq "Path" } | Select-Object -First 1
        if (-not $pathName) {
            return @{ Kind = "ExpandString"; Value = ""; Name = "Path" }
        }
        $kind = $key.GetValueKind($pathName)
        $value = $key.GetValue($pathName, "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $kindName = if ($kind -eq [Microsoft.Win32.RegistryValueKind]::ExpandString) { "ExpandString" } else { "String" }
        return @{ Kind = $kindName; Value = [string]$value; Name = [string]$pathName }
    } finally {
        $key.Close()
    }
}

function Add-PathEntry([string]$Current, [string]$Entry) {
    foreach ($item in ($Current -split ";")) {
        if ($item.TrimEnd("\") -ieq $Entry.TrimEnd("\")) {
            return $Current
        }
    }
    if ([string]::IsNullOrEmpty($Current)) { return $Entry }
    return "$Entry;$Current"
}

function Publish-EnvironmentChange {
    if (-not ("Jfs.Native" -as [type])) {
        Add-Type -Namespace Jfs -Name Native -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Unicode)]
public static extern System.IntPtr SendMessageTimeout(
    System.IntPtr hWnd, int Msg, System.IntPtr wParam, string lParam,
    int fuFlags, int uTimeout, out System.IntPtr lpdwResult);
"@
    }
    $result = [IntPtr]::Zero
    [void][Jfs.Native]::SendMessageTimeout([IntPtr]0xffff, 0x001A, [IntPtr]::Zero, "Environment", 2, 5000, [ref]$result)
}

$DataDir = Join-Path $env:LOCALAPPDATA "jfs"
$BinDir = Join-Path $DataDir "bin"
$Current = Join-Path $DataDir "current"
$Exe = Join-Path $BinDir "jfs.exe"

New-Item -ItemType Directory -Force -Path $BinDir | Out-Null

if ($LocalExe) {
    if (-not (Test-Path -LiteralPath $LocalExe)) {
        throw "Local executable not found: $LocalExe"
    }
    Copy-Item -LiteralPath $LocalExe -Destination $Exe -Force
    Write-Host "Installed $Exe from $LocalExe"
} else {
    $asset = Get-CpuArch
    $url = "https://github.com/$Repo/releases/latest/download/$asset"
    Write-Host "Downloading $url"
    & curl.exe --fail --location --silent --show-error --output $Exe $url
    if ($LASTEXITCODE -ne 0) {
        throw "Could not download $url. Tag a GitHub release (vX.Y.Z) with $asset attached, then run this again."
    }
}

Unblock-File -LiteralPath $Exe
$stream = [IO.File]::OpenRead($Exe)
try {
    $mz0 = $stream.ReadByte()
    $mz1 = $stream.ReadByte()
} finally {
    $stream.Close()
}
if ($mz0 -ne 0x4D -or $mz1 -ne 0x5A) {
    throw "Downloaded file is not a Windows executable."
}

# Absolute path. SetEnvironmentVariable writes REG_SZ, which is what Java tools read.
# PATH is updated through the registry so an existing REG_EXPAND_SZ value stays expandable.
# setx is intentionally not used: it truncates long PATH values.
[Environment]::SetEnvironmentVariable("JAVA_HOME", $Current, "User")

$pathState = Get-UserPathState
if ($pathState.Kind -eq "ExpandString") {
    $binEntry = "%LOCALAPPDATA%\jfs\bin"
    $javaEntry = "%JAVA_HOME%\bin"
} else {
    $binEntry = $BinDir
    $javaEntry = Join-Path $Current "bin"
}
$updated = Add-PathEntry $pathState.Value $javaEntry
$updated = Add-PathEntry $updated $binEntry

$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
try {
    $kind = if ($pathState.Kind -eq "ExpandString") {
        [Microsoft.Win32.RegistryValueKind]::ExpandString
    } else {
        [Microsoft.Win32.RegistryValueKind]::String
    }
    $key.SetValue($pathState.Name, $updated, $kind)
} finally {
    $key.Close()
}

Publish-EnvironmentChange

$env:JAVA_HOME = $Current
$env:Path = "$BinDir;$env:Path"

Write-Host ""
Write-Host "jfs is installed."
Write-Host "  exe       $Exe"
Write-Host "  JAVA_HOME $Current"
Write-Host ""
Write-Host "Open a new terminal, then register a JDK you already have:"
Write-Host "  jfs add 21 `"C:\path\to\jdk`""
Write-Host "  jfs use 21"
Write-Host "  java -version"
Write-Host ""
Write-Host "Switching versions does not change JAVA_HOME again. It retargets the current junction."

$machineKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey("SYSTEM\CurrentControlSet\Control\Session Manager\Environment", $false)
if ($machineKey) {
    try {
        $machinePath = [string]$machineKey.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally {
        $machineKey.Close()
    }
    $shadows = @()
    foreach ($entry in ($machinePath -split ";")) {
        if ([string]::IsNullOrWhiteSpace($entry)) { continue }
        if ($entry -match '%JAVA_HOME%') { continue }
        $expanded = [Environment]::ExpandEnvironmentVariables($entry)
        $candidate = Join-Path $expanded "java.exe"
        if (Test-Path -LiteralPath $candidate) { $shadows += $entry }
    }
    if ($shadows.Count -gt 0) {
        Write-Host ""
        Write-Host "warning: Windows searches the system PATH before your user PATH." -ForegroundColor Yellow
        Write-Host "These system directories contain java.exe and will hide jfs until they are removed:" -ForegroundColor Yellow
        $shadows | ForEach-Object { Write-Host "  $_" -ForegroundColor Yellow }
    }
}

& $Exe --version
& $Exe list
