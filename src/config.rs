use std::collections::HashMap;
use std::io::IsTerminal;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
struct RawClusterEntry {
    host: String,
    port: Option<u16>,
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

/// Try to extract an embedded port from a URL string.
/// Returns `(host_without_port, Some(port))` if a port was found, or `(host, None)` otherwise.
fn extract_port_from_url(host: &str) -> (String, Option<u16>) {
    if let Ok(mut url) = Url::parse(host)
        && let Some(port) = url.port()
    {
        let _ = url.set_port(None);
        let clean = url.as_str().trim_end_matches('/').to_string();
        return (clean, Some(port));
    }
    (host.to_string(), None)
}

/// Sanitize the `host` and `port` values from a raw cluster entry:
/// - Strip trailing `/` from `host`
/// - Extract an embedded port from `host` if present
/// - Resolve conflicts between an embedded port and the `port` field
///
/// Returns `(clean_host, resolved_port)`.
fn sanitize_host_port(alias: &str, host: String, port: Option<u16>) -> Result<(String, u16)> {
    // Strip trailing slash(es)
    let host = host.trim_end_matches('/').to_string();

    // Extract embedded port (if any) and get a port-free host string
    let (clean_host, embedded_port) = extract_port_from_url(&host);

    match (embedded_port, port) {
        // No port anywhere — error
        (None, None) => Err(anyhow!(
            "Cluster '{}': 'port' is required when host does not contain an embedded port",
            alias
        )),
        // Only the port field is set — use it
        (None, Some(p)) => Ok((host, p)),
        // Only an embedded port — use it
        (Some(ep), None) => Ok((clean_host, ep)),
        // Both set and they agree — use either
        (Some(ep), Some(fp)) if ep == fp => Ok((clean_host, fp)),
        // Conflict — prompt interactively or error in non-interactive mode
        (Some(ep), Some(fp)) => {
            eprintln!(
                "warning: cluster '{}': port conflict — host URL contains port {} but port field is {}",
                alias, ep, fp
            );
            if std::io::stdin().is_terminal() {
                eprint!(
                    "  Which port should be used?\n  [1] {} (from URL)\n  [2] {} (from port field)\n  Choice [1]: ",
                    ep, fp
                );
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                match input.trim() {
                    "" | "1" => Ok((clean_host, ep)),
                    "2" => Ok((host, fp)),
                    other => Err(anyhow!(
                        "Cluster '{}': invalid choice '{}' for port conflict resolution",
                        alias,
                        other
                    )),
                }
            } else {
                Err(anyhow!(
                    "Cluster '{}': port conflict — host URL contains port {} but port field is {}. \
                     Fix cluster.yml to resolve.",
                    alias,
                    ep,
                    fp
                ))
            }
        }
    }
}

fn validate_entry(alias: String, entry: RawClusterEntry) -> Result<ClusterConfig> {
    let (host, port) = sanitize_host_port(&alias, entry.host, entry.port)?;

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
        host,
        port,
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

    #[test]
    fn trailing_slash_stripped() {
        let yaml = "
my_cluster:
  host: https://example.com/
  port: 9200
  api_key: key
";
        let configs = parse(yaml).unwrap();
        assert_eq!(configs[0].host, "https://example.com");
    }

    #[test]
    fn embedded_port_extracted_when_port_field_absent() {
        let yaml = "
my_cluster:
  host: https://example.com:9200
  api_key: key
";
        let configs = parse(yaml).unwrap();
        assert_eq!(configs[0].host, "https://example.com");
        assert_eq!(configs[0].port, 9200);
    }

    #[test]
    fn embedded_port_matches_port_field() {
        let yaml = "
my_cluster:
  host: https://example.com:9200
  port: 9200
  api_key: key
";
        let configs = parse(yaml).unwrap();
        assert_eq!(configs[0].host, "https://example.com");
        assert_eq!(configs[0].port, 9200);
    }

    #[test]
    fn embedded_port_conflicts_with_port_field_non_interactive() {
        // Stdin is not a TTY in test runs, so this must return an error.
        let yaml = "
my_cluster:
  host: https://example.com:9300
  port: 9200
  api_key: key
";
        assert!(parse(yaml).is_err());
    }
}
