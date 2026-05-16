#!/usr/bin/env bash
# ============================================================================
# Hermes Dashboard V4 — 安全卸载脚本 (Linux / macOS)
# ============================================================================
# 一键卸载：
#   curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/uninstall.sh | bash
#
# 保留数据（仅移除程序文件）：
#   bash uninstall.sh
#
# 完全清除（包括 usage.db / state.db）：
#   bash uninstall.sh --purge
#
# 静默模式（不询问，默认保留数据）：
#   bash uninstall.sh --yes
# ============================================================================

set -euo pipefail

# ---- 颜色 ----
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'
CYAN='\033[0;36m'; BOLD='\033[1m'; NC='\033[0m'

info()  { printf "${GREEN}✓${NC} %s\n" "$*"; }
warn()  { printf "${YELLOW}⚠${NC} %s\n" "$*"; }
err()   { printf "${RED}✗${NC} %s\n" "$*"; }
header(){ printf "\n${CYAN}═══ %s ═══${NC}\n" "$*"; }
bold()  { printf "${BOLD}%s${NC}\n" "$*"; }

# ---- 默认路径 ----
INSTALL_DIR="${INSTALL_DIR:-$HOME/.hermes/dashboard-v4}"
BIN_DIR="${BIN_DIR:-$HOME/.local/bin}"
HERMES_HOME="${HERMES_HOME:-$HOME/.hermes}"
SERVICE_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user"
SERVICE_NAME="hermes-dashboard.service"
BINARY="dashboard-server"

PURGE=false
YES=false

# ---- 解析参数 ----
while [[ $# -gt 0 ]]; do
  case "$1" in
    --purge)   PURGE=true  ; shift ;;
    --yes|-y)  YES=true    ; shift ;;
    --help|-h)
      sed -n '3,15p' "$0"
      echo ""
      echo "  选项:"
      echo "    --purge     同时删除 Hermes 数据库 (usage.db / state.db)"
      echo "    --yes, -y   静默模式，跳过确认提示"
      echo "    --help, -h  显示此帮助"
      exit 0
      ;;
    *) err "未知参数: $1 (使用 --help 查看用法)" ;;
  esac
done

# ============================================================================
# 预检
# ============================================================================
header "Hermes Dashboard V4 — 卸载"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
echo "  系统: $OS"
echo "  卸载目录: $INSTALL_DIR"
echo "  二进制目录: $BIN_DIR"
echo ""

# 检测哪些组件已安装
FOUND=false

BINARY_PATH="$BIN_DIR/$BINARY"
[[ -f "$BINARY_PATH" ]]           && FOUND=true || BINARY_PATH=""
[[ -d "$INSTALL_DIR" ]]           && FOUND=true
[[ -f "$SERVICE_DIR/$SERVICE_NAME" ]] && FOUND=true

if ! $FOUND; then
  warn "未检测到 Hermes Dashboard V4 安装记录"
  echo "  已检查:"
  echo "    - 二进制: $BINARY_PATH"
  echo "    - 安装目录: $INSTALL_DIR"
  echo "    - systemd 服务: $SERVICE_DIR/$SERVICE_NAME"
  echo ""
  echo "  如需查找残余文件，请手动搜索:"
  echo "    find ~/.hermes -name '*dashboard*' 2>/dev/null"
  exit 0
fi

# ---- 确认 ----
if ! $YES; then
  bold ""
  bold "以下组件将被移除:"
  [[ -n "$BINARY_PATH" ]] && echo "  • 二进制: $BINARY_PATH"
  [[ -d "$INSTALL_DIR" ]] && echo "  • 安装目录: $INSTALL_DIR/"
  [[ -f "$SERVICE_DIR/$SERVICE_NAME" ]] && echo "  • systemd 服务: $SERVICE_DIR/$SERVICE_NAME"
  echo ""
  if $PURGE; then
    warn "⚠ --purge 模式：usage.db / state.db 也将被删除！"
  else
    echo "  Hermes 数据库文件 (usage.db / state.db) 将保留"
    echo "  使用 --purge 可一并清除数据库"
  fi
  echo ""
  read -r -p "确认卸载？[y/N] " CONFIRM
  [[ "$CONFIRM" =~ ^[Yy]$ ]] || { info "已取消"; exit 0; }
