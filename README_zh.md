# cargo-lpatch

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

`cargo-lpatch` 是一个 Cargo 插件，用于自动克隆依赖项并设置本地补丁（local patches），方便开发者在本地调试和修改依赖库。

## 功能特性

- 🔍 **自动查询**: 从 crates.io 查询 crate 的仓库地址
- 📦 **Git 支持**: 支持直接使用 Git URL（https, ssh, git://）
- 🔧 **自动配置**: 自动创建和更新 `.cargo/config.toml` 文件
- 📁 **灵活目录**: 支持自定义克隆目录
- 🔄 **增量更新**: 已存在的仓库自动 pull 最新更改
- ⚡ **进度显示**: 显示克隆和拉取进度

## 安装

### 从源码安装

```bash
cargo install --git https://github.com/rcore-os/cargo-lpatch
```

### 本地构建

```bash
git clone https://github.com/rcore-os/cargo-lpatch
cd cargo-lpatch
cargo install --path .
```

## 使用方法

### 基本用法

为 crates.io 上的包创建本地补丁：

```bash
cargo lpatch --name serde
```

这将：

1. 查询 crates.io 获取 `serde` 的仓库地址
2. 将仓库克隆到 `crates/serde/` 目录
3. 在 `.cargo/config.toml` 中添加本地补丁配置

### 自定义克隆目录

```bash
cargo lpatch --name tokio --dir my-dependencies
```

### 使用 Git URL

直接使用仓库地址：

```bash
# HTTPS
cargo lpatch --name https://github.com/serde-rs/serde.git

# SSH
cargo lpatch --name git@github.com:tokio-rs/tokio.git

# Git protocol
cargo lpatch --name git://github.com/clap-rs/clap.git
```

### 命令选项

```
cargo lpatch [OPTIONS] --name <CRATE_NAME>

OPTIONS:
  -n, --name <CRATE_NAME>  要补丁的 crate 名称（可以是 crate 名或 git URL）
  -d, --dir <DIRECTORY>    克隆目录 [默认: crates]
  -h, --help              显示帮助信息
```

## 工作原理

1. **依赖解析**:
   - 如果输入是 Git URL，直接使用
   - 如果是 crate 名称，查询 crates.io API 获取仓库地址

2. **仓库操作**:
   - 检查目标目录是否存在
   - 不存在则克隆仓库
   - 存在则拉取最新更改

3. **配置更新**:
   - 创建或读取 `.cargo/config.toml`
   - 添加 `[patch.crates-io]` 条目
   - 保存配置文件

## 生成的配置示例

运行命令后，您的 `.cargo/config.toml` 将包含类似内容：

```toml
[patch.crates-io.serde]
path = "crates/serde"

[patch.crates-io.tokio]
path = "my-dependencies/tokio"
```

## 使用场景

### 调试依赖问题

```bash
cargo lpatch --name problematic-crate
# 在 crates/problematic-crate 中添加调试信息
cargo build  # 使用本地版本
```

### 贡献开源项目

```bash
cargo lpatch --name target-crate
cd crates/target-crate
# 进行修改并测试
git checkout -b my-feature
# 提交更改并创建 PR
```

### 验证补丁

```bash
cargo lpatch --name some-crate
# 应用您的补丁到 crates/some-crate
cargo test  # 使用补丁版本运行测试
```

## 注意事项

- 确保有网络连接用于查询 crates.io 和克隆仓库
- Git 必须安装并在 PATH 中可用
- 对于私有仓库，需要配置相应的 Git 凭据
- 本地更改会影响所有使用该依赖的项目

## 移除补丁

要移除本地补丁：

1. 从 `.cargo/config.toml` 中删除相应条目
2. （可选）删除克隆的目录

## 故障排除

### SSL 证书错误

如果遇到证书验证问题，请检查您的 Git 配置：

```bash
git config --global http.sslVerify true
```

### 认证失败

对于私有仓库，确保配置了正确的 SSH 密钥或访问令牌。

### 网络问题

检查网络连接和防火墙设置。

## 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

## 许可证

本项目采用 MIT OR Apache-2.0 双重许可证。详见 [LICENSE-MIT](LICENSE-MIT) 和 [LICENSE-APACHE](LICENSE-APACHE)。
