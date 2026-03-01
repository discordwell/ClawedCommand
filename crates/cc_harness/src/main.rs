//! ClawedCommand MCP Harness Server
//! Launches a headless simulation accessible via MCP tools over stdio.

use cc_harness::headless::HeadlessSim;
use cc_harness::server::HarnessServer;
use rmcp::ServiceExt;
use rmcp::transport::stdio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let sim = HeadlessSim::new(64, 64);
    let server = HarnessServer::new(sim);

    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
