//! # higgs-macros — server-function wiring
//!
//! Attribute macros for Leptos server functions used with package `higgs`: set a
//! task-local operation name before the body runs, and optionally require a signed-in
//! session with `(auth)`.
//!
//! ## Features
//!
//! - **server** — Wraps Leptos `#[server]` and sets the task-local operation name before
//!   the body runs (read by `current_operation` / `unsafe_system_valence`).
//!   [Quick example](#quick-example)
//! - **server(auth)** — Same as `server`, and requires a signed-in session via
//!   `higgs::require_session`. [Quick example](#quick-example)
//!
//! `permission = "..."` on [`server`] is **not shipped** in this release (compile-time error
//! if used). `higgs_core::server_runtime` helpers exist for hand-rolled permission checks.
//!
//! # Getting started
//!
//! Depend on package `higgs` (feature `ssr`) and annotate Leptos server functions so Higgs
//! can attribute the call and optionally require a session.
//!
//! 1. Depend on `higgs` (package name **`higgs`**, feature `ssr`) and `higgs-macros`.
//! 2. Annotate server functions with Leptos `#[server]` and [`server`].
//! 3. Call `higgs::Higgs::from_request` inside the body; `unsafe_system_valence` picks up
//!    the function name as the operation (`higgs::with_operation` → `higgs_host` task-local).
//!
//! First success: `cargo run -p higgs --example server_fn_context --features ssr`
//! exercises the macro + request context path and prints a success line.
//!
//! # Quick example
//!
//! Write a public or `(auth)` server function that calls `Higgs::from_request` and reads
//! the session actor.
//!
//! Prerequisites: `higgs` with feature `ssr`, `provide_context(Arc<HiggsConfig>)`, and
//! session middleware when using `(auth)`.
//!
//! ```ignore
//! use higgs::Higgs;
//! use leptos::prelude::*;
//!
//! #[server]
//! #[higgs_macros::server]
//! pub async fn public_ping() -> Result<(), ServerFnError> {
//!     Ok(())
//! }
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
//!     assert!(!user.is_empty(), "session user id is non-empty");
//!     Ok(user.to_string())
//! }
//! ```
//!
//! Variant: bare `#[higgs_macros::server]` for public endpoints; `(auth)` fails closed
//! without a session. See package `higgs` crate-root docs for boot wiring and feature flags.
use proc_macro::TokenStream;

mod server;

/// Wrapper around Leptos `#[server]` that sets operation context before running
/// the function body.
///
/// Available without Cargo features on this crate. Pair with package `higgs`
/// feature `ssr` so `Higgs::from_request` compiles in the host.
///
/// # Usage
///
/// - `#[higgs_macros::server]` — public or self-authorized endpoints
/// - `#[higgs_macros::server(auth)]` — requires `SessionSnapshot` (signed-in)
///
/// `permission = "..."` is **not shipped** (compile-time error if used).
///
/// Runnable teaching path: `cargo run -p higgs --example server_fn_context --features ssr`.
///
/// # Examples
///
/// ```ignore
/// use higgs::Higgs;
/// use leptos::prelude::*;
///
/// #[server]
/// #[higgs_macros::server]
/// pub async fn public_ping() -> Result<(), ServerFnError> {
///     Ok(())
/// }
///
/// #[server]
/// #[higgs_macros::server(auth)]
/// pub async fn whoami() -> Result<String, ServerFnError> {
///     let ctx = Higgs::from_request().await?;
///     let valence = ctx.valence().map_err(ServerFnError::new)?;
///     let user = valence
///         .actor()
///         .user_id()
///         .ok_or_else(|| ServerFnError::new("expected User actor"))?;
///     assert!(!user.is_empty(), "session user id is non-empty");
///     Ok(user.to_string())
/// }
/// ```
#[proc_macro_attribute]
pub fn server(attr: TokenStream, input: TokenStream) -> TokenStream {
    server::expand_server(attr, input)
}
