# install.ps1 — Build and install the Social Production Network node (sp-node)
#
# Supported platform: Windows (PowerShell 5.1+ or PowerShell 7+)
# For Linux/macOS use scripts/install.sh instead.
#
# Usage (run from repo root or scripts\):
#   powershell -ExecutionPolicy Bypass -File scripts\install.ps1 [-Prefix <dir>]
#
# Options:
#   -Prefix <dir>   Install directory.  Defaults to $env:LOCALAPPDATA\Programs\spn
#                   Pass a directory under Program Files to install system-wide
#                   (requires running as Administrator).
#
# The script optionally registers a Windows Service using sc.exe when run as
# Administrator; otherwise it just adds the binary to the user PATH.

[CmdletBinding()]
param(
    [string]$Prefix = "$env:LOCALAPPDATA\Programs\spn"
)

$ErrorActionPreference = 'Stop'

# ── Resolve paths ──────────────────────────────────────────────────────────────
$ScriptDir    = Split-Path -Parent $MyInvocation.MyCommand.Path
$WorkspaceDir = Split-Path -Parent $ScriptDir
$BinDir       = Join-Path $Prefix 'bin'
$BinaryDst    = Join-Path $BinDir 'sp-node.exe'

# ── Build ──────────────────────────────────────────────────────────────────────
Write-Host "==> Building sp-node (release)..."
Push-Location $WorkspaceDir
try {
    cargo build --release --manifest-path Cargo.toml -p sp-node
} finally {
    Pop-Location
}

$BinarySrc = Join-Path $WorkspaceDir 'target\release\sp-node.exe'
if (-not (Test-Path $BinarySrc)) {
    Write-Error "Build succeeded but binary not found at $BinarySrc"
    exit 1
}

# ── Install binary ─────────────────────────────────────────────────────────────
Write-Host "==> Installing sp-node to $BinDir..."
New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
Copy-Item -Force $BinarySrc $BinaryDst
Write-Host "    Installed: $BinaryDst"

# ── Add to user PATH (if not already present) ──────────────────────────────────
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($UserPath -notlike "*$BinDir*") {
    Write-Host "==> Adding $BinDir to user PATH..."
    [Environment]::SetEnvironmentVariable('Path', "$UserPath;$BinDir", 'User')
    Write-Host "    Added to PATH.  Restart your terminal for it to take effect."
} else {
    Write-Host "    $BinDir already in PATH."
}

# ── Optionally register a Windows Service (requires Administrator) ─────────────
$IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
            ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if ($IsAdmin) {
    $ServiceName = 'sp-node'
    $existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

    if ($null -eq $existing) {
        Write-Host "==> Registering Windows Service '$ServiceName'..."
        sc.exe create $ServiceName `
            binPath= "`"$BinaryDst`" --quiet" `
            DisplayName= "Social Production Network Node" `
            start= auto | Out-Null
        sc.exe description $ServiceName "P2P node for the Social Production Network." | Out-Null
        Write-Host "    Service registered.  Start with:"
        Write-Host "      Start-Service $ServiceName"
        Write-Host "    Or via sc.exe:  sc.exe start $ServiceName"
    } else {
        Write-Host "    Windows Service '$ServiceName' already exists — skipping registration."
        Write-Host "    To update it, stop the service first then re-run this script."
    }
} else {
    Write-Host ""
    Write-Host "    (Skipping Windows Service registration — run as Administrator to register the service)"
    Write-Host "    You can register it manually from an elevated prompt:"
    Write-Host "      sc.exe create sp-node binPath= `"`"$BinaryDst`" --quiet`" DisplayName= `"Social Production Network Node`" start= auto"
    Write-Host "      sc.exe start sp-node"
}

Write-Host ""
Write-Host "Installation complete.  Run 'sp-node --help' to get started."
