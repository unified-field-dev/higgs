//! # Higgs — platform host wiring
//!
//! Process-wide [`HiggsConfig`] and per-request `Higgs` for hosts:
//! one Valence factory shared by server functions, Chronon jobs, Boson tasks, and
//! Photon handlers, plus optional Chronon / Boson / Photon accessors when those
//! backends are registered at boot.
//!
//! ## Features
//!
//! - **Shared Valence factory** — One host factory on [`HiggsConfig`] rebuilds Valence for
//!   server functions and Chronon / Boson / Photon workers from the same actor JSON.
//!   Wire it at process boot. [Getting started](#getting-started)
//! - **Request context** — Per HTTP request, `Higgs::from_request` (feature `ssr`) builds
//!   the router, session actor, and config for Leptos server functions.
//!   [SSR](#ssr--leptos-server-functions)
//! - **Valence accessors** — After `from_request`, `Higgs::valence` uses the session actor;
//!   `Higgs::unsafe_system_valence` creates an operation-tagged `System` actor for privileged
//!   work. [SSR](#ssr--leptos-server-functions)
//! - **Subsystem accessors** — When the host registered backends on the builder, SSR code
//!   calls Chronon / Boson / Photon through `Higgs::chronon` / `boson` / `photon`.
//!   Worker Valence recovery lives under [Workers](#workers--chronon--boson--photon).
//! - **Startup preflight** — Host-owned boot checks and idempotent seed hooks: run once after
//!   the database router is up and before schedulers, keep structured pass/fail for logs or
//!   setup UI. [Preflight at startup](#preflight-at-startup)
//! - **Macros and session** — Lets Leptos `#[server]` handlers set the Higgs operation name
//!   and optionally require a signed-in session before the body runs (per request).
//!   [Macros and session](#macros-and-session)
//! - **Permission helpers** — Provides encode/decode helpers for hand-rolled permission-denied
//!   payloads ([`server_runtime`]). The `permission = "..."` macro attribute is not shipped.
//!
//! # Getting started
//!
//! Higgs boots a process-wide [`HiggsConfig`] (shared Valence factory plus optional backends),
//! then serves requests and workers from that same factory. The host owns boot order.
//!
//! 1. Implement a host Valence factory (usually both [`HiggsValenceFactory`] for this
//!    crate and `valence::ValenceFactory` for Chronon / Boson / Photon identity adapters).
//! 2. Build [`HiggsConfig`] once at startup (`valence_factory`, optional `.chronon` /
//!    `.boson` / `.photon`), wrap in `Arc`.
//! 3. Provide `Arc<HiggsConfig>` in Leptos context for server functions.
//! 4. Hand the **same** `valence::ValenceFactory` into Chronon `ContextFactory`, Boson
//!    `ExecutionContextFactory`, and Photon identity / executor wiring (see
//!    [Workers](#workers--chronon--boson--photon)).
//! 5. SSR: `Higgs::from_request` → `valence()` / subsystem accessors.
//! 6. Workers: recover Valence from that family's context helper — workers have no Leptos
//!    request, so they do not call `Higgs::from_request`.
//! 7. Optionally run `preflight` after the database router is installed and before
//!    schedulers start.
//!
//! ```rust,no_run
//! # #[cfg(feature = "test-utils")]
//! # fn boot() -> Result<(), higgs::HiggsError> {
//! use std::sync::Arc;
//! use higgs::{HiggsConfig, test_support::UnreachableValenceFactory};
//!
//! let config = Arc::new(
//!     HiggsConfig::builder()
//!         .valence_factory(UnreachableValenceFactory)
//!         .build()?,
//! );
//! assert!(
//!     Arc::strong_count(&config) >= 1,
//!     "host boots with a shared Arc<HiggsConfig>"
//! );
//! // Host boot: leptos::provide_context(config.clone());
//! # Ok(())
//! # }
//! ```
//!
//! Observable boot path: `cargo run -p higgs --example config_boot` prints
//! `HiggsConfig booted with in-memory Valence factory (anonymous + user OK)`.
//!
//! Next: [SSR](#ssr--leptos-server-functions), [Workers](#workers--chronon--boson--photon),
//! [Preflight at startup](#preflight-at-startup), or [Macros and session](#macros-and-session).
//!
//! # Feature flags
//!
//! | Feature | What it enables |
//! |---------|-----------------|
//! | `ssr` | `Higgs`, `Higgs::from_request`, pulls `higgs-host` / Leptos SSR, enables `preflight` |
//! | `preflight` | Re-export of `preflight` (also enabled by `ssr`) |
//! | `chronon` | Chronon builder + `Higgs::chronon` / scheduler / registry accessors |
//! | `boson` | Boson builder + `Higgs::boson` |
//! | `photon` | Photon builder + `Higgs::photon` |
//! | `full` | `chronon` + `boson` + `photon` |
//! | `test-utils` | Test doubles from `higgs_core` |
//!
//! Default features are empty — enable `ssr` for request context, and subsystem
//! features (or `full`) when the host wires those backends.
//!
//! # Security notes
//!
//! - Prefer `Higgs::valence` for user-driven data access after
//!   `require_session` / `#[higgs_macros::server(auth)]`.
//! - `Higgs::unsafe_system_valence` mints `Actor::System` and **bypasses Valence
//!   privacy**. Do not use it to paper over missing policies or auth gates. There is
//!   no soft-named `system_valence` alias.
//! - Raw router access is `Higgs::unsafe_database_router` / `higgs_host::unsafe_data_plane`.
//! - Install [`actor_policy::external_actor_json_policy`] on factories that rebuild
//!   Valence from untrusted actor JSON (see repository `SECURITY.md`).
//! - Use `#[higgs_macros::server(auth)]` for endpoints that require a signed-in session.
//!
//! # Runnable examples
//!
//! `cargo run -p higgs --example config_boot`
//! `cargo run -p higgs --example shared_factory --features ssr`
//! `cargo run -p higgs --example preflight_boot --features preflight`
//! `cargo run -p higgs --example axum_session_host --features ssr`
//! `cargo run -p higgs --example server_fn_context --features ssr`
//! `cargo run -p higgs --example server_fn_backends --features full,ssr`
//! `cargo run -p higgs --example chronon_job --features chronon`
//! `cargo run -p higgs --example boson_task --features boson`
//! `cargo run -p higgs --example photon_worker --features photon`
//!
//! See `higgs/examples/README.md` for the teaching path and success stdout lines.
//!
//! # SSR — Leptos server functions
//!
//! Per-request Valence for Leptos server functions: the host provides `Arc<HiggsConfig>`,
//! and `Higgs::from_request` builds router + session actor for the current call.
//!
//! Prerequisites: feature `ssr`, `Arc<HiggsConfig>` in Leptos context, session middleware
//! when using `#[higgs_macros::server(auth)]`.
//!
//! ```rust,no_run
//! # #[cfg(all(feature = "ssr", feature = "test-utils"))]
//! # async fn demo() -> Result<(), leptos::prelude::ServerFnError> {
//! use std::sync::Arc;
//! use higgs::{Higgs, HiggsConfig, test_support::UnreachableValenceFactory};
//!
//! let config = Arc::new(
//!     HiggsConfig::builder()
//!         .valence_factory(UnreachableValenceFactory)
//!         // .chronon(scheduler, backend, registry)  // feature = "chronon"
//!         // .photon(photon)                         // feature = "photon"
//!         // .boson(backend)                         // feature = "boson"
//!         .build()
//!         .expect("valence_factory set"),
//! );
//! assert!(
//!     Arc::strong_count(&config) >= 1,
//!     "host boots with a shared Arc<HiggsConfig>"
//! );
//! // Host boot: leptos::provide_context(config.clone());
//!
//! let ctx = Higgs::from_request().await?;
//! let valence = ctx.valence().map_err(leptos::prelude::ServerFnError::new)?;
//! // Use `valence` for user-scoped reads/writes. With features chronon|boson|photon:
//! // let backend = ctx.chronon()?;
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
//! Observable outcome: `cargo run -p higgs --example server_fn_context --features ssr`
//! prints a success line after assembling request context and Valence. With backends
//! registered: `cargo run -p higgs --example server_fn_backends --features full,ssr`.
//!
//! Errors: missing `Arc<HiggsConfig>` → [`HiggsError::ConfigNotInContext`]. Subsystem
//! accessors without a matching builder call → [`HiggsError::SubsystemNotConfigured`].
//! Next: [Workers](#workers--chronon--boson--photon) for non-request Valence, or
//! [Macros and session](#macros-and-session) for `#[server]` wiring.
//!
//! # Workers — Chronon / Boson / Photon
//!
//! Recover Valence **inside** each family's handler macro (`#[script]`, `#[task]`,
//! `#[subscribe]`). Workers have no Leptos request, so they never call
//! `Higgs::from_request`. At boot, register the same `Arc<dyn valence::ValenceFactory>`
//! with Chronon / Boson / Photon identity adapters (see the runnable examples for that
//! wiring). Hosts often implement both [`HiggsValenceFactory`] (for [`HiggsConfig`]) and
//! `valence::ValenceFactory` on one type.
//!
//! ## Chronon — `#[script]`
//!
//! ```rust,ignore
//! use chronon_core::ScriptContext;
//! use chronon_valence_identity::valence_from_context;
//!
//! #[chronon_coordinator_macros::script(name = "higgs_demo_cleanup")]
//! async fn higgs_demo_cleanup(ctx: Box<dyn ScriptContext>) -> anyhow::Result<()> {
//!     let valence = valence_from_context(&*ctx)?;
//!     anyhow::ensure!(valence.actor().user_id() == Some("chronon-job-user"));
//!     assert_eq!(valence.actor().user_id(), Some("chronon-job-user"));
//!     Ok(())
//! }
//! ```
//!
//! ## Boson — `#[task]`
//!
//! ```rust,ignore
//! use boson_core::ExecutionContext;
//! use boson_macros::task;
//! use boson_valence_identity::valence_from_context;
//!
//! #[task(name = "higgs_demo_greet")]
//! async fn higgs_demo_greet(ctx: Box<dyn ExecutionContext>, name: String) -> boson_core::Result<()> {
//!     let valence = valence_from_context(ctx.as_ref())?;
//!     if valence.actor().user_id() != Some("boson-task-user") {
//!         return Err(boson_core::BosonError::internal(format!(
//!             "unexpected actor for greet({name})"
//!         )));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Photon — `#[subscribe]`
//!
//! Rebuild Valence from the published event's actor JSON (same factory Arc the host
//! passed to `start_executor`), not `from_request`:
//!
//! ```rust,ignore
//! use photon_macros::subscribe;
//! use photon_types::Event;
//! use valence::Actor;
//!
//! #[subscribe(topic = "higgs.demo.greeting", durable = "higgs-demo-logger")]
//! async fn on_higgs_demo_greeting(
//!     _actor: Box<dyn Actor>,
//!     event: HiggsDemoGreeting,
//!     transport: &Event,
//! ) -> photon::Result<()> {
//!     let factory = PROCESS_FACTORY
//!         .get()
//!         .ok_or_else(|| photon::PhotonError::Internal("valence factory not installed".into()))?;
//!     let valence = factory
//!         .build(&transport.actor_json)
//!         .map_err(|e| photon::PhotonError::Internal(e.to_string()))?;
//!     if valence.actor().user_id() != Some("photon-worker-user") {
//!         return Err(photon::PhotonError::Internal(format!(
//!             "unexpected actor for greeting {}",
//!             event.name
//!         )));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! Observable outcomes: `cargo run -p higgs --example chronon_job --features chronon`,
//! `boson_task` (`boson`), `photon_worker` (`photon`) — each prints a success line after
//! the macro handler recovers Valence and checks the actor.
//!
//! `Higgs::chronon()` / `boson()` / `photon()` remain for **SSR** code that needs to talk
//! to those backends (enqueue a job, publish an event). Next: [Preflight at startup](#preflight-at-startup)
//! before schedulers start.
//!
//! # Preflight at startup
//!
//! Startup checks and idempotent seed hooks the host runs once at boot. After the database
//! router is up and before background schedulers start, register `PreflightCheck`s, call
//! `PreflightRunner::run_all`, and keep the returned statuses (for logs or an auth-gated
//! setup UI). Prefer this for boot-visible validation and seeding instead of only Chronon
//! `RunOnce` jobs.
//!
//! Prerequisites: feature `preflight` (also enabled by `ssr`), database router installed,
//! before background schedulers start.
//!
//! Implement `preflight::PreflightCheck`, register on `preflight::PreflightRunner`,
//! run once, retain results for any setup UI.
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use async_trait::async_trait;
//! use higgs::preflight::{PreflightCheck, PreflightRunner, PreflightResult, PreflightStatus};
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
//!
//! Observable outcome:
//! `cargo run -p higgs --example preflight_boot --features preflight` prints
//! `preflight: demo-always-pass — Passed { … }`.
//!
//! Failures preserve check status on the runner results (failed checks are not silent).
//! Next: [SSR](#ssr--leptos-server-functions) or [Workers](#workers--chronon--boson--photon).
//!
//! # Macros and session
//!
//! Lets Leptos server functions set a task-local operation name (and optionally require a
//! signed-in session) before the body runs. Use this when writing `#[server]` handlers that
//! call `Higgs::from_request` per request.
//!
//! Prerequisites: package `higgs` with feature `ssr`, dependency on `higgs-macros`,
//! `provide_context(Arc<HiggsConfig>)`, and host middleware that inserts
//! `higgs_identity::SessionSnapshot` when using `(auth)`.
//!
//! ```rust,ignore
//! use higgs::Higgs;
//! use leptos::prelude::*;
//!
//! #[server]
//! #[higgs_macros::server(auth)]
//! pub async fn whoami() -> Result<String, ServerFnError> {
//!     let ctx = Higgs::from_request().await?;
//!     let valence = ctx.valence().map_err(ServerFnError::new)?;
//!     let user = valence
//!         .actor()
//!         .user_id()
//!         .ok_or_else(|| ServerFnError::new("expected User actor"))?;
//!     assert_eq!(user, "demo-user");
//!     Ok(user.to_string())
//! }
//! ```
//!
//! Variant: bare `#[higgs_macros::server]` (no `(auth)`) for public endpoints. With
//! `(auth)`, missing session fails closed. `unsafe_system_valence` picks up the
//! operation name the macro sets.
//!
//! Observable outcome: `cargo run -p higgs --example server_fn_context --features ssr`
//! prints a success line for the macro + context path.
//!
//! Config boot and `from_request` errors (`ConfigNotInContext`, subsystem accessors) live
//! under [SSR](#ssr--leptos-server-functions).
//!
//! `permission = "..."` on the macro is **not shipped** (compile-time error). Use
//! [`server_runtime`] for hand-rolled permission payloads.
//!
//! Next: open the `higgs-macros`, `higgs-identity`, and `higgs-host` crate docs for
//! attribute and extractor contracts (not Guide destinations on this page).
//!
//! # Notes
//!
//! - `higgs_core` owns Valence/session config and request context without Chronon /
//!   Boson / Photon accessors. Depend on this crate as package name **`higgs`** so
//!   `higgs-macros` keeps resolving `higgs::Higgs::from_request()`.
//! - Missing `Arc<HiggsConfig>` in Leptos context → [`HiggsError::ConfigNotInContext`].
//! - Subsystem accessors fail with [`HiggsError::SubsystemNotConfigured`] when the
//!   feature is on but the host never called the matching builder method.
//! - Session middleware should insert `higgs_identity::SessionSnapshot` in Axum
//!   extensions; `higgs_host::host_ctx` / `Higgs::from_request` read it for the actor.
mod config;
mod error;

#[cfg(feature = "ssr")]
mod context;

pub use config::{HiggsConfig, HiggsConfigBuilder};
pub use error::HiggsError;
pub use higgs_core::{actor_policy, server_runtime, HiggsValenceFactory};

#[cfg(feature = "preflight")]
pub use higgs_core::preflight;

#[cfg(feature = "ssr")]
pub use context::Higgs;

#[cfg(feature = "ssr")]
pub use higgs_host::{current_operation, require_session, with_operation};

#[cfg(any(test, feature = "test-utils"))]
pub use higgs_core::test_support;
