# 系统 Git 配置集成

这个文档说明了 `cargo-lpatch` 如何完全使用系统的 Git 配置。

## 更改说明

本次更新确保所有 Git 操作都完全依赖系统的 Git 配置，而不是使用自定义的认证方式。

### 主要改进

1. **优先使用系统默认凭据**
   - `Cred::default()` - 使用系统配置的默认认证方式
   - 这会自动调用系统配置的 credential helper

2. **系统 SSH 配置集成**
   - 优先使用系统 SSH Agent (`ssh-agent`)
   - 按系统标准顺序查找 SSH 密钥文件 (`~/.ssh/`)
   - 支持所有标准 SSH 密钥类型 (RSA, ECDSA, Ed25519, DSA)

3. **系统 Git Credential Helper 支持**
   - 使用 `git2::Config::open_default()` 读取系统配置
   - 调用 `Cred::credential_helper()` 使用系统配置的 credential helper
   - 支持系统配置的用户名和密码存储

4. **系统 SSL/TLS 配置遵循**
   - 遵循 `http.sslVerify` 系统配置
   - 支持自定义 CA 证书路径
   - 提供清晰的配置指导

5. **调试信息增强**
   - 在每次 Git 操作前显示系统配置信息
   - 包括用户名、邮箱、credential helper 等
   - 帮助用户了解当前使用的配置

## 使用的系统配置

### 认证配置

```bash
# 查看当前配置
git config --list

# 配置 credential helper
git config --global credential.helper store
# 或者
git config --global credential.helper manager-core

# 配置用户信息
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

### SSH 配置

```bash
# 检查 SSH agent
ssh-add -l

# 添加 SSH 密钥到 agent
ssh-add ~/.ssh/id_rsa

# 测试 SSH 连接
ssh -T git@github.com
```

### SSL/TLS 配置

```bash
# 启用 SSL 验证（推荐）
git config --global http.sslVerify true

# 如需禁用 SSL 验证（不推荐）
git config --global http.sslVerify false

# 配置自定义 CA 证书
git config --global http.sslCAInfo /path/to/certificate.pem
```

## 认证优先级

程序按以下优先级尝试认证：

1. **系统默认凭据** (`Cred::default()`)
   - 自动使用系统配置的认证方式

2. **SSH 认证**（用于 SSH URLs）
   - SSH Agent 中的密钥
   - 系统标准路径的 SSH 密钥文件

3. **HTTPS 认证**（用于 HTTPS URLs）
   - 系统配置的 Git credential helper
   - 环境变量（向后兼容）

4. **环境变量**（向后兼容）
   - `GIT_USERNAME` / `GIT_PASSWORD`
   - `GIT_TOKEN` / `GITHUB_TOKEN`

## 故障排除

如果遇到认证问题，程序会提供详细的解决方案提示：

1. **SSH 问题**
   - 检查 SSH 密钥配置
   - 验证 SSH agent 状态
   - 测试 SSH 连接

2. **HTTPS 问题**
   - 配置 Git credential helper
   - 验证系统 Git 配置
   - 检查环境变量设置

3. **证书问题**
   - 检查 SSL 验证设置
   - 配置自定义 CA 证书
   - 根据需要调整 SSL 策略

## 调试输出

程序会显示以下调试信息：

```
🔍 System Git Configuration:
  👤 User name: Your Name
  📧 User email: your.email@example.com
  🔑 Credential helper: store
  🔒 SSL verify: true
```

这有助于验证系统配置是否正确设置。