fi

echo ""

# ============================================================================
# 清理流程
# ============================================================================

# 1. 停止运行中的 dashboard-server 进程
header "停止运行中的进程"
if command -v pkill &>/dev/null; then
  pkill -f "$BINARY" 2>/dev/null && info "已终止 dashboard-server 进程" || warn "未发现运行中的 dashboard-server"
elif [[ "$OS" == "darwin" ]]; then
  # macOS fallback
  kill "$(pgrep -f "$BINARY" 2>/dev/null)" 2>/dev/null && info "已终止 dashboard-server 进程" || warn "未发现运行中的 dashboard-server"
fi

# 2. systemd 服务处理 (Linux)
if [[ "$OS" == "linux" ]] && command -v systemctl &>/dev/null; then
  if systemctl --user is-enabled "$SERVICE_NAME" &>/dev/null 2>&1; then
    header "停用 systemd 服务"
    systemctl --user stop "$SERVICE_NAME" 2>/dev/null   && info "已停止服务"
    systemctl --user disable "$SERVICE_NAME" 2>/dev/null && info "已禁用开机自启"
  fi
fi

# 3. 删除 systemd service 文件
if [[ -f "$SERVICE_DIR/$SERVICE_NAME" ]]; then
  header "删除 systemd 服务文件"
  rm -f "$SERVICE_DIR/$SERVICE_NAME"
  info "已删除: $SERVICE_DIR/$SERVICE_NAME"
  if command -v systemctl &>/dev/null; then
    systemctl --user daemon-reload
    info "systemd daemon 已重载"
  fi
fi

# 4. 删除二进制
if [[ -n "$BINARY_PATH" ]]; then
  header "删除二进制"
  rm -f "$BINARY_PATH"
  info "已删除: $BINARY_PATH"
fi

# 5. 删除安装目录 (dist, scripts)
if [[ -d "$INSTALL_DIR" ]]; then
  header "删除安装目录"
  rm -rf "$INSTALL_DIR"
  info "已删除: $INSTALL_DIR/"
fi

# 6. 清理空目录 (如果 ~/.hermes 空了，删除它)
if [[ -d "$HERMES_HOME" ]] && [[ -z "$(ls -A "$HERMES_HOME" 2>/dev/null)" ]]; then
  rmdir "$HERMES_HOME" 2>/dev/null && info "已删除空目录: $HERMES_HOME"
fi

# 7. 可选: 删除 Hermes 数据库 (--purge)
if $PURGE; then
  header "清除数据库"
  for db in usage.db state.db; do
    db_path="$HERMES_HOME/$db"
    if [[ -f "$db_path" ]]; then
      rm -f "$db_path"
      info "已删除: $db_path"
    fi
  done
  # 清理空目录
  if [[ -d "$HERMES_HOME" ]] && [[ -z "$(ls -A "$HERMES_HOME" 2>/dev/null)" ]]; then
    rmdir "$HERMES_HOME" 2>/dev/null
  fi
fi

# 8. 检查 PATH 并提示清理
if [[ -n "$BIN_DIR" ]] && [[ ":$PATH:" == *":$BIN_DIR:"* ]]; then
  warn "$BIN_DIR 仍在 PATH 中，如需清理请编辑 shell 配置 (~/.bashrc, ~/.zshrc 等)"
  echo "  查找: grep '$BIN_DIR' ~/.bashrc ~/.zshrc ~/.bash_profile 2>/dev/null"
fi

# ============================================================================
# 完成
# ============================================================================
echo ""
header "卸载完成"
echo ""
if $PURGE; then
  echo "  ${RED}完全清除${NC} — 所有 Dashboard 文件及数据库已移除"
else
  echo "  ${GREEN}安全卸载${NC} — Dashboard 文件已移除"
  echo "  数据库文件保留在: $HERMES_HOME/"
  echo "  如需彻底清除请使用: bash <(curl -fsSL ...) --purge"
fi
echo ""
echo "  ${CYAN}再见！${NC}"
echo ""
