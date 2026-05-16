## 下载

| 平台 | 下载 | 一键安装 |
|------|------|---------|
| **Linux** (x86_64) | [`dashboard-v4-x86_64-unknown-linux-gnu.tar.gz`][linux] | `curl -fsSL https://git.io/hermes-d4 \| bash` |
| **macOS** (Intel) | [`dashboard-v4-x86_64-apple-darwin.tar.gz`][mac-intel] | `curl -fsSL https://git.io/hermes-d4 \| bash` |
| **macOS** (Apple Silicon) | [`dashboard-v4-aarch64-apple-darwin.tar.gz`][mac-arm] | `curl -fsSL https://git.io/hermes-d4 \| bash` |
| **Windows** (x86_64) | [`dashboard-v4-x86_64-pc-windows-msvc.zip`][win] | `PowerShell -c "irm https://git.io/hermes-d4 \| iex"` |

[linux]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.0/dashboard-v4-x86_64-unknown-linux-gnu.tar.gz
[mac-intel]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.0/dashboard-v4-x86_64-apple-darwin.tar.gz
[mac-arm]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.0/dashboard-v4-aarch64-apple-darwin.tar.gz
[win]: https://github.com/Moritz230127/hermes-dashboard-v4/releases/download/v4.0.0/dashboard-v4-x86_64-pc-windows-msvc.zip

### 包内容

```
dashboard-v4/
├── dashboard-server          # Rust 二进制 (~5MB)
├── dist/index.html           # 仪表板页面
└── scripts/
    ├── install.sh            # Linux/macOS 安装脚本
    ├── install.ps1           # Windows 安装脚本
    ├── kill_session.py       # 进程终止
    └── compress_session.py   # 会话压缩
```

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
