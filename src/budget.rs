use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BudgetKind {
    CandidatePairs,
    GeneratedCandidates,
    SupportComparisons,
    DevelopmentChecks,
    OperationSteps,
    ActiveTransitionRecords,
    ActiveCandidateRecords,
    RetrievedCandidates,
    CandidateChecks,
    ActiveRecords,
    TotalCandidateRecords,
    TotalEventRecords,
    FullStateRebuilds,
    HiddenStateAccesses,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BudgetScope {
    TrainingUpdate,
    Query,
    TotalRun,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BudgetConfig {
    limits: [(BudgetKind, u64); 14],
}

impl BudgetConfig {
    pub const fn v1() -> Self {
        Self {
            limits: [
                (BudgetKind::CandidatePairs, 16),
                (BudgetKind::GeneratedCandidates, 32),
                (BudgetKind::SupportComparisons, 128),
                (BudgetKind::DevelopmentChecks, 32),
                (BudgetKind::OperationSteps, 512),
                (BudgetKind::ActiveTransitionRecords, 32),
                (BudgetKind::ActiveCandidateRecords, 64),
                (BudgetKind::RetrievedCandidates, 16),
                (BudgetKind::CandidateChecks, 16),
                (BudgetKind::ActiveRecords, 16),
                (BudgetKind::TotalCandidateRecords, 128),
                (BudgetKind::TotalEventRecords, 128),
                (BudgetKind::FullStateRebuilds, 0),
                (BudgetKind::HiddenStateAccesses, 0),
            ],
        }
    }

    pub fn limit(self, kind: BudgetKind) -> u64 {
        self.limit_for(BudgetScope::TrainingUpdate, kind)
    }

    pub fn limit_for(self, scope: BudgetScope, kind: BudgetKind) -> u64 {
        if scope == BudgetScope::Query {
            return match kind {
                BudgetKind::RetrievedCandidates => 16,
                BudgetKind::CandidateChecks => 16,
                BudgetKind::OperationSteps => 64,
                BudgetKind::ActiveRecords => 16,
                _ => 0,
            };
        }
        if scope == BudgetScope::TotalRun {
            return match kind {
                BudgetKind::TotalCandidateRecords => 128,
                BudgetKind::TotalEventRecords => 128,
                BudgetKind::FullStateRebuilds => 0,
                BudgetKind::HiddenStateAccesses => 0,
                _ => 0,
            };
        }
        let mut index = 0;
        while index < self.limits.len() {
            if self.limits[index].0 == kind {
                return self.limits[index].1;
            }
            index += 1;
        }
        0
    }

    pub const fn limits(self) -> [(BudgetKind, u64); 14] {
        self.limits
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetError {
    scope: BudgetScope,
    kind: BudgetKind,
    limit: u64,
    used: u64,
    requested: u64,
}

impl BudgetError {
    pub const fn kind(&self) -> BudgetKind {
        self.kind
    }

    pub const fn scope(&self) -> BudgetScope {
        self.scope
    }

    pub const fn limit(&self) -> u64 {
        self.limit
    }

    pub const fn used(&self) -> u64 {
        self.used
    }

    pub const fn requested(&self) -> u64 {
        self.requested
    }
}

impl fmt::Display for BudgetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "budget {:?} exceeded: used {}, requested {}, limit {}",
            self.kind, self.used, self.requested, self.limit
        )
    }
}

impl std::error::Error for BudgetError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BudgetState {
    config: BudgetConfig,
    used: BTreeMap<BudgetKind, u64>,
    scoped_used: BTreeMap<(BudgetScope, BudgetKind), u64>,
    peaks: BTreeMap<BudgetKind, u64>,
    scoped_peaks: BTreeMap<(BudgetScope, BudgetKind), u64>,
}

impl BudgetState {
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            used: BTreeMap::new(),
            scoped_used: BTreeMap::new(),
            peaks: BTreeMap::new(),
            scoped_peaks: BTreeMap::new(),
        }
    }

    pub fn config(&self) -> BudgetConfig {
        self.config
    }

    pub fn used(&self, kind: BudgetKind) -> u64 {
        self.used.get(&kind).copied().unwrap_or(0)
    }

    pub fn peak(&self, kind: BudgetKind) -> u64 {
        self.peaks.get(&kind).copied().unwrap_or(0)
    }

    pub fn charge(&mut self, kind: BudgetKind, amount: u64) -> Result<(), BudgetError> {
        self.charge_in(BudgetScope::TrainingUpdate, kind, amount)
    }

    pub fn charge_query(&mut self, kind: BudgetKind, amount: u64) -> Result<(), BudgetError> {
        self.charge_in(BudgetScope::Query, kind, amount)
    }

    pub fn charge_total(&mut self, kind: BudgetKind, amount: u64) -> Result<(), BudgetError> {
        self.charge_in(BudgetScope::TotalRun, kind, amount)
    }

    pub fn charge_in(
        &mut self,
        scope: BudgetScope,
        kind: BudgetKind,
        amount: u64,
    ) -> Result<(), BudgetError> {
        let used = self.used_in(scope, kind);
        let limit = self.config.limit_for(scope, kind);
        let next = used.checked_add(amount).ok_or(BudgetError {
            scope,
            kind,
            limit,
            used,
            requested: amount,
        })?;

        if next > limit {
            return Err(BudgetError {
                scope,
                kind,
                limit,
                used,
                requested: amount,
            });
        }

        self.scoped_used.insert((scope, kind), next);
        if scope == BudgetScope::TrainingUpdate {
            self.used.insert(kind, next);
        }
        Ok(())
    }

    pub fn set_active(&mut self, kind: BudgetKind, amount: u64) -> Result<(), BudgetError> {
        self.set_active_in(BudgetScope::TrainingUpdate, kind, amount)
    }

    pub fn set_active_in(
        &mut self,
        scope: BudgetScope,
        kind: BudgetKind,
        amount: u64,
    ) -> Result<(), BudgetError> {
        let limit = self.config.limit_for(scope, kind);
        if amount > limit {
            return Err(BudgetError {
                scope,
                kind,
                limit,
                // Active limits report the previously accepted high-water mark.
                used: self.peak_in(scope, kind),
                requested: amount,
            });
        }
        self.scoped_peaks
            .entry((scope, kind))
            .and_modify(|peak| *peak = (*peak).max(amount))
            .or_insert(amount);
        if scope == BudgetScope::TrainingUpdate {
            self.peaks
                .entry(kind)
                .and_modify(|peak| *peak = (*peak).max(amount))
                .or_insert(amount);
        }
        Ok(())
    }

    pub fn used_in(&self, scope: BudgetScope, kind: BudgetKind) -> u64 {
        self.scoped_used.get(&(scope, kind)).copied().unwrap_or(0)
    }

    pub fn peak_in(&self, scope: BudgetScope, kind: BudgetKind) -> u64 {
        self.scoped_peaks.get(&(scope, kind)).copied().unwrap_or(0)
    }

    /// Start a new operation lifetime without resetting total-run accounting.
    pub fn reset_local(&mut self, scope: BudgetScope) {
        match scope {
            BudgetScope::TotalRun => {}
            BudgetScope::TrainingUpdate | BudgetScope::Query => {
                self.scoped_used.retain(|(s, _), _| *s != scope);
                self.scoped_peaks.retain(|(s, _), _| *s != scope);
                if scope == BudgetScope::TrainingUpdate {
                    self.used.clear();
                    self.peaks.clear();
                }
            }
        }
    }
}
