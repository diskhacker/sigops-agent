//! SigOps Agent library crate.
//!
//! Exposes the agent's internals so integration tests and external
//! embedders can drive heartbeat, discovery, execution, and security
//! logic directly. The `sigops-agent` binary (`src/main.rs`) wires
//! these modules into a long-running process.

pub mod config;
pub mod discovery;
pub mod executor;
pub mod health;
pub mod heartbeat;
pub mod security;
