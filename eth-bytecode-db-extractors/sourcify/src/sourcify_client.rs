use crate::settings::Settings;
use anyhow::Context;
use eth_bytecode_db_proto::blockscout::eth_bytecode_db::v2::{
    solidity_verifier_client::SolidityVerifierClient, VerifySolidityMultiPartRequest,
    VerifySolidityStandardJsonRequest,
};
use reqwest::Client as HttpClient;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_rate_limiter::RateLimiter;
use reqwest_retry::{policies::ExponentialBackoff, RetryTransientMiddleware};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::{info, warn};

#[derive(Clone)]
pub struct SourcifyClient {
    http_client: ClientWithMiddleware,
    eth_bytecode_db_client: SolidityVerifierClient<Channel>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContractList {
    full: Vec<String>,
    partial: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ContractInfo {
    pub compiler: CompilerInfo,
    pub language: String,
    pub sources: BTreeMap<String, Source>,
    pub settings: serde_json::Value,
    pub files: BTreeMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompilerInfo {
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Source {
    pub content: String,
}

impl SourcifyClient {
    pub async fn try_new(settings: &Settings) -> anyhow::Result<Self> {
        let http_client = HttpClient::new();
        let retry_policy = ExponentialBackoff::builder().build_with_max_retries(3);
        let rate_limiter = RateLimiter::direct(
            governor::Quota::per_second(std::num::NonZeroU32::new(settings.limit_requests_per_second).unwrap())
                .allow_burst(std::num::NonZeroU32::new(settings.limit_requests_per_second * 2).unwrap()),
        );
        let http_client = ClientBuilder::new(http_client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .with(rate_limiter)
            .build();

        let eth_bytecode_db_client = SolidityVerifierClient::connect(settings.eth_bytecode_db_url.clone()).await?;

        Ok(Self {
            http_client,
            eth_bytecode_db_client,
        })
    }

    pub async fn extract_chain(&self, chain_id: u64) -> anyhow::Result<()> {
        info!("Extracting contracts for chain {}", chain_id);
        let list_url = format!("{}/contracts/list/{}", "https://sourcify.dev/server", chain_id);
        let response = self.http_client.get(&list_url).send().await?;
        if !response.status().is_success() {
            warn!("Failed to get contract list for chain {}: {}", chain_id, response.status());
            return Ok(());
        }
        let contract_list: ContractList = response.json().await?;
        let addresses = contract_list.full.into_iter().chain(contract_list.partial);

        for address in addresses {
            if let Err(e) = self.extract_contract(chain_id, &address).await {
                warn!("Failed to extract contract {} on chain {}: {:?}", address, chain_id, e);
            }
        }
        Ok(())
    }

    async fn extract_contract(&self, chain_id: u64, address: &str) -> anyhow::Result<()> {
        let info_url = format!("{}/contracts/full_match/{}/{}", "https://sourcify.dev/server", chain_id, address);
        let response = self.http_client.get(&info_url).send().await?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("Failed to get contract info: {}", response.status()));
        }
        let contract_info: ContractInfo = response.json().await?;

        // Assume it's Solidity for now
        if contract_info.language.to_lowercase() != "solidity" {
            return Ok(()); // Skip non-Solidity
        }

        // Get bytecode from metadata.json
        let metadata_content = contract_info.files.get("metadata.json").context("No metadata.json")?;
        let metadata: serde_json::Value = serde_json::from_str(metadata_content)?;
        let bytecode = metadata["bytecode"].as_str().context("No bytecode in metadata")?;

        // Prepare sources
        let sources = contract_info
            .sources
            .into_iter()
            .map(|(name, source)| (name, source.content))
            .collect::<BTreeMap<_, _>>();

        // For simplicity, assume standard JSON input
        let input = serde_json::json!({
            "language": "Solidity",
            "sources": sources,
            "settings": contract_info.settings
        });

        let request = VerifySolidityStandardJsonRequest {
            bytecode: bytecode.to_string(),
            bytecode_type: 0, // Creation
            compiler_version: contract_info.compiler.version,
            input: serde_json::to_string(&input)?,
            metadata: Some(metadata_content.clone()),
        };

        self.eth_bytecode_db_client.verify_solidity_standard_json(request).await?;

        info!("Verified contract {} on chain {}", address, chain_id);

        Ok(())
    }
}