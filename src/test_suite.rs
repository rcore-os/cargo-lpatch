#[cfg(test)]
mod test_suite {
    use crate::{is_git_url, extract_crate_name_from_git_url};
    use crate::config::CargoConfig;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_git_url() {
        assert!(is_git_url("https://github.com/user/repo.git"));
        assert!(is_git_url("http://github.com/user/repo.git"));
        assert!(is_git_url("git://github.com/user/repo.git"));
        assert!(is_git_url("ssh://git@github.com/user/repo.git"));
        assert!(is_git_url("git@github.com:user/repo.git"));
        
        assert!(!is_git_url("serde"));
        assert!(!is_git_url("my-crate-name"));
    }

    #[test]
    fn test_extract_crate_name_from_git_url() {
        assert_eq!(
            extract_crate_name_from_git_url("https://github.com/dtolnay/anyhow.git").unwrap(),
            "anyhow"
        );
        assert_eq!(
            extract_crate_name_from_git_url("https://github.com/serde-rs/serde").unwrap(),
            "serde"
        );
        assert_eq!(
            extract_crate_name_from_git_url("git@github.com:tokio-rs/tokio.git").unwrap(),
            "tokio"
        );
    }

    #[test]
    fn test_cargo_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join(".cargo").join("config.toml");
        
        // 设置临时目录为当前目录
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        
        // 创建配置
        let mut config = CargoConfig::create_new().unwrap();
        
        // 添加 patch
        let patch_path = temp_dir.path().join("crates").join("serde");
        config.add_patch("serde", &patch_path).unwrap();
        
        // 保存配置
        config.save().unwrap();
        
        // 验证文件存在
        assert!(config_path.exists());
        
        // 验证内容
        let content = fs::read_to_string(&config_path).unwrap();
        println!("Generated config content: {content}");
        assert!(content.contains("[patch") && content.contains("crates-io"));
        assert!(content.contains("serde"));
        
        // 恢复原始目录
        std::env::set_current_dir(original_dir).unwrap();
    }
}
