# uninstall.ps1 — Remove the Social Production Network node (sp-node)
#
# Supported platform: Windows (PowerShell 5.1+ or PowerShell 7+)
# For Linux/macOS use scripts/uninstall.sh instead.
#
# Usage:
#   powershell -ExecutionPolicy Bypass -File scripts\uninstall.ps1 [-Prefix <dir>]
#
# Options:
#   -Prefix <dir>   Install directory used during installation.
#                   Defaults to $env:LOCALAPPDATA\Programs\spn

[CmdletBinding()]
param(
    [string]$Prefix = "$env:LOCALAPPDATA\Programs\spn"
)

$ErrorActionPreference = 'Stop'

$BinDir    = Join-Path $Prefix 'bin'
$BinaryDst = Join-Path $BinDir 'sp-node.exe'

# ── Check for Administrator privileges ────────────────────────────────────────
$IsAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()
            ).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

# ── Stop and remove Windows Service (requires Administrator) ───────────────────
$ServiceName = 'sp-node'
$existing = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue

if ($null -ne $existing) {
    if ($IsAdmin) {
        Write-Host "==> Stopping and removing Windows Service '$ServiceName'..."
        if ($existing.Status -eq 'Running') {
            Stop-Service -Name $ServiceName -Force
            Write-Host "    Stopped service."
        }
        sc.exe delete $ServiceName | Out-Null
        Write-Host "    Removed service '$ServiceName'."
    } else {
        Write-Host ""
        Write-Host "    (Skipping Windows Service removal — run as Administrator to remove the service)"
        Write-Host "    You can remove it manually from an elevated prompt:"
        Write-Host "      Stop-Service sp-node"
        Write-Host "      sc.exe delete sp-node"
    }
}

# ── Remove binary ──────────────────────────────────────────────────────────────
if (Test-Path $BinaryDst) {
    Write-Host "==> Removing $BinaryDst..."
    Remove-Item -Force $BinaryDst
    Write-Host "    Removed: $BinaryDst"

    # Remove the bin dir if it's now empty.
    if ((Get-ChildItem $BinDir -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
        Remove-Item -Force $BinDir -ErrorAction SilentlyContinue
    }
} else {
    Write-Host "    Binary not found at $BinaryDst (already removed?)"
}

# ── Remove from user PATH ──────────────────────────────────────────────────────
$UserPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($UserPath -like "*$BinDir*") {
    Write-Host "==> Removing $BinDir from user PATH..."
    $NewPath = ($UserPath -split ';' | Where-Object { $_ -ne $BinDir }) -join ';'
    [Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    Write-Host "    Removed from PATH.  Restart your terminal for it to take effect."
}

Write-Host ""
Write-Host "Uninstall complete."
