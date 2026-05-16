<div align="center">

# Hermes Dashboard V4

<img src="https://img.shields.io/badge/Rust-Axum-orange?logo=rust" />
<img src="https://img.shields.io/badge/Frontend-Dioxus_WASM-blue?logo=webassembly" />
<img src="https://img.shields.io/badge/Linux-macOS-Windows-brightgreen" />
<img src="https://img.shields.io/github/v/release/Moritz230127/hermes-dashboard-v4" />

**Hermes Agent 实时流量监控面板** — Rust 高性能后端 + WASM 前端

[📖 特性](#特性) • [⚡ 一键安装](#-一键安装) • [📦 对比](#-两种安装模式对比) • [🔧 配置](#-配置) • [🛠 开发](#-从源码开发)

</div>

---

## 特性

- **实时监控** — Token 用量、API 调用量、模型分布、TPS 一目了然
- **会话管理** — 查看运行中会话、停止会话、批量清理
- **历史趋势** — 7 天平均、30 天趋势、10 分钟粒度热力图
- **告警系统** — TPS 过低、缓存命中率、异常流量自动检测
- **跨平台** — Linux / macOS / Windows 全支持
- **双模式** — 预编译二进制快速上手 或 源码构建完整控制

---

## ⚡ 一键安装

### Linux / macOS

```bash
# 自动下载最新预编译二进制（推荐）
curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh | bash
```

> macOS 用户首次运行需授权：`xattr -dr com.apple.quarantine ~/.local/bin/dashboard-server`

### Windows

```powershell
# PowerShell (管理员)
irm https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.ps1 | iex
```

### 从源码安装（需要 Rust）

```bash
curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh | bash -s -- --source
```

安装后终端输入 `dashboard-server` 即可启动。

### 安全卸载

```bash
# Linux / macOS
curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/uninstall.sh | bash

# 完全清除（包括 Hermes 数据库）
curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/uninstall.sh | bash -s -- --purge
```

```powershell
# Windows (管理员 PowerShell)
irm https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/uninstall.ps1 | iex

# 完全清除
irm ... | iex -Args "--purge"
```

> 卸载脚本会：停止进程 → 停用 systemd 服务 → 删除二进制 → 清理安装目录。
> 默认**保留** Hermes 数据库 (`usage.db` / `state.db`)，加 `--purge` 一并清除。

---

## 📦 两种安装模式对比

|  | ⚡ 二进制模式（默认） | 🔧 源码模式 |
|---|---|---|
| **前置依赖** | 仅 `curl` | Rust 工具链（`rustup`） |
| **安装速度** | ～10 秒 | ～3 分钟（编译） |
| **二进制体积** | ～15 MB（已编译） | — |
| **适用场景** | 快速部署、生产环境 | 开发者、自定义修改 |
| **架构优化** | 特定 CPU 优化 | 本地 CPU 原生优化 |

---

## 🚀 快速启动

```bash
# 1. 启动服务器
dashboard-server

# 2. 浏览器打开
open http://localhost:8654
```

启动日志：
```
Hermes API Dashboard V4 (Rust)
  http://127.0.0.1:8654
  API: http://127.0.0.1:8654/api/data
  Databases: /home/user/.hermes
  V4 Features: Pool Rotator, SSE, Cleanup
  Press Ctrl+C to stop
```

### systemd 自启动（Linux）

```bash
# 安装脚本已自动创建服务，仅需启用
systemctl --user start hermes-dashboard
systemctl --user enable hermes-dashboard
journalctl --user -u hermes-dashboard -f
```

### Windows 自启动

双击 `%USERPROFILE%\.hermes\dashboard-v4\start.bat`，或创建任务计划程序开机启动。

---

## 🔧 配置

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `HERMES_HOME` | `~/.hermes` | Hermes Agent 数据目录（包含 usage.db / state.db）|
| `HERMES_DASHBOARD_PORT` | `8654` | 监听端口 |
| `HERMES_DASHBOARD_HOST` | `127.0.0.1` | 监听地址（改为 `0.0.0.0` 可局域网访问）|

### 前置要求

Dashboard 需要读取 Hermes Agent 生成的 SQLite 数据库：

```
~/.hermes/
├── usage.db       # API 调用日志
└── state.db       # 会话状态
```

> 这些文件由 Hermes Agent **自动生成**，无需手动创建。

---

## 🛠 从源码开发

### 构建

```bash
git clone https://github.com/Moritz230127/hermes-dashboard-v4.git
cd hermes-dashboard-v4

# 构建 server（Rust）
cargo build --release --package dashboard-server

# 构建前端（WASM，需要 dioxus-cli）
cargo install dioxus-cli
cd frontend
dx build --release
```

### 项目结构

```
hermes-dashboard-v4/
├── server/                 # Rust Axum 后端
│   ├── src/main.rs         # 入口：路由、启动、CORS
│   ├── src/api/            # API 端点
│   │   ├── data.rs         # GET /api/data — 主数据聚合
│   │   ├── sessions.rs     # 会话管理 CRUD
│   │   ├── alerts.rs       # 告警逻辑
│   │   ├── health.rs       # 健康检查
│   │   ├── models.rs       # 模型统计
│   │   └── historical.rs   # 历史数据
│   └── src/db/             # 数据库层
│       ├── usage.rs        # usage.db 查询
│       ├── state.rs        # state.db 查询
│       └── pool_rotator.rs # API Key 轮询状态
├── frontend/               # Dioxus WASM 前端
│   └── src/components/     # 统计卡片、图表、会话列表
├── dist/index.html         # 仪表板 HTML（Chart.js）
├── scripts/                # 运维脚本
│   ├── install.sh          # Linux/macOS 安装
│   ├── install.ps1         # Windows 安装
│   ├── uninstall.sh        # Linux/macOS 安全卸载
│   ├── uninstall.ps1       # Windows 安全卸载
│   ├── kill_session.py     # 进程终止
│   └── compress_session.py # 会话压缩
└── .github/workflows/      # CI：三平台构建发布
```

### API 端点

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/data?model=__all__` | 全量数据（今日、历史、会话、告警）|
| GET | `/api/health` | 健康检查 |
| GET | `/api/models` | 模型列表及汇总 |
| GET | `/api/alerts` | 告警列表 |
| GET | `/api/historical` | 历史数据（30 天）|
| GET | `/api/sessions/running` | 运行中会话 |
| POST | `/api/sessions/stop/{id}` | 停止特定会话 |
| POST | `/api/sessions/stop-others` | 停止其他会话（保留最新）|
| POST | `/api/sessions/mark-compression` | 标记压缩 |
| POST | `/api/memory/cleanup` | 清理 7 天前旧会话 |

---

## 📸 截图

> 仪表板包含：实时统计卡片、调用量趋势图、小时级热力图、会话列表、模型分布、API Key 状态

---

## 📄 许可

MIT License

---

<div align="center">
<sub>Built with ❤️ for the Hermes Agent ecosystem</sub>
</div>
