use blockscout_service_launcher::launcher::ConfigSettings;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub database_url: String,
    #[serde(default)]
    pub create_database: bool,
    #[serde(default)]
    pub run_migrations: bool,

    #[serde(default = "default_sourcify_url")]
    pub sourcify_url: String,

    pub eth_bytecode_db_url: String,
    pub eth_bytecode_db_api_key: Option<String>,

    #[serde(default = "default_limit_requests_per_second")]
    pub limit_requests_per_second: u32,

    #[serde(default = "default_n_threads")]
    pub n_threads: usize,

    /// List of chain IDs to extract from
    pub chains: Vec<u64>,
}

impl ConfigSettings for Settings {
    const SERVICE_NAME: &'static str = "SOURCIFY_EXTRACTOR__CONFIG";

    fn validate(&self) -> anyhow::Result<()> {
        if self.chains.is_empty() {
            return Err(anyhow::anyhow!("`chains` should not be empty"));
        }
        Ok(())
    }
}

fn default_sourcify_url() -> String {
    "https://sourcify.dev/server".to_string()
}

fn default_limit_requests_per_second() -> u32 {
    10
}

fn default_n_threads() -> usize {
    4
}