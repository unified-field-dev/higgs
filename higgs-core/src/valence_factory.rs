//! Host-side Valence construction contract (decoupled from valence `ssr` feature gating).

use valence::Valence;

/// Builds request-scoped [`Valence`] instances from serialized [`valence::Actor`] JSON.
///
/// Hosts implement this once and install it on [`HiggsConfig`](crate::HiggsConfig).
/// Prefer also implementing `valence::ValenceFactory` on the same type (or wrapping the
/// same `Arc`) so Chronon / Boson / Photon identity adapters share the factory with SSR.
///
/// SSR recovers Valence via `Higgs::valence` / `Higgs::from_request` (package `higgs`,
/// feature `ssr`). Workers recover Valence from family helpers
/// (`chronon_valence_identity::valence_from_context`,
/// `boson_valence_identity::valence_from_context`, Photon `Valence` handler params) — not
/// from `Higgs::from_request`.
///
/// See the [crate-root example](crate#quick-example) for builder wiring in this crate;
/// package `higgs` documents the full SSR + worker paths.
///
/// Runnable: `cargo run -p higgs --example shared_factory --features ssr`
///
/// # Examples
///
/// ```rust,no_run
/// # #[cfg(feature = "test-utils")]
/// # {
/// use higgs_core::HiggsValenceFactory;
/// use higgs_core::test_support::UnreachableValenceFactory;
///
/// // Host adapters usually delegate to RouterValenceFactory or a shared Arc<dyn ValenceFactory>.
/// let factory: &dyn HiggsValenceFactory = &UnreachableValenceFactory;
/// # let _ = factory;
/// # }
/// ```
pub trait HiggsValenceFactory: Send + Sync + 'static {
    /// Build a [`Valence`] scoped to the actor described by `actor_json`.
    ///
    /// # Errors
    ///
    /// Implementation-defined (`anyhow::Error`) when Valence cannot be constructed
    /// for the given actor JSON.
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence>;
}
