## 下载

| 平台 | 包 | 一键安装 |
|------|-----|---------|
| **Linux** (x86_64) | `dashboard-v4-x86_64-unknown-linux-gnu.tar.gz` | `curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh \| bash` |
| **macOS** (Intel) | `dashboard-v4-x86_64-apple-darwin.tar.gz` | `curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh \| bash` |
| **macOS** (Apple Silicon) | `dashboard-v4-aarch64-apple-darwin.tar.gz` | `curl -fsSL https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.sh \| bash` |
| **Windows** (x86_64) | `dashboard-v4-x86_64-pc-windows-msvc.zip` | `PowerShell -c "irm https://raw.githubusercontent.com/Moritz230127/hermes-dashboard-v4/main/scripts/install.ps1 \| iex"` |

## 使用

```bash
# 启动
dashboard-server

# systemd (Linux)
systemctl --user start hermes-dashboard

# 浏览器打开
open http://localhost:8654
```

## 从源码构建

```bash
git clone --depth=1 https://github.com/Moritz230127/hermes-dashboard-v4.git
cd hermes-dashboard-v4
cargo build --release --package dashboard-server
./target/release/dashboard-server
```
