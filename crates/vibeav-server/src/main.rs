//! vibeav - Multi-protocol streaming server.
//!
//! A MediaMTX-like streaming server that routes media between protocols.

use std::sync::Arc;

use tokio::sync::broadcast;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use vibeav_core::{ProtocolServer, Router};
use vibeav_rtsp::{RtspServer, RtspServerConfig};

mod testpattern;
use testpattern::{spawn_test_pattern, TestPatternConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_target(false)
        .compact()
        .init();

    info!("vibeav starting...");

    // Create shared router
    let router = Arc::new(Router::new());

    // Shutdown signal for test pattern generator
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    // Optionally start test pattern generator
    // Set VIBEAV_TEST_PATTERN=0 to disable
    let enable_test_pattern = std::env::var("VIBEAV_TEST_PATTERN")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(true); // Enabled by default

    if enable_test_pattern {
        let test_config = TestPatternConfig::default();
        info!(
            "Starting H.264 test pattern stream at {}",
            test_config.stream_path
        );
        let _test_handle = spawn_test_pattern(
            test_config,
            router.clone(),
            shutdown_tx.subscribe(),
        );
    }

    // Configure RTSP server
    let rtsp_config = RtspServerConfig::default()
        .with_bind("0.0.0.0:8554".parse()?);

    // Create and start RTSP server
    let rtsp_server = RtspServer::new(rtsp_config, router.clone());

    info!("Starting RTSP server on rtsp://0.0.0.0:8554");

    // Run the server (blocks until shutdown)
    rtsp_server.start().await?;

    // Signal shutdown to test pattern
    let _ = shutdown_tx.send(());

    Ok(())
}
