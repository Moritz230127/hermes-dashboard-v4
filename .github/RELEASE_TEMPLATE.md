## 下载

| 平台 | 下载 | 一键安装 |
|------|------|---------|
| **Linux** (x86_64) | [`dashboard-v4-x86_64-unknown-linux-gnu.tar.gz`][linux] | `curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh \| bash` |
| **macOS** (Intel) | [`dashboard-v4-x86_64-apple-darwin.tar.gz`][mac-intel] | `curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh \| bash` |
| **macOS** (Apple Silicon) | [`dashboard-v4-aarch64-apple-darwin.tar.gz`][mac-arm] | `curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh \| bash` |
| **Windows** (x86_64) | [`dashboard-v4-x86_64-pc-windows-msvc.zip`][win] | `irm https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.ps1 \| iex` |

[linux]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.2/dashboard-v4-x86_64-unknown-linux-gnu.tar.gz
[mac-intel]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.2/dashboard-v4-x86_64-apple-darwin.tar.gz
[mac-arm]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.2/dashboard-v4-aarch64-apple-darwin.tar.gz
[win]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.2/dashboard-v4-x86_64-pc-windows-msvc.zip

### 包内容

```
dashboard-v4/
├── dashboard-server          # Rust 二进制 (~5MB)
├── dist/index.html           # 仪表板页面
└── scripts/
    ├── install.sh            # Linux/macOS 安装脚本
    ├── install.ps1           # Windows 安装脚本
    ├── uninstall.sh          # Linux/macOS 卸载脚本
    ├── uninstall.ps1         # Windows 卸载脚本
    ├── kill_session.py       # 进程终止
    └── compress_session.py   # 会话压缩
```

### ⚠️ 免责声明

本面板仅在 **Arch Linux (Wayland, niri)** 环境下经过完整测试。
**Windows** 和 **macOS** 为 CI 自动构建，未经人工验证。
如果遇到问题，欢迎提交 [Issue](https://github.com/Moritz230127/hermes-dashboard-v4/issues)。

## 用法

```bash
# 启动
dashboard-server

# 浏览器打开
open http://localhost:8654

# systemd 自启动 (Linux)
systemctl --user start hermes-dashboard
systemctl --user enable hermes-dashboard
journalctl --user -u hermes-dashboard -f

# GUI 双击 (Windows)
%USERPROFILE%\.hermes\dashboard-v4\start.bat
```

## 从源码构建

```bash
git clone --depth=1 https://github.com/Moritz230127/hermes-dashboard-v4.git
cd hermes-dashboard-v4
cargo build --release --package dashboard-server
./target/release/dashboard-server
```
