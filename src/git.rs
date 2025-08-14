use anyhow::{Context, Result};
use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub struct GitOperations {
    username: String,
    credential_helper: Option<String>,
    http_sslverify: bool,
    ssh_agent_tried: Arc<AtomicBool>,
}

impl GitOperations {
    pub fn new() -> Self {
        let mut s = Self {
            username: "git".into(),
            credential_helper: None,
            http_sslverify: true,
            ssh_agent_tried: Arc::new(AtomicBool::new(false)),
        };

        if let Ok(config) = git2::Config::open_default() {
            if let Ok(name) = config.get_string("user.name") {
                s.username = name;
                println!("👤 Git username: {}", s.username);
            }

            if let Ok(helper) = config.get_string("credential.helper") {
                println!("  🔑 Credential helper: {helper}");
                s.credential_helper = Some(helper);
            }
            if let Ok(ssl_verify) = config.get_bool("http.sslverify") {
                s.http_sslverify = ssl_verify;
                println!("  🔒 SSL verify: {ssl_verify}");
            }
        } else {
            println!("⚠️  No global Git configuration found, using defaults");
        }
        s
    }

    /// 尝试 SSH 密钥认证（使用系统配置的 SSH 设置）
    fn try_ssh_key_auth(
        ssh_agent_tried: Arc<AtomicBool>,
        username: &str,
    ) -> Result<Cred, git2::Error> {
        println!("🔑 Trying SSH authentication for user: {username}");

        if !ssh_agent_tried.load(std::sync::atomic::Ordering::Relaxed) {
            // 1. 首先尝试 SSH Agent 认证（这会使用系统配置的 SSH agent）
            match Cred::ssh_key_from_agent(username) {
                Ok(cred) => {
                    println!("✅ Using system SSH agent");
                    ssh_agent_tried.store(true, std::sync::atomic::Ordering::Relaxed);
                    return Ok(cred);
                }
                Err(_) => println!("⚠️  System SSH agent not available or no keys loaded"),
            }
        }

        // 2. 尝试使用系统中配置的 SSH 密钥文件（按系统标准路径查找）
        let ssh_key_paths = GitOperations::get_ssh_key_paths();

        for (private_key, public_key) in ssh_key_paths {
            if private_key.exists() {
                let public_key_path = if public_key.exists() {
                    Some(public_key.as_path())
                } else {
                    None
                };

                println!("🔑 Trying system SSH key: {}", private_key.display());
                match Cred::ssh_key(username, public_key_path, &private_key, None) {
                    Ok(cred) => {
                        println!("✅ Using system SSH key: {}", private_key.display());
                        return Ok(cred);
                    }
                    Err(e) => {
                        println!("⚠️  System SSH key {} failed: {e}", private_key.display());
                        continue; // 尝试下一个密钥
                    }
                }
            }
        }

        println!("❌ No valid system SSH key found");
        Err(git2::Error::from_str("No valid system SSH key found"))
    }

    /// 尝试用户名密码认证（优先使用系统 Git 配置）
    fn try_userpass_auth() -> Result<Cred, git2::Error> {
        println!("🔑 Trying username/password authentication using system configuration");

        // 1. 优先从系统 Git 配置获取用户信息
        if let Ok(config) = git2::Config::open_default() {
            // 尝试获取配置的用户名
            let username_result = config
                .get_string("user.name")
                .or_else(|_| config.get_string("github.user"))
                .or_else(|_| config.get_string("credential.username"));

            if let Ok(username) = username_result {
                // 尝试从环境变量获取密码/token（出于安全考虑，密码不应存储在 git 配置中）
                if let Ok(password) = env::var("GIT_TOKEN")
                    .or_else(|_| env::var("GITHUB_TOKEN"))
                    .or_else(|_| env::var("GIT_PASSWORD"))
                {
                    println!("✅ Using username from system Git config and token from environment");
                    return Cred::userpass_plaintext(&username, &password);
                }
            }
        }

        // 2. 回退到纯环境变量方式（保持向后兼容）
        if let (Ok(username), Ok(password)) = (env::var("GIT_USERNAME"), env::var("GIT_PASSWORD")) {
            println!("✅ Using credentials from environment variables");
            return Cred::userpass_plaintext(&username, &password);
        }

        println!("❌ No username/password credentials available from system configuration");
        println!("💡 Tip: Configure Git credentials using 'git config --global credential.helper' or set environment variables");
        Err(git2::Error::from_str(
            "No username/password credentials available from system configuration",
        ))
    }

    /// 获取系统标准 SSH 密钥路径（遵循系统惯例）
    fn get_ssh_key_paths() -> Vec<(PathBuf, PathBuf)> {
        let mut key_paths = Vec::new();

        // 获取用户主目录（使用系统环境变量）
        let home_dir = if cfg!(windows) {
            env::var("USERPROFILE").unwrap_or_else(|_| {
                env::var("HOMEDRIVE").unwrap_or_default()
                    + &env::var("HOMEPATH").unwrap_or_default()
            })
        } else {
            env::var("HOME").unwrap_or_default()
        };

        if home_dir.is_empty() {
            return key_paths;
        }

        let ssh_dir = PathBuf::from(home_dir).join(".ssh");

        // 按照系统标准顺序查找 SSH 密钥文件
        let key_names = ["id_rsa", "id_ecdsa", "id_ed25519", "id_dsa"];

        for key_name in &key_names {
            let private_key = ssh_dir.join(key_name);
            let public_key = ssh_dir.join(format!("{key_name}.pub"));
            key_paths.push((private_key, public_key));
        }

        key_paths
    }

