#!/usr/bin/env bash
# ============================================================================
# Hermes Dashboard V4 — 一键安装脚本 (Linux / macOS)
# ============================================================================
# 用法:
#   curl -fsSL https://git.io/hermes-d4 | bash
#   curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh | bash
#
# 选项:
#   bash install.sh --source       从源码构建（需要 Rust）
#   bash install.sh --port 8654    自定义端口
#   bash install.sh --bin-dir ~/.local/bin  自定义二进制安装目录
# ============================================================================

set -euo pipefail

# ---- 颜色 ----
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; NC='\033[0m'

info()  { printf "${GREEN}✓${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}⚠${NC} %s\n" "$*"; }
err()   { printf "${RED}✗${NC} %s\n" "$*"; exit 1; }
header(){ printf "\n${CYAN}═══ %s ═══${NC}\n" "$*"; }

# ---- 默认配置 ----
REPO="Moritz230127/hermes-dashboard-v4"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.hermes/dashboard-v4}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
HERMES_HOME="${HERMES_HOME:-$HOME/.hermes}"
PORT="${PORT:-8654}"
MODE="binary"

# ---- 解析参数 ----
while [[ $# -gt 0 ]]; do
  case "$1" in
    --source)  MODE="source"  ; shift ;;
    --port)    PORT="$2"      ; shift 2 ;;
    --bin-dir) BIN_DIR="$2"   ; shift 2 ;;
    --help|-h) sed -n '3,11p' "$0"; exit 0 ;;
    *) err "未知参数: $1 (使用 --help 查看用法)" ;;
  esac
done

# ---- 系统检测 ----
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)  TARGET="x86_64-unknown-linux-gnu"; EXT="";     PKG_EXT=".tar.gz" ;;
  darwin)
    case "$ARCH" in
      x86_64|amd64) TARGET="x86_64-apple-darwin";   EXT=""; PKG_EXT=".tar.gz" ;;
      arm64|aarch64) TARGET="aarch64-apple-darwin";  EXT=""; PKG_EXT=".tar.gz" ;;
      *) err "不支持的 macOS 架构: $ARCH" ;;
    esac
    ;;
  *) err "不支持的操作系统: $OS (仅支持 Linux / macOS)" ;;
esac

header "Hermes Dashboard V4 — 安装"
echo "  系统:    $OS ($ARCH)"
echo "  模式:    $MODE"
echo "  端口:    $PORT"
echo "  目录:    $INSTALL_DIR"
echo "  二进制:  $BIN_DIR"
echo "  HERMES:  $HERMES_HOME"
echo ""

