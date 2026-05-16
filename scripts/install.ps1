<#
.SYNOPSIS
    Hermes Dashboard V4 — 一键安装脚本 (Windows)
.DESCRIPTION
    从 GitHub 下载预编译二进制或从源码构建。
    支持 PowerShell 5.1+ / PowerShell 7+
.LINK
    https://github.com/Moritz230127/hermes-dashboard-v4

    # 一键安装 (管理员 PowerShell):
    irm https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.ps1 | iex

    # 从源码构建:
    irm ... | iex -Args "--source"
#>

param(
    [switch]$Source,
    [string]$Port = "8654",
    [string]$InstallDir = "$env:USERPROFILE\.hermes\dashboard-v4",
    [string]$BinDir = "$env:USERPROFILE\.hermes\bin"
)

$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"
$Repo = "Moritz230127/hermes-dashboard-v4"

# ---- 颜色 ----
function Write-Info  { Write-Host "✓ $args" -ForegroundColor Green }
function Write-Warn  { Write-Host "⚠ $args" -ForegroundColor Yellow }
function Write-Err   { Write-Host "✗ $args" -ForegroundColor Red; exit 1 }
function Write-Head  { Write-Host "`n══ $args ══" -ForegroundColor Cyan }

# ---- 检测架构 ----
$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    "AMD64"  { "x86_64-pc-windows-msvc" }
    "ARM64"  { "aarch64-pc-windows-msvc" }
    default  { Write-Err "不支持的架构: $env:PROCESSOR_ARCHITECTURE" }
}

Write-Head "Hermes Dashboard V4 — 安装 (Windows)"
Write-Host "  架构: $arch"
Write-Host "  模式: $(if($Source){'源码'}else{'二进制'})"
Write-Host "  端口: $Port"
Write-Host ""

# ---- 前置检查 ----
if (-Not (Get-Command curl -ErrorAction SilentlyContinue) -and
    -Not (Get-Command Invoke-WebRequest -ErrorAction SilentlyContinue)) {
    Write-Err "需要 curl 或 PowerShell 5.1+"
}
if ($Source -and -Not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Err "源码模式需要 Rust: https://rustup.rs"
}

# ---- 创建目录 ----
New-Item -ItemType Directory -Force -Path $InstallDir, $BinDir, "$env:USERPROFILE\.hermes" | Out-Null

# ============================================================================
# 模式 A: 下载预编译二进制
# ============================================================================
if (-Not $Source) {
    Write-Head "下载预编译二进制"

    # 获取最新版本
    $ver = if ($env:VERSION) { $env:VERSION } else {
        Write-Info "获取最新版本..."
        try {
            $release = curl.exe -s "https://api.github.com/repos/$Repo/releases/latest"
            ($release | ConvertFrom-Json).tag_name
        } catch {
            Write-Err "无法获取版本号，请设置 VERSION 环境变量"
        }
    }

    $pkgName = "dashboard-v4-${arch}.zip"
    $url = "https://github.com/$Repo/releases/download/$ver/$pkgName"
    $tmpDir = "$env:TEMP\hermes-d4"

    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
    New-Item -ItemType Directory -Force -Path $tmpDir | Out-Null

    Write-Info "下载: $url"
    curl.exe -fsSL "$url" -o "$tmpDir\$pkgName"

    Write-Info "解压..."
    Expand-Archive -Path "$tmpDir\$pkgName" -DestinationPath $tmpDir -Force

    # 安装二进制
    $binSrc = Get-ChildItem -Recurse "$tmpDir\dashboard-server.exe" | Select-Object -First 1
    if (-Not $binSrc) { Write-Err "未找到 dashboard-server.exe" }
    Copy-Item $binSrc.FullName "$BinDir\dashboard-server.exe" -Force
    Write-Info "二进制 → $BinDir\dashboard-server.exe"

    # 复制 dist/ 和 scripts/
    if (Test-Path "$tmpDir\dist") {
        Copy-Item "$tmpDir\dist\*" "$InstallDir\dist\" -Recurse -Force
    }
    if (Test-Path "$tmpDir\scripts") {
        Copy-Item "$tmpDir\scripts\*" "$InstallDir\scripts\" -Recurse -Force
    }
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}
# ============================================================================
# 模式 B: 从源码构建
# ============================================================================
else {
    Write-Head "从源码构建"
    $buildDir = "$env:TEMP\hermes-d4-build"

    Remove-Item -Recurse -Force $buildDir -ErrorAction SilentlyContinue

    Write-Info "克隆仓库..."
    git clone --depth=1 "https://github.com/$Repo.git" $buildDir
    Set-Location $buildDir

    Write-Info "构建 server (release)..."
    cargo build --release --package dashboard-server
    if (-Not (Test-Path "target\release\dashboard-server.exe")) {
        Write-Err "构建失败"
    }

    Copy-Item "target\release\dashboard-server.exe" "$BinDir\dashboard-server.exe" -Force
    Write-Info "二进制 → $BinDir\dashboard-server.exe"

    if (Test-Path "dist")  { Copy-Item "dist\*" "$InstallDir\dist\" -Recurse -Force }
    if (Test-Path "scripts") { Copy-Item "scripts\*" "$InstallDir\scripts\" -Recurse -Force }
    Remove-Item -Recurse -Force $buildDir -ErrorAction SilentlyContinue
}

# ---- 添加 PATH ----
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$BinDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$BinDir", "User")
    Write-Info "已将 $BinDir 添加到用户 PATH"
    $env:Path += ";$BinDir"
}

# ---- 创建快捷启动脚本 ----
$launcher = @"
@echo off
REM Hermes Dashboard V4 — Windows 启动脚本
set HERMES_HOME=%USERPROFILE%\.hermes
set HERMES_DASHBOARD_PORT=$Port
cd /d "$InstallDir"
"$BinDir\dashboard-server.exe"
"@
Set-Content -Path "$InstallDir\start.bat" -Value $launcher -Encoding ASCII
Write-Info "启动脚本 → $InstallDir\start.bat"

# ---- 完成 ----
Write-Head "安装完成"
Write-Host ""
if (Get-Command dashboard-server.exe -ErrorAction SilentlyContinue) {
    Write-Host "  dashboard-server.exe 已就绪" -ForegroundColor Green
    Write-Host ""
    Write-Host "  ${CYAN}启动命令:${NC}" -ForegroundColor Cyan
    Write-Host "    dashboard-server.exe"
    Write-Host ""
    Write-Host "  ${CYAN}或双击:${NC}" -ForegroundColor Cyan
    Write-Host "    $InstallDir\start.bat"
    Write-Host ""
    Write-Host "  ${CYAN}浏览器访问:${NC}" -ForegroundColor Cyan
    Write-Host "    http://localhost:$Port"
    Write-Host ""
    Write-Host "  ${CYAN}前置要求:${NC}" -ForegroundColor Cyan
    Write-Host "    确认 %USERPROFILE%\.hermes\usage.db 和 state.db 存在"
    Write-Host "    （由 Hermes Agent 自动生成）"
} else {
    Write-Warn "dashboard-server.exe 不在 PATH 中，请重启终端或手动添加:"
    Write-Host "  set PATH=%PATH%;$BinDir"
}
