use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Workspace 配置结构
#[derive(Debug, Deserialize)]
pub struct WorkspaceConfig {
    pub members: Option<Vec<String>>,
    pub exclude: Option<Vec<String>>,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, toml::Value>,
}

/// 根 Cargo.toml 结构（用于检测 workspace）
#[derive(Debug, Deserialize)]
pub struct RootCargoToml {
    pub workspace: Option<WorkspaceConfig>,
    pub _package: Option<toml::Value>,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, toml::Value>,
}

/// 包配置结构
#[derive(Debug, Deserialize)]
pub struct PackageConfig {
    pub name: String,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, toml::Value>,
}

/// 包 Cargo.toml 结构（用于获取包名）
#[derive(Debug, Deserialize)]
pub struct PackageCargoToml {
    pub package: Option<PackageConfig>,
    #[serde(flatten)]
    pub _other: std::collections::HashMap<String, toml::Value>,
}

/// Workspace 检测和处理工具
pub struct WorkspaceDetector;

impl WorkspaceDetector {
    /// 检测指定路径是否是 workspace，如果是则返回目标 crate 的路径
    pub fn find_crate_path(repo_path: &Path, crate_name: &str) -> Result<PathBuf> {
        let cargo_toml_path = repo_path.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Err(anyhow!("No Cargo.toml found in repository root"));
        }

        let content = fs::read_to_string(&cargo_toml_path)
            .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;

        let root_config: RootCargoToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

        // 检查是否是 workspace
        if let Some(workspace) = root_config.workspace {
            info!("🏗️  Detected workspace structure");
            Self::find_crate_in_workspace(repo_path, crate_name, &workspace)
        } else {
            // 不是 workspace，检查是否是目标 crate
            if Self::is_target_crate(repo_path, crate_name)? {
                info!("📦 Single crate repository matches target '{crate_name}'");
                Ok(repo_path.to_path_buf())
            } else {
                Err(anyhow!(
                    "Repository is not a workspace and does not contain crate '{}'",
                    crate_name
                ))
            }
        }
    }

    /// 在 workspace 中查找目标 crate
    fn find_crate_in_workspace(
        repo_path: &Path,
        crate_name: &str,
        workspace: &WorkspaceConfig,
    ) -> Result<PathBuf> {
        let empty_vec = vec![];
        let members = workspace.members.as_ref().unwrap_or(&empty_vec);
        let exclude = workspace.exclude.as_ref().unwrap_or(&empty_vec);

        info!("  📂 Workspace members: {members:?}");
        if !exclude.is_empty() {
            info!("  🚫 Excluded: {exclude:?}");
        }

        // 收集所有潜在的 crate 路径
        let mut candidate_paths = Vec::new();

        for member in members {
            let member_paths = Self::expand_glob_pattern(repo_path, member)?;
            candidate_paths.extend(member_paths);
        }

        // 过滤掉被排除的路径
        for exclude_pattern in exclude {
            let exclude_paths = Self::expand_glob_pattern(repo_path, exclude_pattern)?;
            candidate_paths.retain(|path| !exclude_paths.contains(path));
        }

        // 在候选路径中查找目标 crate
        for candidate_path in candidate_paths {
            if Self::is_target_crate(&candidate_path, crate_name)? {
                info!(
                    "  ✅ Found crate '{}' at: {}",
                    crate_name,
                    candidate_path.display()
                );
                return Ok(candidate_path);
            }
        }

        Err(anyhow!(
            "Crate '{}' not found in workspace members",
            crate_name
        ))
    }

    /// 展开 glob 模式（简单实现）
    fn expand_glob_pattern(base_path: &Path, pattern: &str) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        if pattern.contains('*') {
            // 处理通配符模式，如 "crates/*"
            let pattern_path = base_path.join(pattern);
            let parent = pattern_path.parent().unwrap_or(base_path);

            if parent.exists() && parent.is_dir() {
                for entry in fs::read_dir(parent)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_dir() {
                        // 简单的通配符匹配：只处理 "dir/*" 的情况
                        if pattern.ends_with("/*") {
                            paths.push(path);
                        }
                    }
                }
            }
        } else {
            // 直接路径
            let direct_path = base_path.join(pattern);
            if direct_path.exists() {
                paths.push(direct_path);
            }
        }

        Ok(paths)
    }

    /// 检查指定路径是否包含目标 crate
    fn is_target_crate(path: &Path, crate_name: &str) -> Result<bool> {
        let cargo_toml_path = path.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&cargo_toml_path)
            .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;

        let package_config: PackageCargoToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

        if let Some(package) = package_config.package {
            Ok(package.name == crate_name)
        } else {
            Ok(false)
        }
    }

    /// 列出 workspace 中的所有 crate
    pub fn list_workspace_crates(repo_path: &Path) -> Result<Vec<(String, PathBuf)>> {
        let cargo_toml_path = repo_path.join("Cargo.toml");

        if !cargo_toml_path.exists() {
            return Ok(vec![]);
        }

        let content = fs::read_to_string(&cargo_toml_path)
            .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;

        let root_config: RootCargoToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

        let mut crates = Vec::new();

        if let Some(workspace) = root_config.workspace {
            let empty_vec = vec![];
            let members = workspace.members.as_ref().unwrap_or(&empty_vec);
            let exclude = workspace.exclude.as_ref().unwrap_or(&empty_vec);

            // 收集所有候选路径
            let mut candidate_paths = Vec::new();
            for member in members {
                let member_paths = Self::expand_glob_pattern(repo_path, member)?;
                candidate_paths.extend(member_paths);
            }

            // 过滤排除的路径
            for exclude_pattern in exclude {
                let exclude_paths = Self::expand_glob_pattern(repo_path, exclude_pattern)?;
                candidate_paths.retain(|path| !exclude_paths.contains(path));
            }

            // 获取每个 crate 的名称
            for candidate_path in candidate_paths {
                if let Ok(name) = Self::get_crate_name(&candidate_path) {
                    crates.push((name, candidate_path));
                }
            }
        } else {
            // 单个 crate
            if let Ok(name) = Self::get_crate_name(repo_path) {
                crates.push((name, repo_path.to_path_buf()));
            }
        }

        Ok(crates)
    }

    /// 获取指定路径的 crate 名称
    fn get_crate_name(path: &Path) -> Result<String> {
        let cargo_toml_path = path.join("Cargo.toml");

        let content = fs::read_to_string(&cargo_toml_path)
            .with_context(|| format!("Failed to read {}", cargo_toml_path.display()))?;

        let package_config: PackageCargoToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse {}", cargo_toml_path.display()))?;

        if let Some(package) = package_config.package {
            Ok(package.name)
        } else {
            Err(anyhow!(
                "No package section found in {}",
                cargo_toml_path.display()
            ))
        }
    }
}
