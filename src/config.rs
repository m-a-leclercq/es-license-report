use std::collections::HashMap;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RawClusterEntry {
    host: String,
    port: u16,
    username: Option<String>,
    password: Option<String>,
    api_key: Option<String>,
    #[serde(default = "default_verify_certs")]
    verify_certs: bool,
    ca_certs: Option<String>,
}

fn default_verify_certs() -> bool {
    true
}

/// Authentication method for a cluster connection.
#[derive(Debug, Clone)]
pub enum Auth {
    Basic { username: String, password: String },
    ApiKey(String),
}

/// Fully validated and normalized cluster connection configuration.
#[derive(Debug, Clone)]
pub struct ClusterConfig {
    pub alias: String,
    pub host: String,
    pub port: u16,
    pub auth: Auth,
    pub verify_certs: bool,
    pub ca_certs: Option<String>,
}

/// Read, parse, and validate a cluster YAML config file.
/// Returns one `ClusterConfig` per cluster alias, sorted by alias.
pub fn load_config(path: &Path) -> Result<Vec<ClusterConfig>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    parse_config_str(&content)
}

/// Parse and validate cluster config from a YAML string (also used in tests).
pub fn parse_config_str(content: &str) -> Result<Vec<ClusterConfig>> {
    let raw: HashMap<String, RawClusterEntry> =
        serde_yaml::from_str(content).context("Failed to parse YAML config")?;

    let mut configs: Vec<ClusterConfig> = raw
        .into_iter()
        .map(|(alias, entry)| validate_entry(alias, entry))
        .collect::<Result<_>>()?;

    configs.sort_by(|a, b| a.alias.cmp(&b.alias));
    Ok(configs)
}

fn validate_entry(alias: String, entry: RawClusterEntry) -> Result<ClusterConfig> {
    // api_key takes priority over username/password
    let auth = if let Some(key) = entry.api_key.filter(|k| !k.is_empty()) {
        Auth::ApiKey(key)
    } else if let Some(username) = entry.username.filter(|u| !u.is_empty()) {
        let password = entry
            .password
            .filter(|p| !p.is_empty())
            .ok_or_else(|| anyhow!("Cluster '{}': 'username' is set but 'password' is missing", alias))?;
        Auth::Basic { username, password }
    } else {
        return Err(anyhow!(
            "Cluster '{}': requires either 'api_key' or 'username'/'password'",
            alias
        ));
    };

    if let Some(ref ca_path) = entry.ca_certs
        && !Path::new(ca_path).exists()
    {
        return Err(anyhow!(
            "Cluster '{}': ca_certs file not found: {}",
            alias,
            ca_path
        ));
    }

    Ok(ClusterConfig {
        alias,
        host: entry.host,
        port: entry.port,
        auth,
        verify_certs: entry.verify_certs,
        ca_certs: entry.ca_certs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Result<Vec<ClusterConfig>> {
        parse_config_str(yaml)
    }

    #[test]
    fn valid_basic_auth() {
        let yaml = "
my_cluster:
  host: https://localhost
  port: 9200
  username: user
  password: pass
";
        let configs = parse(yaml).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(matches!(configs[0].auth, Auth::Basic { .. }));
    }

    #[test]
    fn valid_api_key() {
        let yaml = "
my_cluster:
  host: https://localhost
  port: 9200
  api_key: mykey123
";
        let configs = parse(yaml).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(matches!(configs[0].auth, Auth::ApiKey(_)));
    }

    #[test]
    fn api_key_priority_over_basic() {
        let yaml = "
my_cluster:
  host: https://localhost
  port: 9200
  api_key: mykey123
  username: user
  password: pass
";
        let configs = parse(yaml).unwrap();
        assert_eq!(configs.len(), 1);
        assert!(matches!(configs[0].auth, Auth::ApiKey(_)));
    }

    #[test]
    fn missing_auth_fails() {
        let yaml = "
my_cluster:
  host: https://localhost
  port: 9200
";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn username_without_password_fails() {
        let yaml = "
my_cluster:
  host: https://localhost
  port: 9200
  username: user
";
        assert!(parse(yaml).is_err());
    }

    #[test]
    fn missing_pem_file_fails() {
        let yaml = "
my_cluster:
  host: https://localhost
  port: 9200
  api_key: key
  ca_certs: /nonexistent/path/cert.pem
";
        assert!(parse(yaml).is_err());
    }
}
