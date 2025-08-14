use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct CargoConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<HashMap<String, HashMap<String, PatchConfig>>>,
    
    #[serde(flatten)]
    pub other: HashMap<String, toml::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatchConfig {
    pub path: String,
}

impl CargoConfig {
    pub fn load_or_create() -> Result<Self> {
        let config_path = Self::get_config_path();
        
        if config_path.exists() {
            println!("📄 Loading existing .cargo/config.toml");
            Self::load_from_file(&config_path)
        } else {
            println!("📄 Creating new .cargo/config.toml");
            Self::create_new()
        }
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        
        let config: CargoConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse config.toml")?;
        
        Ok(config)
    }

    pub fn create_new() -> Result<Self> {
        let config_dir = Self::get_config_dir();
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)
                .with_context(|| format!("Failed to create .cargo directory: {}", config_dir.display()))?;
        }
        
        Ok(Self::default())
    }

    pub fn add_patch(&mut self, crate_name: &str, local_path: &Path) -> Result<()> {
        self.add_patch_with_source(crate_name, local_path, "crates-io")
    }

    pub fn add_patch_with_source(&mut self, crate_name: &str, local_path: &Path, patch_source: &str) -> Result<()> {
        // 确保 patch 表存在
        if self.patch.is_none() {
            self.patch = Some(HashMap::new());
        }
        
        let patch_table = self.patch.as_mut().unwrap();
        
        // 确保指定的 patch 源表存在
        if !patch_table.contains_key(patch_source) {
            patch_table.insert(patch_source.to_string(), HashMap::new());
        }
        
        let source_patches = patch_table.get_mut(patch_source).unwrap();
        
        // 将路径转换为相对路径（相对于当前工作目录）
        let current_dir = std::env::current_dir()
            .context("Failed to get current directory")?;
        
        let relative_path = if local_path.is_absolute() {
            match local_path.strip_prefix(&current_dir) {
                Ok(rel_path) => rel_path.to_path_buf(),
                Err(_) => local_path.to_path_buf(), // 如果无法创建相对路径，使用绝对路径
            }
        } else {
            local_path.to_path_buf()
        };
        
        let path_str = relative_path.to_string_lossy().to_string();
        
        // 添加或更新 patch 配置
        source_patches.insert(crate_name.to_string(), PatchConfig {
            path: path_str,
        });
        
        println!("➕ Added patch for '{}' -> '{}' (source: {})", crate_name, relative_path.display(), patch_source);
        
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        let toml_string = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;
        
        fs::write(&config_path, toml_string)
            .with_context(|| format!("Failed to write config file: {}", config_path.display()))?;
        
        println!("💾 Saved configuration to {}", config_path.display());
        Ok(())
    }

    fn get_config_dir() -> PathBuf {
        // 尝试获取当前工作目录的 .cargo 目录
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let local_cargo_dir = current_dir.join(".cargo");
        
        // 如果当前目录没有 .cargo 目录，检查是否在 Rust 项目中
        if !local_cargo_dir.exists() {
            // 向上查找 Cargo.toml 文件
            let mut search_dir = current_dir.clone();
            loop {
                if search_dir.join("Cargo.toml").exists() {
                    return search_dir.join(".cargo");
                }
                match search_dir.parent() {
                    Some(parent) => search_dir = parent.to_path_buf(),
                    None => break,
                }
            }
        }
        
        local_cargo_dir
    }

    fn get_config_path() -> PathBuf {
        Self::get_config_dir().join("config.toml")
    }

    pub fn remove_patch(&mut self, crate_name: &str) -> Result<bool> {
        if let Some(patch_table) = &mut self.patch {
            if let Some(crates_io_patches) = patch_table.get_mut("crates-io") {
                let removed = crates_io_patches.remove(crate_name).is_some();
                
                // 如果 crates-io 表为空，移除它
                if crates_io_patches.is_empty() {
                    patch_table.remove("crates-io");
                }
                
                // 如果整个 patch 表为空，移除它
                if patch_table.is_empty() {
                    self.patch = None;
                }
                
                if removed {
                    println!("➖ Removed patch for '{}'", crate_name);
                }
                
                return Ok(removed);
            }
        }
        
        Ok(false)
    }

    pub fn list_patches(&self) -> Vec<(String, String)> {
        let mut patches = Vec::new();
        
        if let Some(patch_table) = &self.patch {
            if let Some(crates_io_patches) = patch_table.get("crates-io") {
                for (name, config) in crates_io_patches {
                    patches.push((name.clone(), config.path.clone()));
                }
            }
        }
        
        patches
    }
}
