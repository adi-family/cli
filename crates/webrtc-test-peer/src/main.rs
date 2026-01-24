//! WebRTC Test Peer CLI
//!
//! A minimal WebRTC peer for E2E testing of web-app WebRTC functionality.
//!
//! # Usage
//!
//! ```bash
//! # Basic usage - connect to signaling server
//! webrtc-test-peer --signaling ws://localhost:8080/ws
//!
//! # With custom device ID
//! webrtc-test-peer --signaling ws://localhost:8080/ws --device-id test-cocoon-001
//!
//! # One-shot mode (exit after first session)
//! webrtc-test-peer --signaling ws://localhost:8080/ws --one-shot
//!
//! # With mock filesystem
//! webrtc-test-peer --signaling ws://localhost:8080/ws --mock-fs-root ./fixtures/
//!
//! # Error injection for testing
//! webrtc-test-peer --signaling ws://localhost:8080/ws --drop-ice-rate 0.2 --latency-ms 100
//!
//! # Test connection timeouts
//! webrtc-test-peer --signaling ws://localhost:8080/ws --no-answer
//! ```

use clap::Parser;
use tracing_subscriber::EnvFilter;

use webrtc_test_peer::{CliArgs, Config, TestPeer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI args
    let args = CliArgs::parse();

    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&args.log_level));
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    // Build config from args
    let config = Config::from_cli(args.clone());

    tracing::info!(
        "Starting WebRTC Test Peer\n  \
         Signaling: {}\n  \
         Device ID: {}\n  \
         Name: {}\n  \
         One-shot: {}\n  \
         Bypass auth: {}",
        config.signaling_url,
        config.device_id,
        config.name,
        config.one_shot,
        config.bypass_auth,
    );

    if config.scenario.drop_ice_rate > 0.0 {
        tracing::warn!("ICE drop rate: {:.1}%", config.scenario.drop_ice_rate * 100.0);
    }
    if config.scenario.drop_data_rate > 0.0 {
        tracing::warn!("Data drop rate: {:.1}%", config.scenario.drop_data_rate * 100.0);
    }
    if !config.scenario.latency.is_zero() {
        tracing::warn!("Artificial latency: {:?}", config.scenario.latency);
    }
    if config.scenario.no_answer {
        tracing::warn!("No-answer mode enabled (will not send WebRTC answers)");
    }

    // Create and run peer
    let peer = TestPeer::new(config.clone());

    // Handle shutdown signals
    let shutdown = async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Received shutdown signal");
    };

    // Run with optional timeout
    let result = if let Some(timeout) = config.timeout {
        tokio::select! {
            result = peer.run_with_timeout(timeout) => result,
            _ = shutdown => {
                tracing::info!("Shutting down...");
                Ok(())
            }
        }
    } else {
        tokio::select! {
            result = peer.run() => result,
            _ = shutdown => {
                tracing::info!("Shutting down...");
                Ok(())
            }
        }
    };

    match result {
        Ok(()) => {
            tracing::info!("Test peer finished successfully");
            Ok(())
        }
        Err(e) => {
            tracing::error!("Test peer failed: {}", e);
            Err(e.into())
        }
    }
}
