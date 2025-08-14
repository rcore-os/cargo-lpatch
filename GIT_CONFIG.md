# Git 配置使用指南

## 概述

`cargo-lpatch` 现在已完全集成主机的 Git 配置，支持多种认证方式，兼容 Linux、Windows 和 macOS。

## 支持的认证方式

### 1. SSH 密钥认证（推荐）

#### SSH Agent

工具会自动尝试使用 SSH agent 中加载的密钥：

```bash
# 检查 SSH agent 状态
ssh-add -l

# 添加密钥到 SSH agent
ssh-add ~/.ssh/id_rsa
ssh-add ~/.ssh/id_ed25519
```

#### 本地 SSH 密钥文件

工具会自动搜索以下密钥文件：

- `~/.ssh/id_rsa` / `~/.ssh/id_rsa.pub`
- `~/.ssh/id_ed25519` / `~/.ssh/id_ed25519.pub`
- `~/.ssh/id_ecdsa` / `~/.ssh/id_ecdsa.pub`
- `~/.ssh/id_dsa` / `~/.ssh/id_dsa.pub`

### 2. HTTPS 认证

#### Git Credential Helper

工具会自动使用 Git 的 credential helper：

```bash
# 查看当前配置的 credential helper
git config --global credential.helper

# 配置 credential helper（Linux/macOS）
git config --global credential.helper store

# 配置 credential helper（Windows）
git config --global credential.helper manager-core
```

#### 环境变量

设置以下环境变量进行认证：

```bash
# 基本用户名密码
export GIT_USERNAME="your-username"
export GIT_PASSWORD="your-password"

# 或使用 token
export GIT_USERNAME="your-username" 
export GIT_TOKEN="your-token"
export GITHUB_TOKEN="your-github-token"
```

### 3. 默认 Git 配置

工具会自动读取 Git 的默认配置，包括：

- 用户信息 (`user.name`, `user.email`)
- 存储的凭据
- 证书设置

## 使用示例

### SSH URL

```bash
# 使用 SSH URL（需要配置 SSH 密钥）
cargo lpatch --name git@github.com:user/repo.git

# SSH URL 另一种格式
cargo lpatch --name ssh://git@github.com/user/repo.git
```

### HTTPS URL

```bash
# 使用 HTTPS URL
cargo lpatch --name https://github.com/user/repo.git

# crates.io 的 crate（自动查询仓库 URL）
cargo lpatch --name serde
```

## 故障排除

### SSH 认证问题

1. **检查 SSH 密钥**：

   ```bash
   ls -la ~/.ssh/
   ```

2. **测试 GitHub 连接**：

   ```bash
   ssh -T git@github.com
   ```

3. **检查 SSH agent**：

   ```bash
   ssh-add -l
   ```

4. **添加密钥到 SSH agent**：

   ```bash
   ssh-add ~/.ssh/id_rsa
   ```

### HTTPS 认证问题

1. **配置 credential helper**：

   ```bash
   git config --global credential.helper store
   ```

2. **手动认证一次**：

   ```bash
   git clone https://github.com/user/repo.git /tmp/test
   # 输入用户名和密码/token
   rm -rf /tmp/test
   ```

3. **使用 Personal Access Token**：
   - GitHub: 创建 PAT 并设置为 `GIT_PASSWORD`
   - GitLab: 创建 PAT 并设置为 `GIT_PASSWORD`

### 证书问题

如果遇到 SSL 证书错误：

```bash
# 临时跳过证书验证（不推荐用于生产）
git config --global http.sslVerify false

# 或配置证书路径
git config --global http.sslCAInfo /path/to/certificate.pem
```

## 跨平台兼容性

### Linux

- SSH 密钥路径: `~/.ssh/`
- Git 配置路径: `~/.gitconfig`

### Windows

- SSH 密钥路径: `%USERPROFILE%\.ssh\`
- Git 配置路径: `%USERPROFILE%\.gitconfig`

### macOS

- SSH 密钥路径: `~/.ssh/`
- Git 配置路径: `~/.gitconfig`
- 支持 Keychain 集成

## 调试信息

运行 `cargo lpatch` 时，您会看到详细的认证过程：

```
🔑 Authenticating for URL: https://github.com/user/repo.git
✅ Using default Git credentials
```

或

```
🔑 Authenticating for URL: ssh://git@github.com/user/repo.git
🔑 Trying SSH authentication for user: git
✅ Using SSH agent
```

这些信息帮助您了解使用了哪种认证方式。
