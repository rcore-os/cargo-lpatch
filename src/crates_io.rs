use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CrateResponse {
    #[serde(rename = "crate")]
    crate_info: CrateInfo,
}

#[derive(Debug, Deserialize)]
struct CrateInfo {
    repository: Option<String>,
}

pub struct CratesIoClient {
    client: Client,
    base_url: String,
}

impl CratesIoClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://crates.io/api/v1".to_string(),
        }
    }

    pub async fn get_repository_url(&self, crate_name: &str) -> Result<String> {
        let url = format!("{}/crates/{}", self.base_url, crate_name);

        info!("Querying crates.io for crate: {crate_name}");

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "cargo-lpatch/0.1.0")
            .send()
            .await?;

        if response.status().is_success() {
            let crate_response: CrateResponse = response.json().await?;

            match crate_response.crate_info.repository {
                Some(repo_url) => {
                    // 处理一些常见的仓库 URL 格式
                    let cleaned_url = self.clean_repository_url(&repo_url)?;
                    Ok(cleaned_url)
                }
                None => Err(anyhow!(
                    "Crate '{}' does not have a repository URL",
                    crate_name
                )),
            }
        } else {
            Err(anyhow!(
                "Failed to fetch crate info for '{}': HTTP {}",
                crate_name,
                response.status()
            ))
        }
    }

    fn clean_repository_url(&self, url: &str) -> Result<String> {
        let mut cleaned = url.to_string();

        // 移除常见的无用后缀
        if cleaned.ends_with("/tree/master") {
            cleaned = cleaned.replace("/tree/master", "");
        }
        if cleaned.ends_with("/tree/main") {
            cleaned = cleaned.replace("/tree/main", "");
        }

        // 确保 GitHub URL 是 .git 格式（适合克隆）
        if cleaned.contains("github.com") && !cleaned.ends_with(".git") {
            cleaned.push_str(".git");
        }

        // 验证是否是有效的 git URL
        if !self.is_valid_git_url(&cleaned) {
            return Err(anyhow!("Invalid repository URL: {}", cleaned));
        }

        Ok(cleaned)
    }

    fn is_valid_git_url(&self, url: &str) -> bool {
        url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("git://")
            || url.starts_with("ssh://")
            || url.contains("git@")
    }
}
