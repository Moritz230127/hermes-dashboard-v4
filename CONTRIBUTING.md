# 贡献指南

感谢你对 Hermes Dashboard V4 的关注！

## 报告 Bug

- 使用 [GitHub Issues](https://github.com/Moritz230127/hermes-dashboard-v4/issues) 提交
- 请包含：系统版本、错误日志、复现步骤

## 提交 PR

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feat/my-feature`
3. 提交前确保编译通过：
   ```bash
   cargo build --release --package dashboard-server
   cargo clippy --package dashboard-server -- -D warnings
   ```
4. 提交到你的分支并创建 Pull Request

## 开发环境

- Rust 稳定版（rustup 安装）
- 可选：dioxus-cli（前端开发）

## 代码风格

- 遵循 Rust 官方风格：`cargo fmt`
- clippy 零警告
- 添加必要的注释和文档
