//! Trait Resolution. See the [rustc-dev-guide] for more information on how this works.
//!
//! [rustc-dev-guide]: https://rustc-dev-guide.rust-lang.org/traits/resolution.html

mod engine;
pub mod error_reporting;
mod project;
mod structural_impls;
pub mod util;

use rustc_hir as hir;
use rustc_middle::ty::error::{ExpectedFound, TypeError};
use rustc_middle::ty::{self, Const, Ty};
use rustc_span::Span;

pub use self::FulfillmentErrorCode::*;
pub use self::ImplSource::*;
pub use self::SelectionError::*;
pub use rustc_middle::traits::select::ObligationCauseCode::*;

pub use self::engine::{TraitEngine, TraitEngineExt};
pub use self::project::MismatchedProjectionTypes;
pub(crate) use self::project::UndoLog;
pub use self::project::{
    Normalized, NormalizedTy, ProjectionCache, ProjectionCacheEntry, ProjectionCacheKey,
    ProjectionCacheStorage, Reveal,
};
pub use rustc_middle::traits::*;

pub type PredicateObligation<'tcx> = Obligation<'tcx, ty::Predicate<'tcx>>;
pub type TraitObligation<'tcx> = Obligation<'tcx, ty::PolyTraitPredicate<'tcx>>;

// `PredicateObligation` is used a lot. Make sure it doesn't unintentionally get bigger.
#[cfg(all(target_arch = "x86_64", target_pointer_width = "64"))]
static_assert_size!(PredicateObligation<'_>, 32);

pub type PredicateObligations<'tcx> = Vec<PredicateObligation<'tcx>>;

pub type Selection<'tcx> = ImplSource<'tcx, PredicateObligation<'tcx>>;

pub struct FulfillmentError<'tcx> {
    pub obligation: PredicateObligation<'tcx>,
    pub code: FulfillmentErrorCode<'tcx>,
    /// Diagnostics only: we opportunistically change the `code.span` when we encounter an
    /// obligation error caused by a call argument. When this is the case, we also signal that in
    /// this field to ensure accuracy of suggestions.
    pub points_at_arg_span: bool,
}

#[derive(Clone)]
pub enum FulfillmentErrorCode<'tcx> {
    CodeSelectionError(SelectionError<'tcx>),
    CodeProjectionError(MismatchedProjectionTypes<'tcx>),
    CodeSubtypeError(ExpectedFound<Ty<'tcx>>, TypeError<'tcx>), // always comes from a SubtypePredicate
    CodeConstEquateError(ExpectedFound<&'tcx Const<'tcx>>, TypeError<'tcx>),
    CodeAmbiguity,
    CodeOverflow,
}

impl<'tcx, O> Obligation<'tcx, O> {
    pub fn new(
        cause: ObligationCause<'tcx>,
        param_env: ty::ParamEnv<'tcx>,
        predicate: O,
    ) -> Obligation<'tcx, O> {
        Obligation { cause, param_env, recursion_depth: 0, predicate }
    }

    pub fn with_depth(
        cause: ObligationCause<'tcx>,
        recursion_depth: usize,
        param_env: ty::ParamEnv<'tcx>,
        predicate: O,
    ) -> Obligation<'tcx, O> {
        Obligation { cause, param_env, recursion_depth, predicate }
    }

    pub fn misc(
        span: Span,
        body_id: hir::HirId,
        param_env: ty::ParamEnv<'tcx>,
        trait_ref: O,
    ) -> Obligation<'tcx, O> {
        Obligation::new(ObligationCause::misc(span, body_id), param_env, trait_ref)
    }

    pub fn with<P>(&self, value: P) -> Obligation<'tcx, P> {
        Obligation {
            cause: self.cause.clone(),
            param_env: self.param_env,
            recursion_depth: self.recursion_depth,
            predicate: value,
        }
    }
}

impl<'tcx> FulfillmentError<'tcx> {
    pub fn new(
        obligation: PredicateObligation<'tcx>,
        code: FulfillmentErrorCode<'tcx>,
    ) -> FulfillmentError<'tcx> {
        FulfillmentError { obligation, code, points_at_arg_span: false }
    }
}

impl<'tcx> TraitObligation<'tcx> {
    pub fn self_ty(&self) -> ty::Binder<'tcx, Ty<'tcx>> {
        self.predicate.map_bound(|p| p.self_ty())
    }
}
