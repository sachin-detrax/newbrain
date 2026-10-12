//! Compatibility adapter for the benchmark-specific v2 evaluator seam.
//!
//! The reusable [`crate::framework`] and [`crate::runtime`] modules do not
//! depend on this trait. Keeping the bridge here lets the existing
//! `EvaluatorWorld` and experiment runner continue to use their receipt-based
//! transport while custom environments implement [`Environment`] directly.

use crate::framework::{DecisionReceipt, InteractionLedger, StepInput, StepResult};
use crate::runtime::{Environment, EnvironmentActionError, EnvironmentTurn};

/// Legacy v2 evaluator transport.
///
/// This is intentionally benchmark-facing. New environment implementations
/// should use [`Environment`] and [`EnvironmentTurn::resolve`] instead of
/// depending on this compatibility trait.
pub trait Evaluator {
    type Error: std::error::Error;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error>;

    fn apply_recorded(
        &mut self,
        ledger: &mut InteractionLedger,
        receipt: &DecisionReceipt,
    ) -> Result<Option<StepResult>, Self::Error>;
}

/// Preserve the MVP-1 evaluator bridge without making the reusable runtime
/// depend on evaluator-world types.
impl<T> Environment for T
where
    T: Evaluator,
{
    type Error = T::Error;

    fn reset(&mut self, ledger: &InteractionLedger) -> Result<StepInput, Self::Error> {
        Evaluator::reset(self, ledger)
    }

    fn apply(
        &mut self,
        turn: &mut EnvironmentTurn<'_>,
    ) -> Result<(), EnvironmentActionError<Self::Error>> {
        let receipt = turn.receipt().clone();
        let result = Evaluator::apply_recorded(self, turn.ledger_mut(), &receipt)
            .map_err(EnvironmentActionError::Environment)?;
        turn.adopt_recorded(result)
            .map_err(EnvironmentActionError::Ledger)
    }
}
