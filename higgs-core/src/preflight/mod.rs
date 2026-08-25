//! Startup validation and idempotent seeding hooks (preflight).
//!
//! The host runs a [`PreflightRunner`](crate::preflight::PreflightRunner) once during
//! process startup after the global database router is installed and before background
//! schedulers, so seeding and validation stay visible and structured instead of only
//! Chronon `RunOnce` jobs. Keep the returned statuses for logs or an auth-gated setup UI.
//!
//! Runnable: `cargo run -p higgs --example preflight_boot --features preflight`
//!
//! # Examples
//!
//! Implement a check, register it, run once against Valence, and assert Passed.
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use async_trait::async_trait;
//! use higgs_core::preflight::{
//!     PreflightCheck, PreflightResult, PreflightRunner, PreflightStatus,
//! };
//! use valence::{InMemoryBackend, Valence};
//!
//! struct AlwaysPass;
//!
//! #[async_trait]
//! impl PreflightCheck for AlwaysPass {
//!     fn name(&self) -> &'static str {
//!         "demo-always-pass"
//!     }
//!     fn description(&self) -> &'static str {
//!         "example check that always passes"
//!     }
//!     async fn check(&self, _valence: &Valence) -> PreflightResult {
//!         PreflightResult {
//!             check_name: self.name().to_string(),
//!             status: PreflightStatus::Passed {
//!                 message: "demo ok".into(),
//!             },
//!         }
//!     }
//! }
//!
//! let valence = Valence::builder()
//!     .add_backend("default", Arc::new(InMemoryBackend::new()))
//!     .build()?;
//! let mut runner = PreflightRunner::new();
//! runner.register(AlwaysPass);
//! let results = runner.run_all(&valence).await;
//! assert_eq!(results.len(), 1);
//! assert!(matches!(results[0].status, PreflightStatus::Passed { .. }));
//! ```

mod check;
mod runner;
mod status;
mod store;

pub use check::PreflightCheck;
pub use runner::PreflightRunner;
pub use status::{PreflightResult, PreflightStatus};
pub use store::{preflight_results_snapshot, store_preflight_results};
