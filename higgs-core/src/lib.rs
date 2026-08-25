//! # Higgs core — Valence / session hub
//!
//! Valence/session hub: config, preflight, and per-request context for server functions.
//!
//! ## Features
//!
//! - **Config** — [`HiggsConfig`] / [`HiggsConfigBuilder`] hold a
//!   [`HiggsValenceFactory`]; build once at host boot and store as
//!   `Arc<HiggsConfig>` in Leptos context — [Quick example](#quick-example)
//! - **Factory** — host-implemented [`HiggsValenceFactory`] builds request-scoped
//!   `valence::Valence` from serialized actor JSON — [Quick example](#quick-example)
//! - **Request context** — `Higgs` (feature `ssr`) via `Higgs::from_request` —
//!   [Per-request context](#per-request-context-ssr)
//! - **Server helpers** — [`server_runtime`] permission-denied payload encode/decode —
//!   [`server_runtime`] module
//! - **Startup** — `preflight` (feature `preflight`) after the database router is
//!   installed — [Startup preflight](#startup-preflight)
//! - **Actor policy** — [`actor_policy`] helpers such as
//!   [`actor_policy::external_actor_json_policy`] — [`actor_policy`] module
//!
//! # Quick example
//!
//! ```rust,no_run
//! # #[cfg(feature = "test-utils")]
//! # fn boot() -> Result<(), higgs_core::HiggsError> {
//! use std::sync::Arc;
//! use higgs_core::{HiggsConfig, test_support::UnreachableValenceFactory};
//!
//! let config = Arc::new(
//!     HiggsConfig::builder()
//!         .valence_factory(UnreachableValenceFactory)
//!         .build()?,
//! );
//! // provide_context(config) at host boot; Higgs::from_request reads it (feature ssr).
//! assert!(
//!     Arc::strong_count(&config) >= 1,
//!     "boot succeeds when valence_factory is set"
//! );
//! # Ok(())
//! # }
//! ```
//!
//! Observable boot: `cargo run -p higgs --example config_boot` prints
//! `HiggsConfig booted with in-memory Valence factory (anonymous + user OK)`.
//!
//! Typical sequence: implement [`HiggsValenceFactory`] → build [`HiggsConfig`] →
//! provide `Arc` in Leptos context → (feature `ssr`) `Higgs::from_request` → optional
//! `preflight::PreflightRunner` after the DB router is up.
//!
//! Next: [Per-request context](#per-request-context-ssr), [Startup preflight](#startup-preflight),
//! [`server_runtime`], or [`actor_policy`].
//!
//! # Feature flags
//!
//! | Feature | What it enables |
//! |---------|-----------------|
//! | `ssr` | `Higgs`, `Higgs::from_request`, `higgs-host` / Leptos SSR, enables `preflight` |
//! | `preflight` | `preflight` module (also enabled by `ssr`) |
//! | `test-utils` | `test_support` doubles for downstream tests |
//!
//! # Per-request context (SSR)
//!
//! Prerequisites: feature `ssr`, `Arc<HiggsConfig>` in Leptos context, Valence router
//! (and optional session snapshot) in Axum extensions via `higgs-host`.
//!
//! Call `Higgs::from_request` inside server functions; use `valence()` for the session
//! actor.
//!
//! ```rust,no_run
//! # #[cfg(all(feature = "ssr", feature = "test-utils"))]
//! # async fn demo() -> Result<(), leptos::prelude::ServerFnError> {
//! use higgs_core::Higgs;
//!
//! let ctx = Higgs::from_request().await?;
//! let valence = ctx.valence().map_err(leptos::prelude::ServerFnError::new)?;
//! let actor = valence.actor();
//! assert!(
//!     matches!(
//!         actor,
//!         valence::Actor::Anonymous | valence::Actor::User { .. } | valence::Actor::System { .. }
//!     )
//! );
//! # Ok(())
//! # }
//! ```
//!
//! Observable outcome:
//! `cargo run -p higgs --example shared_factory --features ssr` prints that interactive
//! and worker rebuild succeeded (external System rejected).
//!
//! Errors surface as [`HiggsError`] (missing context, internal Valence/actor failures).
//! For Chronon / Boson / Photon accessors on the same request, depend on package `higgs`.
//! Next: [Startup preflight](#startup-preflight).
//!
//! # Startup preflight
//!
//! Prerequisites: feature `preflight` (also enabled by `ssr`), database router installed.
//!
//! Run [`preflight::PreflightRunner`] once at host boot.
//!
//! ```rust,ignore
//! use higgs_core::preflight::{PreflightCheck, PreflightRunner, PreflightResult, PreflightStatus};
//!
//! let mut runner = PreflightRunner::new();
//! runner.register(AlwaysPass);
//! let results = runner.run_all(&valence).await;
//! assert_eq!(results.len(), 1);
//! assert!(matches!(results[0].status, PreflightStatus::Passed { .. }));
//! ```
//!
//! Observable outcome:
//! `cargo run -p higgs --example preflight_boot --features preflight` prints
//! `preflight: demo-always-pass — Passed { … }`.
//!
//! Failed checks preserve status on the returned results. Next: [`actor_policy`] for
//! external actor JSON, or package `higgs` for subsystem accessors.
//!
//! # Notes
//!
//! - Failures surface as [`HiggsError`] (missing context, internal Valence/actor
//!   errors). See the [crate root example](#quick-example) for builder wiring.
//! - Host wiring that needs Chronon / Boson / Photon accessors belongs on package
//!   `higgs`, not this crate alone.

// Clippy sometimes emits `too_long_first_doc_paragraph` without a span on libtest builds.
#![allow(clippy::too_long_first_doc_paragraph)]

/// Actor-JSON policy helpers for shared Valence factories.
pub mod actor_policy;
mod config;
mod error;
/// Permission-denied payload encode/decode helpers for server functions.
pub mod server_runtime;
mod valence_factory;

#[cfg(feature = "ssr")]
mod context;
/// Startup validation and idempotent seeding hooks, run once at host boot.
#[cfg(feature = "preflight")]
pub mod preflight;

pub use config::{HiggsConfig, HiggsConfigBuilder};
pub use error::HiggsError;
pub use valence_factory::HiggsValenceFactory;

#[cfg(feature = "ssr")]
pub use context::Higgs;

/// Shared test doubles for downstream crates' own tests (see module docs).
#[cfg(any(test, feature = "test-utils"))]
pub mod test_support;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn valence_factory_is_object_safe() {
        let _: Arc<dyn HiggsValenceFactory> = Arc::new(test_support::UnreachableValenceFactory);
    }

    #[test]
    fn builder_requires_valence_factory() {
        let result = HiggsConfig::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_succeeds_with_factory() {
        let config = HiggsConfig::builder()
            .valence_factory(test_support::UnreachableValenceFactory)
            .build();
        assert!(config.is_ok());
    }

    #[test]
    fn builder_arc_factory() {
        let factory: Arc<dyn HiggsValenceFactory> = test_support::unreachable_valence_factory();
        let config = HiggsConfig::builder().valence_factory_arc(factory).build();
        assert!(config.is_ok());
    }
}
