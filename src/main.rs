mod config;
pub mod model;
mod tools;

use anyhow::Result;
use rmcp::ServiceExt;
use tools::SigilServer;

#[tokio::main]
async fn main() -> Result<()> {
    // Log to stderr so stdout stays clean for MCP transport.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cfg = config::Config::load()?;
    let transport = (tokio::io::stdin(), tokio::io::stdout());
    let service = SigilServer::new(cfg).serve(transport).await?;
    service.waiting().await?;
    Ok(())
}
