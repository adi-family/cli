//! Client library for the ADI daemon IPC protocol
//!
//! Provides protocol types and a client for communicating with the
//! `adi daemon` background service manager.

pub mod client;
pub mod paths;
pub mod protocol;

pub use client::{CommandOutput, DaemonClient};
pub use protocol::{MessageFrame, Request, Response, ServiceConfig, ServiceInfo, ServiceState};
