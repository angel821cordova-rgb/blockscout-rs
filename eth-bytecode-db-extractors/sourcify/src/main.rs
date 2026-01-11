use anyhow::Context;
use blockscout_service_launcher::{launcher::ConfigSettings, tracing as launcher_tracing};
use futures::future::join_all;
use sourcify::{Settings, SourcifyClient};
use std::sync::Arc;

const SERVICE_NAME: &str = "sourcify-extractor";

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    launcher_tracing::init_logs(SERVICE_NAME, &Default::default(), &Default::default())
        .context("tracing initialization")?;

    let settings = Settings::build().context("failed to read config")?;

    let client = SourcifyClient::try_new(&settings).await.context("failed to create client")?;

    let mut handles = Vec::new();
    for chain_id in settings.chains {
        let client = client.clone();
        let handle = tokio::spawn(async move {
            client.extract_chain(chain_id).await
        });
        handles.push(handle);
    }

    for result in join_all(handles).await {
        result.context("join handle")?.context("extract chain")?;
    }

    Ok(())
}