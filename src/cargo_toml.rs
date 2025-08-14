use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 表示一个依赖的信息
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
    pub dep_type: DependencyType,
}

/// 依赖类型
#[derive(Debug, Clone)]
pub enum DependencyType {
    /// 来自 crates.io 的版本依赖
    Version { version: String },
    /// 来自 git 仓库的依赖
    Git {
        git: String,
        branch: Option<String>,
        tag: Option<String>,
        rev: Option<String>,
    },
    /// 本地路径依赖
    Path { path: String },
}

/// 依赖的完整定义（用于解析 TOML）
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum DependencyDefinition {
    /// 简单版本字符串: dependency = "1.0"
    Simple(String),
    /// 详细配置: dependency = { version = "1.0", features = [...] }
    Detailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        git: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tag: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        path: Option<String>,
        #[serde(flatten)]
        other: HashMap<String, toml::Value>,
    },
}

/// Cargo.toml 文件的结构
#[derive(Debug, Deserialize)]
pub struct CargoToml {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<HashMap<String, DependencyDefinition>>,
    #[serde(rename = "dev-dependencies", skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<HashMap<String, DependencyDefinition>>,
    #[serde(rename = "build-dependencies", skip_serializing_if = "Option::is_none")]
    pub build_dependencies: Option<HashMap<String, DependencyDefinition>>,
    #[serde(flatten)]
    pub _other: HashMap<String, toml::Value>,
}

impl CargoToml {
    /// 从指定路径加载 Cargo.toml 文件
    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read Cargo.toml file: {}", path.display()))?;

        let cargo_toml: CargoToml = toml::from_str(&content)
            .with_context(|| format!("Failed to parse Cargo.toml file: {}", path.display()))?;

        Ok(cargo_toml)
    }

    /// 查找当前目录或父目录中的 Cargo.toml 文件
    pub fn find_and_load() -> Result<Self> {
        let cargo_toml_path = Self::find_cargo_toml()?;
        Self::load_from_path(&cargo_toml_path)
    }

    /// 查找 Cargo.toml 文件
    fn find_cargo_toml() -> Result<PathBuf> {
        let mut current_dir = std::env::current_dir().context("Failed to get current directory")?;

        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                return Ok(cargo_toml);
            }

            if let Some(parent) = current_dir.parent() {
                current_dir = parent.to_path_buf();
            } else {
                return Err(anyhow!(
                    "Could not find Cargo.toml file in current directory or any parent directory"
                ));
            }
        }
    }

    /// 获取所有依赖信息
    pub fn get_all_dependencies(&self) -> Vec<DependencyInfo> {
        let mut dependencies = Vec::new();

        // 处理常规依赖
        if let Some(deps) = &self.dependencies {
            dependencies.extend(self.parse_dependencies(deps));
        }

        // 处理开发依赖
        if let Some(dev_deps) = &self.dev_dependencies {
            dependencies.extend(self.parse_dependencies(dev_deps));
        }

        // 处理构建依赖
        if let Some(build_deps) = &self.build_dependencies {
            dependencies.extend(self.parse_dependencies(build_deps));
        }

        dependencies
    }

    /// 根据名称查找特定的依赖
    pub fn find_dependency(&self, name: &str) -> Option<DependencyInfo> {
        self.get_all_dependencies()
            .into_iter()
            .find(|dep| dep.name == name)
    }

    /// 解析依赖定义
    fn parse_dependencies(
        &self,
        deps: &HashMap<String, DependencyDefinition>,
    ) -> Vec<DependencyInfo> {
        deps.iter()
            .filter_map(|(name, def)| {
                self.parse_dependency_definition(name, def)
                    .map_err(|e| {
                        error!("⚠️  Failed to parse dependency '{name}': {e}");
                        e
                    })
                    .ok()
            })
            .collect()
    }

    /// 解析单个依赖定义
    fn parse_dependency_definition(
        &self,
        name: &str,
        def: &DependencyDefinition,
    ) -> Result<DependencyInfo> {
        match def {
            DependencyDefinition::Simple(version) => Ok(DependencyInfo {
                name: name.to_string(),
                dep_type: DependencyType::Version {
                    version: version.clone(),
                },
            }),
            DependencyDefinition::Detailed {
                version,
                git,
                branch,
                tag,
                rev,
                path,
                ..
            } => {
                // 优先级：git > path > version
                if let Some(git_url) = git {
                    Ok(DependencyInfo {
                        name: name.to_string(),
                        dep_type: DependencyType::Git {
                            git: git_url.clone(),
                            branch: branch.clone(),
                            tag: tag.clone(),
                            rev: rev.clone(),
                        },
                    })
                } else if let Some(path_str) = path {
                    Ok(DependencyInfo {
                        name: name.to_string(),
                        dep_type: DependencyType::Path {
                            path: path_str.clone(),
                        },
                    })
                } else if let Some(version_str) = version {
                    Ok(DependencyInfo {
                        name: name.to_string(),
                        dep_type: DependencyType::Version {
                            version: version_str.clone(),
                        },
                    })
                } else {
                    Err(anyhow!("Invalid dependency definition for '{}'", name))
                }
            }
        }
    }

    /// 获取所有 git 依赖
    pub fn get_git_dependencies(&self) -> Vec<DependencyInfo> {
        self.get_all_dependencies()
            .into_iter()
            .filter(|dep| matches!(dep.dep_type, DependencyType::Git { .. }))
            .collect()
    }

    /// 获取所有版本依赖（来自 crates.io）
    pub fn get_version_dependencies(&self) -> Vec<DependencyInfo> {
        self.get_all_dependencies()
            .into_iter()
            .filter(|dep| matches!(dep.dep_type, DependencyType::Version { .. }))
            .collect()
    }

    /// 获取所有路径依赖
    pub fn get_path_dependencies(&self) -> Vec<DependencyInfo> {
        self.get_all_dependencies()
            .into_iter()
            .filter(|dep| matches!(dep.dep_type, DependencyType::Path { .. }))
            .collect()
    }
}

impl DependencyInfo {
    // /// 获取依赖的仓库 URL（如果是 git 依赖）
    // pub fn get_git_url(&self) -> Option<&str> {
    //     match &self.dep_type {
    //         DependencyType::Git { git, .. } => Some(git),
    //         _ => None,
    //     }
    // }

    // /// 检查是否是 git 依赖
    // pub fn is_git_dependency(&self) -> bool {
    //     matches!(self.dep_type, DependencyType::Git { .. })
    // }

    // /// 检查是否是版本依赖（来自 crates.io）
    // pub fn is_version_dependency(&self) -> bool {
    //     matches!(self.dep_type, DependencyType::Version { .. })
    // }

    // /// 检查是否是路径依赖
    // pub fn is_path_dependency(&self) -> bool {
    //     matches!(self.dep_type, DependencyType::Path { .. })
    // }
}
