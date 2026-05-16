<#
.SYNOPSIS
    Hermes Dashboard V4 — 安全卸载脚本 (Windows)
.DESCRIPTION
    移除 dashboard-server 二进制、安装目录、环境变量。
    可选择保留或清除 Hermes 数据库文件。
.LINK
    https://github.com/Moritz230127/hermes-dashboard-v4

    一键卸载 (管理员 PowerShell):
    irm https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/uninstall.ps1 | iex

    完全清除 (包括数据库):
    irm ... | iex -Args "--purge"

    静默卸载:
    irm ... | iex -Args "--yes"
#>

param(
    [switch]$Purge,
    [switch]$Yes,
    [string]$InstallDir = "$env:USERPROFILE\.hermes\dashboard-v4",
    [string]$BinDir = "$env:USERPROFILE\.hermes\bin",
    [string]$ServiceName = "hermes-dashboard"
)

$ErrorActionPreference = "Stop"

# ---- 颜色 ----
function Write-Info  { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "⚠ $args" -ForegroundColor Yellow }
function Write-Err   { Write-Host "✗ $args" -ForegroundColor Red }
function Write-Head  { Write-Host "`n══ $args ══" -ForegroundColor Cyan }

# ============================================================================
# 预检
# ============================================================================
$HERMES_HOME = "$env:USERPROFILE\.hermes"
$binaryPath = "$BinDir\dashboard-server.exe"
$found = $false

if (Test-Path $binaryPath)    { $found = $true }
if (Test-Path $InstallDir)    { $found = $true }

Write-Head "Hermes Dashboard V4 — 卸载 (Windows)"

if (-not $found) {
    Write-Warn "未检测到 Hermes Dashboard V4 安装记录"
    Write-Host "  已检查:"
    Write-Host "    - 二进制: $binaryPath"
    Write-Host "    - 安装目录: $InstallDir"
    exit 0
}

# ---- 打印待删除项 ----
if (-not $Yes) {
    Write-Host ""
    Write-Host "以下组件将被移除:" -ForegroundColor Cyan
    if (Test-Path $binaryPath) { Write-Host "  • 二进制: $binaryPath" }
    if (Test-Path $InstallDir) { Write-Host "  • 安装目录: $InstallDir\" }
    Write-Host ""
    if ($Purge) {
        Write-Warn "--purge 模式：usage.db / state.db 也将被删除！"
    } else {
        Write-Host "  Hermes 数据库文件将保留"
        Write-Host "  使用 -Purge 可一并清除数据库"
    }
    Write-Host ""
    $confirm = Read-Host "确认卸载？[y/N] "
    if ($confirm -notmatch '^[Yy]$') { Write-Info "已取消"; exit 0 }
}

Write-Host ""

# ============================================================================
# 1. 停止运行中的进程
# ============================================================================
Write-Head "停止运行中的进程"
$process = Get-Process -Name "dashboard-server" -ErrorAction SilentlyContinue
if ($process) {
    $process | Stop-Process -Force
    Write-Info "已终止 dashboard-server 进程"
} else {
    Write-Warn "未发现运行中的 dashboard-server"
}

# ============================================================================
# 2. 删除二进制
# ============================================================================
if (Test-Path $binaryPath) {
    Write-Head "删除二进制"
    Remove-Item -Force $binaryPath
    Write-Info "已删除: $binaryPath"
}

# ============================================================================
# 3. 删除安装目录
# ============================================================================
if (Test-Path $InstallDir) {
    Write-Head "删除安装目录"
    Remove-Item -Recurse -Force $InstallDir
    Write-Info "已删除: $InstallDir\"
}

# ============================================================================
# 4. 从 PATH 中移除 BinDir
# ============================================================================
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -and $userPath -like "*$BinDir*") {
    Write-Head "清理系统 PATH"
    $newPath = ($userPath -split ';' | Where-Object { $_ -ne $BinDir -and $_ -ne "$BinDir\" }) -join ';'
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    $env:Path = ($env:Path -split ';' | Where-Object { $_ -ne $BinDir -and $_ -ne "$BinDir\" }) -join ';'
    Write-Info "已从 PATH 中移除: $BinDir"
}

# ============================================================================
# 5. --purge: 删除 Hermes 数据库
# ============================================================================
if ($Purge) {
    Write-Head "清除数据库"
    $dbs = @("usage.db", "state.db")
    foreach ($db in $dbs) {
        $dbPath = "$HERMES_HOME\$db"
        if (Test-Path $dbPath) {
            Remove-Item -Force $dbPath
            Write-Info "已删除: $dbPath"
        }
    }
    # 清理空目录
    if (Test-Path $HERMES_HOME) {
        $items = Get-ChildItem $HERMES_HOME -ErrorAction SilentlyContinue
        if (-not $items) {
            Remove-Item $HERMES_HOME
            Write-Info "已删除空目录: $HERMES_HOME"
        }
    }
}

# ============================================================================
# 完成
# ============================================================================
Write-Host ""
Write-Head "卸载完成"
Write-Host ""
if ($Purge) {
    Write-Host "  ✗ 完全清除 — 所有 Dashboard 文件及数据库已移除" -ForegroundColor Red
} else {
    Write-Host "  ✓ 安全卸载 — Dashboard 文件已移除" -ForegroundColor Green
    Write-Host "  数据库文件保留在: $HERMES_HOME\"
    Write-Host "  如需彻底清除请使用: iex -Args '--purge'"
}
Write-Host ""
Write-Host "  再见！" -ForegroundColor Cyan
Write-Host ""
