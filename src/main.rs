use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use std::fs;
use std::path::PathBuf;
use url::Url;

mod config;
mod crates_io;
mod git;

#[cfg(test)]
mod test_suite;

use config::CargoConfig;
use crates_io::CratesIoClient;
use git::GitOperations;

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub repository_url: String,
    pub is_git_ref: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("cargo-lpatch")
        .about("Locally patch cargo dependencies by cloning and setting up local patches")
        .subcommand(
            Command::new("lpatch")
                .about("Create a local patch for a dependency")
                .arg(
                    Arg::new("name")
                        .long("name")
                        .short('n')
                        .value_name("CRATE_NAME")
                        .help("Name of the crate to patch (can be crate name or git URL)")
                        .required(true),
                )
                .arg(
                    Arg::new("dir")
                        .long("dir")
                        .short('d')
                        .value_name("DIRECTORY")
                        .help("Directory to clone the crate into")
                        .default_value("crates"),
                ),
        )
        .get_matches();

    if let Some(lpatch_matches) = matches.subcommand_matches("lpatch") {
        let name = lpatch_matches.get_one::<String>("name").unwrap();
        let dir = lpatch_matches.get_one::<String>("dir").unwrap();

        run_lpatch(name, dir).await?;
    }

    Ok(())
}

async fn run_lpatch(name: &str, dir: &str) -> Result<()> {
    println!("Creating local patch for: {name}");
    println!("Clone directory: {dir}");

    // 检查是否是 git URL
    let crate_info = if is_git_url(name) {
        CrateInfo {
            name: extract_crate_name_from_git_url(name)?,
            repository_url: name.to_string(),
            is_git_ref: true,
        }
    } else {
        // 从 crates.io 查询
        let client = CratesIoClient::new();
        let repo_url = client
            .get_repository_url(name)
            .await
            .with_context(|| format!("Failed to get repository URL for crate '{}'", name))?;

        CrateInfo {
            name: name.to_string(),
            repository_url: repo_url,
            is_git_ref: false,
        }
    };

    println!("Repository URL: {}", crate_info.repository_url);

    // 创建目标目录
    let target_dir = PathBuf::from(dir);
    if !target_dir.exists() {
        fs::create_dir_all(&target_dir)
            .with_context(|| format!("Failed to create directory '{}'", dir))?;
    }

    // 克隆仓库
    let git_ops = GitOperations::new();
    let clone_path = target_dir.join(&crate_info.name);

    if clone_path.exists() {
        println!(
            "Directory '{}' already exists, pulling latest changes...",
            clone_path.display()
        );
        git_ops.pull(&clone_path)?;
    } else {
        println!("Cloning repository to '{}'...", clone_path.display());
        git_ops.clone(&crate_info.repository_url, &clone_path)?;
    }

    // 更新或创建 .cargo/config.toml
    let mut cargo_config = CargoConfig::load_or_create()?;
    cargo_config.add_patch(&crate_info.name, &clone_path)?;
    cargo_config.save()?;

    println!(
        "✅ Successfully set up local patch for '{}'",
        crate_info.name
    );
    println!("📁 Cloned to: {}", clone_path.display());
    println!("⚙️  Updated .cargo/config.toml with local patch configuration");

    Ok(())
}

fn is_git_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("git://")
        || s.starts_with("ssh://")
        || s.contains("git@")
}

fn extract_crate_name_from_git_url(git_url: &str) -> Result<String> {
    let url = if git_url.starts_with("git@") {
        // 转换 SSH URL 格式
        let parts: Vec<&str> = git_url.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid git SSH URL format"));
        }
        format!("https://{}/{}", parts[0].replace("git@", ""), parts[1])
    } else {
        git_url.to_string()
    };

    let parsed_url = Url::parse(&url).with_context(|| format!("Failed to parse URL: {}", url))?;

    let path = parsed_url.path();
    let name = path
        .trim_start_matches('/')
        .trim_end_matches(".git")
        .split('/')
        .next_back()
        .ok_or_else(|| anyhow!("Could not extract crate name from URL"))?;

    Ok(name.to_string())
}
