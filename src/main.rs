use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use std::fs;
use std::path::PathBuf;
use url::Url;

mod cargo_toml;
mod config;
mod crates_io;
mod git;
mod workspace;

#[cfg(test)]
mod test_suite;

use cargo_toml::{CargoToml, DependencyType};
use config::CargoConfig;
use crates_io::CratesIoClient;
use git::GitOperations;
use workspace::WorkspaceDetector;

#[derive(Debug, Clone)]
pub struct CrateInfo {
    pub name: String,
    pub repository_url: String,
    pub is_git_ref: bool,
    pub original_git_url: Option<String>, // 存储原始的 git URL 用于 patch 配置
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
                        .required(false),
                )
                .arg(
                    Arg::new("dir")
                        .long("dir")
                        .short('d')
                        .value_name("DIRECTORY")
                        .help("Directory to clone the crate into")
                        .default_value("crates"),
                )
                .arg(
                    Arg::new("analyze")
                        .long("analyze")
                        .short('a')
                        .help("Analyze Cargo.toml dependencies and show their types")
                        .action(clap::ArgAction::SetTrue),
                ),
        )
        .get_matches();

    if let Some(lpatch_matches) = matches.subcommand_matches("lpatch") {
        let name = lpatch_matches.get_one::<String>("name");
        let dir = lpatch_matches.get_one::<String>("dir").unwrap();
        let analyze = lpatch_matches.get_flag("analyze");

        if analyze {
            analyze_dependencies().await?;
        } else if let Some(name) = name {
            run_lpatch(name, dir).await?;
        } else {
            // 如果没有提供 name 且没有 analyze，显示帮助
            println!("Error: Either --name or --analyze must be specified.");
            println!("Use --help for more information.");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn analyze_dependencies() -> Result<()> {
    println!("🔍 Analyzing Cargo.toml dependencies...");

    let cargo_toml = CargoToml::find_and_load().context("Failed to find and load Cargo.toml")?;

    let all_deps = cargo_toml.get_all_dependencies();

    if all_deps.is_empty() {
        println!("📦 No dependencies found in Cargo.toml");
        return Ok(());
    }

    println!("📦 Found {} dependencies:", all_deps.len());
    println!();

    // 按类型分组显示
    let version_deps = cargo_toml.get_version_dependencies();
    let git_deps = cargo_toml.get_git_dependencies();
    let path_deps = cargo_toml.get_path_dependencies();

    if !version_deps.is_empty() {
        println!(
            "🌐 Version dependencies (from crates.io): {}",
            version_deps.len()
        );
        for dep in &version_deps {
            if let DependencyType::Version { version } = &dep.dep_type {
                println!("  📋 {} = \"{}\"", dep.name, version);
            }
        }
        println!();
    }

    if !git_deps.is_empty() {
        println!("🔗 Git dependencies: {}", git_deps.len());
        for dep in &git_deps {
            if let DependencyType::Git {
                git,
                branch,
                tag,
                rev,
            } = &dep.dep_type
            {
                print!("  🌿 {} = {{ git = \"{}\"", dep.name, git);
                if let Some(branch) = branch {
                    print!(", branch = \"{}\"", branch);
                }
                if let Some(tag) = tag {
                    print!(", tag = \"{}\"", tag);
                }
                if let Some(rev) = rev {
                    print!(", rev = \"{}\"", rev);
                }
                println!(" }}");
            }
        }
        println!();
    }

    if !path_deps.is_empty() {
        println!("📁 Path dependencies: {}", path_deps.len());
        for dep in &path_deps {
            if let DependencyType::Path { path } = &dep.dep_type {
                println!("  📂 {} = {{ path = \"{}\" }}", dep.name, path);
            }
        }
        println!();
    }

    println!("💡 Use 'cargo lpatch --name <CRATE_NAME>' to patch a specific dependency");

    Ok(())
}

async fn run_lpatch(name: &str, dir: &str) -> Result<()> {
    println!("Creating local patch for: {name}");
    println!("Clone directory: {dir}");

    // 尝试从 Cargo.toml 分析依赖信息
    let dependency_info = if let Ok(cargo_toml) = CargoToml::find_and_load() {
        cargo_toml.find_dependency(name)
    } else {
        None
    };

    // 根据依赖信息或用户输入确定 crate 信息
    let crate_info = if let Some(dep_info) = dependency_info {
        println!("📦 Found dependency '{}' in Cargo.toml", dep_info.name);

        match &dep_info.dep_type {
            DependencyType::Git {
                git,
                branch,
                tag,
                rev,
            } => {
                println!("🔗 Git dependency detected: {}", git);
                if let Some(branch) = branch {
                    println!("  🌿 Branch: {branch}");
                }
                if let Some(tag) = tag {
                    println!("  🏷️  Tag: {tag}");
                }
                if let Some(rev) = rev {
                    println!("  🔄 Revision: {rev}");
                }

                CrateInfo {
                    name: dep_info.name.clone(),
                    repository_url: git.clone(),
                    is_git_ref: true,
                    original_git_url: Some(git.clone()),
                }
            }
            DependencyType::Version { version } => {
                println!("🌐 Version dependency detected: {}", version);
                println!("🔍 Querying crates.io for repository URL...");

                let client = CratesIoClient::new();
                let repo_url = client
                    .get_repository_url(&dep_info.name)
                    .await
                    .with_context(|| {
                        format!("Failed to get repository URL for crate '{}'", dep_info.name)
                    })?;

                CrateInfo {
                    name: dep_info.name.clone(),
                    repository_url: repo_url,
                    is_git_ref: false,
                    original_git_url: None,
                }
            }
            DependencyType::Path { path } => {
                return Err(anyhow!(
                    "Path dependency '{}' at '{}' cannot be patched as it's already local",
                    dep_info.name,
                    path
                ));
            }
        }
    } else {
        // 回退到原有逻辑：检查是否是 git URL
        if is_git_url(name) {
            println!("🔗 Direct git URL detected");
            CrateInfo {
                name: extract_crate_name_from_git_url(name)?,
                repository_url: name.to_string(),
                is_git_ref: true,
                original_git_url: Some(name.to_string()),
            }
        } else {
            // 从 crates.io 查询
            println!("🌐 Querying crates.io for crate: {name}");
            let client = CratesIoClient::new();
            let repo_url = client
                .get_repository_url(name)
                .await
                .with_context(|| format!("Failed to get repository URL for crate '{name}'"))?;

            CrateInfo {
                name: name.to_string(),
                repository_url: repo_url,
                is_git_ref: false,
                original_git_url: None,
            }
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

    // 检测 workspace 并找到正确的 crate 路径
    let actual_crate_path = match WorkspaceDetector::find_crate_path(&clone_path, &crate_info.name)
    {
        Ok(path) => {
            if path != clone_path {
                println!(
                    "🎯 Found crate '{}' in workspace at: {}",
                    crate_info.name,
                    path.display()
                );
            }
            path
        }
        Err(e) => {
            println!("⚠️  Could not locate crate in repository: {}", e);
            println!("📋 Available crates in repository:");

            match WorkspaceDetector::list_workspace_crates(&clone_path) {
                Ok(crates) => {
                    if crates.is_empty() {
                        println!("  (No crates found)");
                        return Err(e);
                    } else {
                        for (name, path) in &crates {
                            let relative_path =
                                path.strip_prefix(&clone_path).unwrap_or(path).display();
                            println!("  📦 {} ({})", name, relative_path);
                        }

                        // 尝试找到名称相似的 crate
                        if let Some((similar_name, similar_path)) =
                            find_similar_crate(&crate_info.name, &crates)
                        {
                            println!("💡 Did you mean '{}'? Using it instead.", similar_name);
                            similar_path
                        } else {
                            return Err(anyhow!("Could not find crate '{}' in the repository. Please check the available crates above.", crate_info.name));
                        }
                    }
                }
                Err(list_err) => {
                    println!("  ❌ Failed to list crates: {}", list_err);
                    return Err(e);
                }
            }
        }
    };

    // 更新或创建 .cargo/config.toml
    let mut cargo_config = CargoConfig::load_or_create()?;

    // 根据依赖类型选择正确的 patch 源
    if let Some(original_git_url) = &crate_info.original_git_url {
        // Git 依赖使用原始的 git URL 作为 patch 源
        cargo_config.add_patch_with_source(
            &crate_info.name,
            &actual_crate_path,
            original_git_url,
        )?;
    } else {
        // 版本依赖使用 crates-io 作为 patch 源
        cargo_config.add_patch(&crate_info.name, &actual_crate_path)?;
    }

    cargo_config.save()?;

    println!(
        "✅ Successfully set up local patch for '{}'",
        crate_info.name
    );
    println!("📁 Cloned to: {}", clone_path.display());
    if actual_crate_path != clone_path {
        println!("🎯 Crate located at: {}", actual_crate_path.display());
    }
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

/// 在 crate 列表中查找与目标名称相似的 crate
fn find_similar_crate(
    target_name: &str,
    crates: &[(String, PathBuf)],
) -> Option<(String, PathBuf)> {
    // 首先尝试精确匹配（不区分大小写）
    for (name, path) in crates {
        if name.to_lowercase() == target_name.to_lowercase() {
            return Some((name.clone(), path.clone()));
        }
    }

    // 然后尝试包含匹配
    for (name, path) in crates {
        if name.to_lowercase().contains(&target_name.to_lowercase())
            || target_name.to_lowercase().contains(&name.to_lowercase())
        {
            return Some((name.clone(), path.clone()));
        }
    }

    // 最后尝试前缀匹配
    for (name, path) in crates {
        if name.to_lowercase().starts_with(&target_name.to_lowercase())
            || target_name.to_lowercase().starts_with(&name.to_lowercase())
        {
            return Some((name.clone(), path.clone()));
        }
    }

    None
}