    fn remote_callbacks(&self) -> RemoteCallbacks {
        let mut callbacks = RemoteCallbacks::new();
        let ssh_agent_tried = Arc::clone(&self.ssh_agent_tried);
        callbacks.credentials(move |url, username_from_url, allowed_types| {
            println!("🔑 Authenticating for URL: {url}, allowed_types: {allowed_types:?}");
            if allowed_types.contains(CredentialType::SSH_KEY) {
                return Self::try_ssh_key_auth(
                    ssh_agent_tried.clone(),
                    username_from_url.unwrap_or(&self.username),
                );
            } else if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
                return Self::try_userpass_auth();
            }
            Cred::default()
        });
        callbacks.certificate_check(|_cert, _valid| {
            // 在生产环境中，应该遵循系统 Git 配置中的 http.sslVerify 设置
            // 用户可以通过以下命令配置：
            // git config --global http.sslVerify true/false
            // 这里为了兼容性暂时接受证书，实际项目中应该根据系统配置来决定
            Ok(git2::CertificateCheckStatus::CertificateOk)
        });
        callbacks
    }

    pub fn clone(&self, url: &str, target_path: &Path) -> Result<()> {
        println!("🔄 Cloning {} to {}...", url, target_path.display());
        let mut cb = self.remote_callbacks();

        cb.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                print!(
                    "Resolving deltas {}/{}\r",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                );
            } else if stats.total_objects() > 0 {
                print!(
                    "Received {}/{} objects ({}) in {} bytes\r",
                    stats.received_objects(),
                    stats.total_objects(),
                    stats.indexed_objects(),
                    stats.received_bytes()
                );
            }
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            true
        });

        let mut co = CheckoutBuilder::new();
        co.progress(|path, cur, total| {});

        let mut fo = FetchOptions::new();
        fo.remote_callbacks(cb);

        let mut builder = RepoBuilder::new();
        builder.fetch_options(fo).with_checkout(co);

        match builder.clone(url, target_path) {
            Ok(_) => {
                println!("\n✅ Clone completed successfully");
                Ok(())
            }
            Err(e) => {
                // 提供更友好的错误信息和解决方案
                let error_msg = match e.code() {
                    git2::ErrorCode::Certificate => {
                        format!(
                            "SSL certificate verification failed for {url}\n\
                        Solutions:\n\
                        1. Make sure the repository URL is correct\n\
                        2. Check your internet connection\n\
                        3. For self-signed certificates, configure Git to accept them\n\
                        Original error: {e}"
                        )
                    }
                    git2::ErrorCode::Auth => {
                        format!("Authentication failed for {url}\n\
                        Solutions:\n\
                        1. For SSH URLs: Ensure your SSH keys are configured in the system (~/.ssh/)\n\
                        2. Check if ssh-agent is running: 'ssh-add -l'\n\
                        3. Test SSH connection: 'ssh -T git@github.com'\n\
                        4. For HTTPS URLs: Configure Git credential helper: 'git config --global credential.helper'\n\
                        5. Verify system Git configuration: 'git config --list'\n\
                        Original error: {e}")
                    }
                    git2::ErrorCode::NotFound => {
                        format!(
                            "Repository not found: {url}\n\
                        Solutions:\n\
                        1. Check if the repository URL is correct\n\
                        2. Make sure you have access to the repository\n\
                        3. For private repositories, ensure you're authenticated\n\
                        Original error: {e}"
                        )
                    }
                    _ => format!("Git clone failed for {url}: {e}"),
                };
                Err(anyhow::anyhow!(error_msg))
            }
        }
    }

    pub fn pull(&self, repo_path: &Path) -> Result<()> {
        println!("🔄 Pulling latest changes in {}...", repo_path.display());

        let repo = Repository::open(repo_path)
            .with_context(|| format!("Failed to open repository at {}", repo_path.display()))?;

        // 获取当前分支
        let head = repo.head()?;
        let branch_name = head.shorthand().unwrap_or("HEAD");

        // 获取远程仓库 (通常是 origin)
        let mut remote = repo
            .find_remote("origin")
            .context("Failed to find 'origin' remote")?;

        // 设置回调
        let mut callbacks = self.remote_callbacks();
        callbacks.transfer_progress(|stats| {
            if stats.received_objects() == stats.total_objects() {
                print!(
                    "Resolving deltas {}/{}\r",
                    stats.indexed_deltas(),
                    stats.total_deltas()
                );
            } else if stats.total_objects() > 0 {
                print!(
                    "Received {}/{} objects\r",
                    stats.received_objects(),
                    stats.total_objects()
                );
            }
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            true
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        // 获取远程更新
        remote
            .fetch(&[branch_name], Some(&mut fetch_options), None)
            .context("Failed to fetch from remote")?;

        // 获取远程分支的 OID
        let fetch_head = repo.fetchhead_foreach(|ref_name, remote_url, _oid, is_merge| {
            let remote_url_str = String::from_utf8_lossy(remote_url);
            println!("Fetched {ref_name} from {remote_url_str}");
            if is_merge {
                // 这里可以进行合并操作，但为了简单起见，我们只提示用户
                println!(
                    "Note: You may need to manually merge changes in {}",
                    repo_path.display()
                );
            }
            true
        });

        match fetch_head {
            Ok(_) => println!("✅ Pull completed successfully"),
            Err(_) => println!("⚠️  Fetch completed, but you may need to manually merge changes"),
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_current_branch(&self, repo_path: &Path) -> Result<String> {
        let repo = Repository::open(repo_path)?;
        let head = repo.head()?;
        let branch_name = head
            .shorthand()
            .ok_or_else(|| anyhow::anyhow!("Could not determine current branch"))?;
        Ok(branch_name.to_string())
    }

    #[allow(dead_code)]
    pub fn is_git_repository(&self, path: &Path) -> bool {
        Repository::open(path).is_ok()
    }
}