# ---- 前置依赖检查 ----
check_deps() {
  local missing=()
  for cmd in curl; do
    command -v "$cmd" &>/dev/null || missing+=("$cmd")
  done
  if [[ "$MODE" == "source" ]] && ! command -v cargo &>/dev/null; then
    missing+=("cargo (Rust)")
  fi
  if [[ ${#missing[@]} -gt 0 ]]; then
    err "缺少依赖: ${missing[*]}"
  fi
}
check_deps

# ---- 创建目录 ----
mkdir -p "$INSTALL_DIR" "$BIN_DIR" "$HERMES_HOME"
INSTALL_DIR="$(cd "$INSTALL_DIR" && pwd)"

# ============================================================================
# 模式 A: 下载预编译二进制
# ============================================================================
install_binary() {
  header "下载预编译二进制"

  # 获取最新版本号
  if [[ -n "${VERSION:-}" ]]; then
    VER="$VERSION"
  else
    info "获取最新版本..."
    VER=$(curl -fsSL "https://api.github.com/repos/$REPO/releases/latest" \
          | grep '"tag_name":' | sed 's/.*"tag_name": "\(.*\)",.*/\1/')
    [[ -z "$VER" ]] && err "无法获取最新版本号，请指定 VERSION 环境变量"
  fi

  PKG_NAME="dashboard-v4-${TARGET}${PKG_EXT}"
  URL="https://github.com/$REPO/releases/download/$VER/$PKG_NAME"
  TMP_DIR=$(mktemp -d)
  trap "rm -rf '$TMP_DIR'" EXIT

  info "下载: $URL"
  curl -fsSL "$URL" -o "$TMP_DIR/$PKG_NAME"

  info "解压..."
  if [[ "$PKG_EXT" == ".tar.gz" ]]; then
    tar xzf "$TMP_DIR/$PKG_NAME" -C "$TMP_DIR"
  else
    unzip -qo "$TMP_DIR/$PKG_NAME" -d "$TMP_DIR"
  fi

  # 安装二进制
  local bin_src="$TMP_DIR/dashboard-server${EXT}"
  if [[ -f "$bin_src" ]]; then
    install -m 755 "$bin_src" "$BIN_DIR/dashboard-server"
    info "二进制 → $BIN_DIR/dashboard-server"
  else
    # 可能解压到子目录
    bin_src=$(find "$TMP_DIR" -name "dashboard-server${EXT}" -type f 2>/dev/null | head -1)
    if [[ -n "$bin_src" ]]; then
      install -m 755 "$bin_src" "$BIN_DIR/dashboard-server"
      info "二进制 → $BIN_DIR/dashboard-server"
    else
      err "无法找到 dashboard-server 二进制"
    fi
  fi

  # 复制 dist/ 和 scripts/
  local dist_src scripts_src
  dist_src=$(find "$TMP_DIR" -type d -name dist 2>/dev/null | head -1)
  scripts_src=$(find "$TMP_DIR" -type d -name scripts 2>/dev/null | head -1)
  if [[ -n "$dist_src" ]]; then
    cp -r "$dist_src" "$INSTALL_DIR/" && info "静态文件 → $INSTALL_DIR/dist/"
  else
    warn "未找到 dist/ 目录"
  fi
  if [[ -n "$scripts_src" ]]; then
    cp -r "$scripts_src" "$INSTALL_DIR/" && info "脚本 → $INSTALL_DIR/scripts/"
  else
    warn "未找到 scripts/ 目录"
  fi
}

# ============================================================================
# 模式 B: 从源码构建
# ============================================================================
install_source() {
  header "从源码构建"

  if ! command -v cargo &>/dev/null; then
    err "需要 Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
  fi

  local BUILD_DIR=$(mktemp -d)
  trap "rm -rf '$BUILD_DIR'" EXIT

  info "克隆仓库..."
  git clone --depth=1 "https://github.com/$REPO.git" "$BUILD_DIR"
  cd "$BUILD_DIR"

  info "构建 server (release)..."
  cargo build --release --package dashboard-server 2>&1 | tail -5

  local binary="target/release/dashboard-server"
  if [[ ! -f "$binary" ]]; then
    # Windows 可能有 .exe
    binary="target/release/dashboard-server${EXT}"
    [[ -f "$binary" ]] || err "构建失败：未找到二进制文件"
  fi

  install -m 755 "$binary" "$BIN_DIR/dashboard-server"
  info "二进制 → $BIN_DIR/dashboard-server"

  # 复制 dist/ 和 scripts/
  cp -r dist "$INSTALL_DIR/" 2>/dev/null || warn "未找到 dist/ 目录"
  cp -r scripts "$INSTALL_DIR/" 2>/dev/null || warn "未找到 scripts/ 目录"
}

# ============================================================================
# 执行安装
# ============================================================================
case "$MODE" in
  binary) install_binary ;;
  source) install_source  ;;
esac

# ---- 创建 systemd user service (Linux) ----
if [[ "$OS" == "linux" ]] && command -v systemctl &>/dev/null; then
  header "配置 systemd 用户服务"

  SERVICE_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
  mkdir -p "$SERVICE_DIR"

  cat > "$SERVICE_DIR/hermes-dashboard.service" <<SVC
[Unit]
Description=Hermes Dashboard V4
After=network.target

[Service]
Type=simple
ExecStart=$BIN_DIR/dashboard-server
Environment=HERMES_HOME=$HERMES_HOME
Environment=HERMES_DASHBOARD_PORT=$PORT
WorkingDirectory=$INSTALL_DIR
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
SVC

  systemctl --user daemon-reload
  info "systemd 服务已创建"
  echo "  启动: systemctl --user start hermes-dashboard"
  echo "  开机启动: systemctl --user enable hermes-dashboard"
  echo "  查看日志: journalctl --user -u hermes-dashboard -f"
fi

# ---- 检查 PATH ----
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
  warn "$BIN_DIR 不在 PATH 中！请添加至 shell 配置:"
  echo "  echo 'export PATH=\"\$PATH:$BIN_DIR\"' >> ~/.bashrc"
  echo "  source ~/.bashrc"
fi

# ---- 验证 ----
header "安装完成"
echo ""
if command -v dashboard-server &>/dev/null; then
  echo "  ${GREEN}dashboard-server${NC} 已就绪"
  echo ""
  echo "  ${CYAN}启动命令:${NC}"
  echo "    dashboard-server"
  echo ""
  if [[ "$OS" == "linux" ]]; then
    echo "  ${CYAN}systemd 启动:${NC}"
    echo "    systemctl --user start hermes-dashboard"
    echo "    systemctl --user enable hermes-dashboard"
    echo ""
  fi
  echo "  ${CYAN}浏览器访问:${NC}"
  echo "    http://localhost:$PORT"
  echo ""
  echo "  ${CYAN}前置要求:${NC}"
  echo "    确认 ~/.hermes/usage.db 和 ~/.hermes/state.db 存在"
  echo "    （由 Hermes Agent 自动生成）"
else
  warn "dashboard-server 不在当前 PATH 中"
  echo "  请手动添加 $BIN_DIR 到 PATH 后重试"
fi
echo ""
