//! Reusable resource and scale measurement for the fixed opaque protocol.
//!
//! Logical charges are preflighted against decision, episode, and run limits
//! before any counter is changed. Resident memory is supplied through an
//! injectable sampler. [`ResourceMeter::runtime`] selects the standard-library
//! platform sampler, while [`ResourceMeter::new`] remains the deterministic
//! missing-memory constructor for tests and callers that do not want host
//! measurements. A missing sampler or missing sample remains
//! [`MemorySample::Missing`] and is never converted to zero. The frozen v2
//! constants remain available so benchmark artifacts do not change.

use std::fmt;
use std::time::Instant;

#[cfg(target_os = "linux")]
use std::fs;

pub const V2_PROTOCOL_VERSION: &str = "opaque-interactive-structural-discovery-v2-protocol-v1.0";
pub const V2_GRAMMAR_VERSION: &str = "opaque-interactive-structural-v2";

pub const WALL_CLOCK_LIMIT_NS: u64 = 60_000_000_000;
pub const PEAK_RESIDENT_MEMORY_LIMIT_BYTES: u64 = 536_870_912;
pub const PROCESS_COUNT_LIMIT: u64 = 1;
pub const WORKER_COUNT_LIMIT: u64 = 1;
pub const GPU_COUNT_LIMIT: u64 = 0;
pub const NETWORK_CONNECTION_COUNT_LIMIT: u64 = 0;

const LOGICAL_RESOURCE_COUNT: usize = 8;
const STAGE_COUNT: usize = 6;

/// Logical resources named by Section 5 of the frozen protocol.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum LogicalResource {
    QueryCalls,
    EnvironmentActions,
    CandidateGenerationAttempts,
    StructuralComparisons,
    ActiveHypotheses,
    OperationSteps,
    RetainedInteractionRecords,
    RetainedProcedureRecords,
}

impl LogicalResource {
    const fn index(self) -> usize {
        match self {
            Self::QueryCalls => 0,
            Self::EnvironmentActions => 1,
            Self::CandidateGenerationAttempts => 2,
            Self::StructuralComparisons => 3,
            Self::ActiveHypotheses => 4,
            Self::OperationSteps => 5,
            Self::RetainedInteractionRecords => 6,
            Self::RetainedProcedureRecords => 7,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::QueryCalls => "query_calls",
            Self::EnvironmentActions => "environment_actions",
            Self::CandidateGenerationAttempts => "candidate_generation_attempts",
            Self::StructuralComparisons => "structural_comparisons",
            Self::ActiveHypotheses => "active_hypotheses",
            Self::OperationSteps => "operation_steps",
            Self::RetainedInteractionRecords => "retained_interaction_records",
            Self::RetainedProcedureRecords => "retained_procedure_records",
        }
    }

    pub const fn all() -> [Self; LOGICAL_RESOURCE_COUNT] {
        [
            Self::QueryCalls,
            Self::EnvironmentActions,
            Self::CandidateGenerationAttempts,
            Self::StructuralComparisons,
            Self::ActiveHypotheses,
            Self::OperationSteps,
            Self::RetainedInteractionRecords,
            Self::RetainedProcedureRecords,
        ]
    }

    const fn is_gauge(self) -> bool {
        matches!(
            self,
            Self::ActiveHypotheses
                | Self::RetainedInteractionRecords
                | Self::RetainedProcedureRecords
        )
    }
}

/// The three fixed logical scopes in the protocol.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceScope {
    Decision,
    Episode,
    Run,
}

/// A fixed logical ceiling for one resource.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimit {
    per_decision: u64,
    per_episode: u64,
    per_run: u64,
}

impl ResourceLimit {
    pub const fn new(per_decision: u64, per_episode: u64, per_run: u64) -> Self {
        Self {
            per_decision,
            per_episode,
            per_run,
        }
    }

    pub const fn per_decision(self) -> u64 {
        self.per_decision
    }

    pub const fn per_episode(self) -> u64 {
        self.per_episode
    }

    pub const fn per_run(self) -> u64 {
        self.per_run
    }

    pub const fn for_scope(self, scope: ResourceScope) -> u64 {
        match scope {
            ResourceScope::Decision => self.per_decision,
            ResourceScope::Episode => self.per_episode,
            ResourceScope::Run => self.per_run,
        }
    }
}

/// The resource ceilings frozen by protocol v1.0.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolResourceLimits {
    logical: [ResourceLimit; LOGICAL_RESOURCE_COUNT],
}

impl ProtocolResourceLimits {
    pub const fn v1_0() -> Self {
        Self {
            logical: [
                ResourceLimit {
                    per_decision: 1,
                    per_episode: 2,
                    per_run: 72,
                },
                ResourceLimit {
                    per_decision: 1,
                    per_episode: 2,
                    per_run: 72,
                },
                ResourceLimit {
                    per_decision: 64,
                    per_episode: 128,
                    per_run: 512,
                },
                ResourceLimit {
                    per_decision: 256,
                    per_episode: 512,
                    per_run: 2_048,
                },
                ResourceLimit {
                    per_decision: 16,
                    per_episode: 16,
                    per_run: 64,
                },
                ResourceLimit {
                    per_decision: 1_024,
                    per_episode: 2_048,
                    per_run: 8_192,
                },
                ResourceLimit {
                    per_decision: 32,
                    per_episode: 32,
                    per_run: 128,
                },
                ResourceLimit {
                    per_decision: 32,
                    per_episode: 64,
                    per_run: 128,
                },
            ],
        }
    }

    pub const fn logical(self, resource: LogicalResource) -> ResourceLimit {
        self.logical[resource.index()]
    }

    pub(crate) const fn from_logical(logical: [ResourceLimit; LOGICAL_RESOURCE_COUNT]) -> Self {
        Self { logical }
    }

    pub const fn physical(self) -> PhysicalResourceLimits {
        PhysicalResourceLimits::v1_0()
    }
}

impl Default for ProtocolResourceLimits {
    fn default() -> Self {
        Self::v1_0()
    }
}

/// Fixed physical ceilings from protocol Section 5.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PhysicalResourceLimits {
    wall_clock_ns: u64,
    peak_resident_memory_bytes: u64,
    process_count: u64,
    worker_count: u64,
    gpu_count: u64,
    network_connection_count: u64,
}

impl PhysicalResourceLimits {
    pub const fn v1_0() -> Self {
        Self {
            wall_clock_ns: WALL_CLOCK_LIMIT_NS,
            peak_resident_memory_bytes: PEAK_RESIDENT_MEMORY_LIMIT_BYTES,
            process_count: PROCESS_COUNT_LIMIT,
            worker_count: WORKER_COUNT_LIMIT,
            gpu_count: GPU_COUNT_LIMIT,
            network_connection_count: NETWORK_CONNECTION_COUNT_LIMIT,
        }
    }

    pub const fn wall_clock_ns(self) -> u64 {
        self.wall_clock_ns
    }

    pub const fn peak_resident_memory_bytes(self) -> u64 {
        self.peak_resident_memory_bytes
    }

    pub const fn process_count(self) -> u64 {
        self.process_count
    }

    pub const fn worker_count(self) -> u64 {
        self.worker_count
    }

    pub const fn gpu_count(self) -> u64 {
        self.gpu_count
    }

    pub const fn network_connection_count(self) -> u64 {
        self.network_connection_count
    }
}

/// A logical budget failure. The meter is unchanged when this is returned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceLimitError {
    resource: LogicalResource,
    scope: ResourceScope,
    used: u64,
    requested: u64,
    limit: u64,
}

impl ResourceLimitError {
    pub const fn resource(self) -> LogicalResource {
        self.resource
    }

    pub const fn scope(self) -> ResourceScope {
        self.scope
    }

    pub const fn used(self) -> u64 {
        self.used
    }

    pub const fn requested(self) -> u64 {
        self.requested
    }

    pub const fn limit(self) -> u64 {
        self.limit
    }
}

impl fmt::Display for ResourceLimitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "v2 {} limit exceeded in {:?}: used {}, requested {}, limit {}",
            self.resource.name(),
            self.scope,
            self.used,
            self.requested,
            self.limit
        )
    }
}

impl std::error::Error for ResourceLimitError {}

/// Errors from the measurement seam or from physical validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurementError {
    LogicalLimit(ResourceLimitError),
    CounterOverflow(LogicalResource),
    StageCounterOverflow(Stage),
    StageAlreadyStarted(Stage),
    StageNotStarted(Stage),
    WallClockLimitExceeded {
        elapsed_ns: u64,
        limit_ns: u64,
    },
    MemoryLimitExceeded {
        observed_bytes: u64,
        limit_bytes: u64,
    },
    MissingMemoryMeasurement,
    DuplicateScalePoint(ScalePoint),
}

impl fmt::Display for MeasurementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LogicalLimit(error) => error.fmt(formatter),
            Self::CounterOverflow(resource) => {
                write!(formatter, "v2 {} counter overflow", resource.name())
            }
            Self::StageCounterOverflow(stage) => {
                write!(formatter, "v2 {} stage counter overflow", stage.name())
            }
            Self::StageAlreadyStarted(stage) => {
                write!(formatter, "v2 {} stage is already started", stage.name())
            }
            Self::StageNotStarted(stage) => {
                write!(formatter, "v2 {} stage is not started", stage.name())
            }
            Self::WallClockLimitExceeded {
                elapsed_ns,
                limit_ns,
            } => write!(
                formatter,
                "v2 wall clock {} exceeds {}",
                elapsed_ns, limit_ns
            ),
            Self::MemoryLimitExceeded {
                observed_bytes,
                limit_bytes,
            } => write!(
                formatter,
                "v2 resident memory {} exceeds {}",
                observed_bytes, limit_bytes
            ),
            Self::MissingMemoryMeasurement => write!(formatter, "v2 resident memory is missing"),
            Self::DuplicateScalePoint(point) => {
                write!(formatter, "duplicate deterministic scale point {:?}", point)
            }
        }
    }
}

impl std::error::Error for MeasurementError {}

impl From<ResourceLimitError> for MeasurementError {
    fn from(error: ResourceLimitError) -> Self {
        Self::LogicalLimit(error)
    }
}

/// The eight required logical counters plus the ledger byte count required by
/// the protocol resource report.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LogicalCounters {
    query_calls: u64,
    environment_actions: u64,
    candidate_generation_attempts: u64,
    structural_comparisons: u64,
    active_hypotheses: u64,
    operation_steps: u64,
    retained_interaction_records: u64,
    retained_procedure_records: u64,
    ledger_size_bytes: u64,
}

impl LogicalCounters {
    #[allow(clippy::too_many_arguments)]
    pub(crate) const fn from_values(
        query_calls: u64,
        environment_actions: u64,
        candidate_generation_attempts: u64,
        structural_comparisons: u64,
        active_hypotheses: u64,
        operation_steps: u64,
        retained_interaction_records: u64,
        retained_procedure_records: u64,
        ledger_size_bytes: u64,
    ) -> Self {
        Self {
            query_calls,
            environment_actions,
            candidate_generation_attempts,
            structural_comparisons,
            active_hypotheses,
            operation_steps,
            retained_interaction_records,
            retained_procedure_records,
            ledger_size_bytes,
        }
    }

    pub const fn query_calls(self) -> u64 {
        self.query_calls
    }

    pub const fn environment_actions(self) -> u64 {
        self.environment_actions
    }

    pub const fn candidate_generation_attempts(self) -> u64 {
        self.candidate_generation_attempts
    }

    pub const fn candidate_count(self) -> u64 {
        self.candidate_generation_attempts
    }

    pub const fn structural_comparisons(self) -> u64 {
        self.structural_comparisons
    }

    pub const fn active_hypotheses(self) -> u64 {
        self.active_hypotheses
    }

    pub const fn operation_steps(self) -> u64 {
        self.operation_steps
    }

    pub const fn retained_interaction_records(self) -> u64 {
        self.retained_interaction_records
    }

    pub const fn ledger_record_count(self) -> u64 {
        self.retained_interaction_records
    }

    pub const fn retained_procedure_records(self) -> u64 {
        self.retained_procedure_records
    }

    pub const fn procedure_count(self) -> u64 {
        self.retained_procedure_records
    }

    pub const fn ledger_size_bytes(self) -> u64 {
        self.ledger_size_bytes
    }

    fn value(self, resource: LogicalResource) -> u64 {
        match resource {
            LogicalResource::QueryCalls => self.query_calls,
            LogicalResource::EnvironmentActions => self.environment_actions,
            LogicalResource::CandidateGenerationAttempts => self.candidate_generation_attempts,
            LogicalResource::StructuralComparisons => self.structural_comparisons,
            LogicalResource::ActiveHypotheses => self.active_hypotheses,
            LogicalResource::OperationSteps => self.operation_steps,
            LogicalResource::RetainedInteractionRecords => self.retained_interaction_records,
            LogicalResource::RetainedProcedureRecords => self.retained_procedure_records,
        }
    }

    fn set_value(&mut self, resource: LogicalResource, value: u64) {
        match resource {
            LogicalResource::QueryCalls => self.query_calls = value,
            LogicalResource::EnvironmentActions => self.environment_actions = value,
            LogicalResource::CandidateGenerationAttempts => {
                self.candidate_generation_attempts = value
            }
            LogicalResource::StructuralComparisons => self.structural_comparisons = value,
            LogicalResource::ActiveHypotheses => self.active_hypotheses = value,
            LogicalResource::OperationSteps => self.operation_steps = value,
            LogicalResource::RetainedInteractionRecords => {
                self.retained_interaction_records = value
            }
            LogicalResource::RetainedProcedureRecords => self.retained_procedure_records = value,
        }
    }

    fn add(&mut self, resource: LogicalResource, amount: u64) -> Result<(), MeasurementError> {
        let next = self
            .value(resource)
            .checked_add(amount)
            .ok_or(MeasurementError::CounterOverflow(resource))?;
        self.set_value(resource, next);
        Ok(())
    }

    pub fn to_json(self) -> String {
        format!(
            "{{\"active_hypotheses\":{},\"candidate_generation_attempts\":{},\"environment_actions\":{},\"ledger_record_count\":{},\"ledger_size_bytes\":{},\"operation_steps\":{},\"procedure_count\":{},\"query_calls\":{},\"retained_interaction_records\":{},\"retained_procedure_records\":{},\"structural_comparisons\":{}}}",
            self.active_hypotheses,
            self.candidate_generation_attempts,
            self.environment_actions,
            self.retained_interaction_records,
            self.ledger_size_bytes,
            self.operation_steps,
            self.retained_procedure_records,
            self.query_calls,
            self.retained_interaction_records,
            self.retained_procedure_records,
            self.structural_comparisons,
        )
    }
}

/// The six independently reportable v2 work stages.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Stage {
    Canonicalization,
    Retrieval,
    Admission,
    Revalidation,
    CandidateGeneration,
    DecisionAction,
}

impl Stage {
    const fn index(self) -> usize {
        match self {
            Self::Canonicalization => 0,
            Self::Retrieval => 1,
            Self::Admission => 2,
            Self::Revalidation => 3,
            Self::CandidateGeneration => 4,
            Self::DecisionAction => 5,
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Canonicalization => "canonicalization",
            Self::Retrieval => "retrieval",
            Self::Admission => "admission",
            Self::Revalidation => "revalidation",
            Self::CandidateGeneration => "candidate_generation",
            Self::DecisionAction => "decision_action",
        }
    }

    pub const fn all() -> [Self; STAGE_COUNT] {
        [
            Self::Canonicalization,
            Self::Retrieval,
            Self::Admission,
            Self::Revalidation,
            Self::CandidateGeneration,
            Self::DecisionAction,
        ]
    }
}

/// Deterministic logical accounting state used by framework persistence.
///
/// Wall-clock and resident-memory samples are intentionally excluded: they are
/// process measurements, not learner state. Logical counters, limits, and
/// stage operation counts are sufficient to continue bounded accounting after
/// a restart.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourcePersistenceSnapshot {
    limits: ProtocolResourceLimits,
    run_counters: LogicalCounters,
    decision_counters: LogicalCounters,
    episode_counters: LogicalCounters,
    stage_operations: [u64; STAGE_COUNT],
}

impl ResourcePersistenceSnapshot {
    pub(crate) const fn from_parts(
        limits: ProtocolResourceLimits,
        run_counters: LogicalCounters,
        decision_counters: LogicalCounters,
        episode_counters: LogicalCounters,
        stage_operations: [u64; STAGE_COUNT],
    ) -> Self {
        Self {
            limits,
            run_counters,
            decision_counters,
            episode_counters,
            stage_operations,
        }
    }

    pub const fn limits(self) -> ProtocolResourceLimits {
        self.limits
    }

    pub const fn run_counters(self) -> LogicalCounters {
        self.run_counters
    }

    pub const fn decision_counters(self) -> LogicalCounters {
        self.decision_counters
    }

    pub const fn episode_counters(self) -> LogicalCounters {
        self.episode_counters
    }

    pub const fn stage_operations(self, stage: Stage) -> u64 {
        self.stage_operations[stage.index()]
    }
}

/// Resident-memory state that preserves the difference between zero bytes and
/// no measurement.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MemorySample {
    #[default]
    Missing,
    Measured(u64),
}

impl MemorySample {
    pub const fn is_missing(self) -> bool {
        matches!(self, Self::Missing)
    }

    pub const fn bytes(self) -> Option<u64> {
        match self {
            Self::Missing => None,
            Self::Measured(bytes) => Some(bytes),
        }
    }

    fn value_json(self) -> String {
        match self {
            Self::Missing => String::from("null"),
            Self::Measured(bytes) => bytes.to_string(),
        }
    }

    const fn state_name(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Measured(_) => "measured",
        }
    }
}

/// Injectable seam for platform-specific resident-memory high-water samples.
pub trait ResidentMemorySampler {
    fn sample_peak_resident_memory_bytes(&mut self) -> Option<u64>;
}

impl<F> ResidentMemorySampler for F
where
    F: FnMut() -> Option<u64>,
{
    fn sample_peak_resident_memory_bytes(&mut self) -> Option<u64> {
        self()
    }
}

/// The runtime sampler is deliberately implemented without a dependency on a
/// platform crate. Each supported platform reports the current process's
/// resident/working-set bytes; [`ResourceMeter`] turns successive samples into
/// a high-water value.
#[derive(Default)]
struct PlatformResidentMemorySampler;

impl ResidentMemorySampler for PlatformResidentMemorySampler {
    fn sample_peak_resident_memory_bytes(&mut self) -> Option<u64> {
        platform_resident_memory_bytes()
    }
}

#[cfg(target_os = "linux")]
fn platform_resident_memory_bytes() -> Option<u64> {
    // VmRSS is the current resident set in KiB. Using /proc/self/status avoids
    // assuming a page size while remaining a std-only Linux sampler.
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name != "VmRSS" {
            return None;
        }
        let mut fields = value.split_whitespace();
        let kib = fields.next()?.parse::<u64>().ok()?;
        if fields.next()? != "kB" {
            return None;
        }
        kib.checked_mul(1024)
    })
}

#[cfg(all(unix, not(target_os = "linux")))]
fn platform_resident_memory_bytes() -> Option<u64> {
    // /proc/self/status is Linux-specific. Keep unsupported Unix targets
    // explicitly missing rather than substituting a fabricated value.
    None
}

#[cfg(windows)]
fn platform_resident_memory_bytes() -> Option<u64> {
    windows_platform::current_working_set_bytes()
}

#[cfg(not(any(unix, windows)))]
fn platform_resident_memory_bytes() -> Option<u64> {
    None
}

#[cfg(windows)]
mod windows_platform {
    use std::ffi::c_void;
    use std::mem::size_of;

    #[repr(C)]
    struct ProcessMemoryCounters {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut c_void;
    }

    #[link(name = "psapi")]
    unsafe extern "system" {
        fn GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCounters,
            size: u32,
        ) -> i32;
    }

    pub(super) fn current_working_set_bytes() -> Option<u64> {
        let mut counters = ProcessMemoryCounters {
            cb: u32::try_from(size_of::<ProcessMemoryCounters>()).ok()?,
            page_fault_count: 0,
            peak_working_set_size: 0,
            working_set_size: 0,
            quota_peak_paged_pool_usage: 0,
            quota_paged_pool_usage: 0,
            quota_peak_non_paged_pool_usage: 0,
            quota_non_paged_pool_usage: 0,
            pagefile_usage: 0,
            peak_pagefile_usage: 0,
        };
        let size = counters.cb;
        let success = unsafe { GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, size) };
        (success != 0).then_some(counters.working_set_size as u64)
    }
}

/// Per-stage counter and physical samples.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StageMeasurement {
    operation_count: u64,
    wall_clock_ns: u64,
    peak_resident_memory: MemorySample,
}

impl StageMeasurement {
    pub const fn operation_count(self) -> u64 {
        self.operation_count
    }

    pub const fn wall_clock_ns(self) -> u64 {
        self.wall_clock_ns
    }

    pub const fn peak_resident_memory(self) -> MemorySample {
        self.peak_resident_memory
    }

    fn to_json(self) -> String {
        format!(
            "{{\"operation_count\":{},\"peak_resident_memory_bytes\":{},\"peak_resident_memory_state\":\"{}\",\"wall_clock_ns\":{}}}",
            self.operation_count,
            self.peak_resident_memory.value_json(),
            self.peak_resident_memory.state_name(),
            self.wall_clock_ns,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ScopeCounters {
    values: LogicalCounters,
}

impl ScopeCounters {
    fn value(&self, resource: LogicalResource) -> u64 {
        self.values.value(resource)
    }

    fn add(&mut self, resource: LogicalResource, amount: u64) -> Result<(), MeasurementError> {
        self.values.add(resource, amount)
    }

    fn set(&mut self, resource: LogicalResource, value: u64) {
        self.values.set_value(resource, value);
    }
}

/// A snapshot suitable for a deterministic resource report or scale result.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceSnapshot {
    counters: LogicalCounters,
    wall_clock_ns: u64,
    peak_resident_memory: MemorySample,
    stages: [StageMeasurement; STAGE_COUNT],
}

impl ResourceSnapshot {
    pub const fn counters(self) -> LogicalCounters {
        self.counters
    }

    pub const fn wall_clock_ns(self) -> u64 {
        self.wall_clock_ns
    }

    pub const fn peak_resident_memory(self) -> MemorySample {
        self.peak_resident_memory
    }

    pub const fn stage(self, stage: Stage) -> StageMeasurement {
        self.stages[stage.index()]
    }

    pub fn to_json(self) -> String {
        format!(
            "{{\"counters\":{},\"peak_resident_memory_bytes\":{},\"peak_resident_memory_state\":\"{}\",\"stages\":{},\"wall_clock_ns\":{}}}",
            self.counters.to_json(),
            self.peak_resident_memory.value_json(),
            self.peak_resident_memory.state_name(),
            stages_json(&self.stages),
            self.wall_clock_ns,
        )
    }
}

fn stages_json(stages: &[StageMeasurement; STAGE_COUNT]) -> String {
    let mut output = String::from("{");
    for (index, stage) in Stage::all().into_iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push('"');
        output.push_str(stage.name());
        output.push_str("\":");
        output.push_str(&stages[stage.index()].to_json());
    }
    output.push('}');
    output
}

/// A v2 logical/physical meter. The clock starts at construction and uses
/// `Instant`, so elapsed wall time is monotonic and not affected by wall-clock
/// adjustments.
pub struct ResourceMeter {
    limits: ProtocolResourceLimits,
    counters: LogicalCounters,
    decision: ScopeCounters,
    episode: ScopeCounters,
    started_at: Instant,
    stage_started_at: [Option<Instant>; STAGE_COUNT],
    stages: [StageMeasurement; STAGE_COUNT],
    memory_sampler: Option<Box<dyn ResidentMemorySampler>>,
    peak_resident_memory: MemorySample,
}

impl Default for ResourceMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceMeter {
    /// Construct a meter for a real v2 execution.
    ///
    /// The platform sampler is intentionally not installed by [`Self::new`],
    /// so deterministic resource tests can continue to assert that an
    /// unconfigured meter reports [`MemorySample::Missing`].
    pub fn runtime() -> Self {
        Self::with_optional_memory_sampler(Some(Box::new(PlatformResidentMemorySampler)))
    }

    pub fn new() -> Self {
        Self::with_optional_memory_sampler(None)
    }

    pub fn with_memory_sampler<S>(sampler: S) -> Self
    where
        S: ResidentMemorySampler + 'static,
    {
        Self::with_optional_memory_sampler(Some(Box::new(sampler)))
    }

    pub fn set_memory_sampler<S>(&mut self, sampler: S)
    where
        S: ResidentMemorySampler + 'static,
    {
        self.memory_sampler = Some(Box::new(sampler));
    }

    pub fn clear_memory_sampler(&mut self) {
        self.memory_sampler = None;
    }

    fn with_optional_memory_sampler(
        memory_sampler: Option<Box<dyn ResidentMemorySampler>>,
    ) -> Self {
        Self {
            limits: ProtocolResourceLimits::v1_0(),
            counters: LogicalCounters::default(),
            decision: ScopeCounters::default(),
            episode: ScopeCounters::default(),
            started_at: Instant::now(),
            stage_started_at: std::array::from_fn(|_| None),
            stages: [StageMeasurement::default(); STAGE_COUNT],
            memory_sampler,
            peak_resident_memory: MemorySample::Missing,
        }
    }

    pub const fn limits(&self) -> ProtocolResourceLimits {
        self.limits
    }

    pub const fn counters(&self) -> LogicalCounters {
        self.counters
    }

    pub fn persistence_snapshot(&self) -> ResourcePersistenceSnapshot {
        let stage_operations = std::array::from_fn(|index| self.stages[index].operation_count);
        ResourcePersistenceSnapshot::from_parts(
            self.limits,
            self.counters,
            self.decision.values,
            self.episode.values,
            stage_operations,
        )
    }

    /// Restore logical accounting without restoring process-local clocks or
    /// memory samplers. The caller must validate the snapshot against its
    /// declared limits before using it as a durable state record.
    pub fn from_persistence(snapshot: ResourcePersistenceSnapshot) -> Self {
        let mut meter = Self::with_optional_memory_sampler(None);
        meter.limits = snapshot.limits;
        meter.counters = snapshot.run_counters;
        meter.decision = ScopeCounters {
            values: snapshot.decision_counters,
        };
        meter.episode = ScopeCounters {
            values: snapshot.episode_counters,
        };
        for stage in Stage::all() {
            meter.stages[stage.index()].operation_count = snapshot.stage_operations[stage.index()];
        }
        meter
    }

    pub fn begin_episode(&mut self) {
        self.episode = ScopeCounters::default();
        self.decision = ScopeCounters::default();
    }

    pub fn begin_decision(&mut self) {
        self.decision = ScopeCounters::default();
    }

    pub fn elapsed_wall_clock_ns(&self) -> u64 {
        duration_ns(self.started_at.elapsed())
    }

    pub fn peak_resident_memory(&self) -> MemorySample {
        self.peak_resident_memory
    }

    /// Take one sample and update the run and every currently active stage.
    pub fn sample_peak_resident_memory(&mut self) -> MemorySample {
        let sample = self
            .memory_sampler
            .as_mut()
            .and_then(|sampler| sampler.sample_peak_resident_memory_bytes());

        if let Some(bytes) = sample {
            self.peak_resident_memory = max_memory(self.peak_resident_memory, bytes);
            for stage in Stage::all() {
                if self.stage_started_at[stage.index()].is_some() {
                    let current = self.stages[stage.index()].peak_resident_memory;
                    self.stages[stage.index()].peak_resident_memory = max_memory(current, bytes);
                }
            }
        }
        self.peak_resident_memory
    }

    /// Charge a cumulative logical resource. All three scope checks happen
    /// before any scope or run counter is changed.
    pub fn try_charge(
        &mut self,
        resource: LogicalResource,
        amount: u64,
    ) -> Result<(), MeasurementError> {
        if resource.is_gauge() {
            return self.set_gauge(resource, amount);
        }

        self.preflight_charge(resource, amount, ResourceScope::Decision)?;
        self.preflight_charge(resource, amount, ResourceScope::Episode)?;
        self.preflight_charge(resource, amount, ResourceScope::Run)?;

        self.decision.add(resource, amount)?;
        self.episode.add(resource, amount)?;
        self.counters.add(resource, amount)?;
        Ok(())
    }

    pub fn record_query_call(&mut self) -> Result<(), MeasurementError> {
        self.preflight_stage_operation(Stage::DecisionAction, 1)?;
        self.try_charge(LogicalResource::QueryCalls, 1)?;
        self.record_stage_operation_unchecked(Stage::DecisionAction, 1);
        Ok(())
    }

    /// Read-only preflight for a prospective cumulative charge. Selection uses
    /// this to abstain before an action would exceed the frozen ceilings.
    pub fn check_charge(
        &self,
        resource: LogicalResource,
        amount: u64,
    ) -> Result<(), MeasurementError> {
        self.preflight_charge(resource, amount, ResourceScope::Decision)?;
        self.preflight_charge(resource, amount, ResourceScope::Episode)?;
        self.preflight_charge(resource, amount, ResourceScope::Run)
    }

    pub fn record_environment_action(&mut self) -> Result<(), MeasurementError> {
        self.preflight_stage_operation(Stage::DecisionAction, 1)?;
        self.try_charge(LogicalResource::EnvironmentActions, 1)?;
        self.record_stage_operation_unchecked(Stage::DecisionAction, 1);
        Ok(())
    }

    pub fn record_candidate_generation(&mut self) -> Result<(), MeasurementError> {
        self.preflight_stage_operation(Stage::CandidateGeneration, 1)?;
        self.try_charge(LogicalResource::CandidateGenerationAttempts, 1)?;
        self.record_stage_operation_unchecked(Stage::CandidateGeneration, 1);
        Ok(())
    }

    pub fn record_structural_comparison(&mut self) -> Result<(), MeasurementError> {
        self.try_charge(LogicalResource::StructuralComparisons, 1)
    }

    pub fn record_operation_step(&mut self) -> Result<(), MeasurementError> {
        self.try_charge(LogicalResource::OperationSteps, 1)
    }

    pub fn set_active_hypotheses(&mut self, count: u64) -> Result<(), MeasurementError> {
        self.set_gauge(LogicalResource::ActiveHypotheses, count)
    }

    pub fn set_retained_interaction_records(&mut self, count: u64) -> Result<(), MeasurementError> {
        self.set_gauge(LogicalResource::RetainedInteractionRecords, count)
    }

    pub fn set_retained_procedure_records(&mut self, count: u64) -> Result<(), MeasurementError> {
        self.set_gauge(LogicalResource::RetainedProcedureRecords, count)
    }

    pub fn set_ledger_size_bytes(&mut self, bytes: u64) {
        self.counters.ledger_size_bytes = bytes;
    }

    pub fn record_ledger_record(&mut self) -> Result<(), MeasurementError> {
        let next = self
            .episode
            .value(LogicalResource::RetainedInteractionRecords)
            .checked_add(1)
            .ok_or(MeasurementError::CounterOverflow(
                LogicalResource::RetainedInteractionRecords,
            ))?;
        self.set_retained_interaction_records(next)
    }

    pub fn record_stage_operation(
        &mut self,
        stage: Stage,
        amount: u64,
    ) -> Result<(), MeasurementError> {
        self.preflight_stage_operation(stage, amount)?;
        self.record_stage_operation_unchecked(stage, amount);
        Ok(())
    }

    pub fn start_stage(&mut self, stage: Stage) -> Result<(), MeasurementError> {
        let slot = &mut self.stage_started_at[stage.index()];
        if slot.is_some() {
            return Err(MeasurementError::StageAlreadyStarted(stage));
        }
        *slot = Some(Instant::now());
        Ok(())
    }

    pub fn finish_stage(&mut self, stage: Stage) -> Result<(), MeasurementError> {
        let started = self.stage_started_at[stage.index()]
            .take()
            .ok_or(MeasurementError::StageNotStarted(stage))?;
        let elapsed = duration_ns(started.elapsed());
        let current = self.stages[stage.index()].wall_clock_ns;
        self.stages[stage.index()].wall_clock_ns = current.saturating_add(elapsed);
        self.sample_peak_resident_memory();
        Ok(())
    }

    pub fn stage(&self, stage: Stage) -> StageMeasurement {
        self.stages[stage.index()]
    }

    pub fn snapshot(&self) -> ResourceSnapshot {
        ResourceSnapshot {
            counters: self.counters,
            wall_clock_ns: self.elapsed_wall_clock_ns(),
            peak_resident_memory: self.peak_resident_memory,
            stages: self.stages,
        }
    }

    pub fn sample_and_snapshot(&mut self) -> ResourceSnapshot {
        self.sample_peak_resident_memory();
        self.snapshot()
    }

    /// Validate physical limits and completeness at the report boundary.
    /// Logical limits are already enforced atomically at each charge.
    pub fn validate_limits(&self) -> Result<(), MeasurementError> {
        let physical = self.limits.physical();
        let elapsed_ns = self.elapsed_wall_clock_ns();
        if elapsed_ns > physical.wall_clock_ns() {
            return Err(MeasurementError::WallClockLimitExceeded {
                elapsed_ns,
                limit_ns: physical.wall_clock_ns(),
            });
        }

        let memory = match self.peak_resident_memory {
            MemorySample::Missing => return Err(MeasurementError::MissingMemoryMeasurement),
            MemorySample::Measured(bytes) => bytes,
        };
        if memory > physical.peak_resident_memory_bytes() {
            return Err(MeasurementError::MemoryLimitExceeded {
                observed_bytes: memory,
                limit_bytes: physical.peak_resident_memory_bytes(),
            });
        }
        Ok(())
    }

    fn preflight_charge(
        &self,
        resource: LogicalResource,
        amount: u64,
        scope: ResourceScope,
    ) -> Result<(), MeasurementError> {
        let used = match scope {
            ResourceScope::Decision => self.decision.value(resource),
            ResourceScope::Episode => self.episode.value(resource),
            ResourceScope::Run => self.counters.value(resource),
        };
        let limit = self.limits.logical(resource).for_scope(scope);
        let crossed = used.checked_add(amount).is_none_or(|next| next > limit);
        if crossed {
            return Err(ResourceLimitError {
                resource,
                scope,
                used,
                requested: amount,
                limit,
            }
            .into());
        }
        Ok(())
    }

    fn set_gauge(&mut self, resource: LogicalResource, value: u64) -> Result<(), MeasurementError> {
        debug_assert!(resource.is_gauge());
        for scope in [
            ResourceScope::Decision,
            ResourceScope::Episode,
            ResourceScope::Run,
        ] {
            let limit = self.limits.logical(resource).for_scope(scope);
            if value > limit {
                let used = match scope {
                    ResourceScope::Decision => self.decision.value(resource),
                    ResourceScope::Episode => self.episode.value(resource),
                    ResourceScope::Run => self.counters.value(resource),
                };
                return Err(ResourceLimitError {
                    resource,
                    scope,
                    used,
                    requested: value,
                    limit,
                }
                .into());
            }
        }

        self.decision.set(resource, value);
        self.episode.set(resource, value);
        let high_water = self.counters.value(resource).max(value);
        self.counters.set_value(resource, high_water);
        Ok(())
    }

    fn preflight_stage_operation(&self, stage: Stage, amount: u64) -> Result<(), MeasurementError> {
        self.stages[stage.index()]
            .operation_count
            .checked_add(amount)
            .map(|_| ())
            .ok_or(MeasurementError::StageCounterOverflow(stage))
    }

    fn record_stage_operation_unchecked(&mut self, stage: Stage, amount: u64) {
        self.stages[stage.index()].operation_count += amount;
    }
}

fn duration_ns(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

fn max_memory(current: MemorySample, next: u64) -> MemorySample {
    match current {
        MemorySample::Missing => MemorySample::Measured(next),
        MemorySample::Measured(previous) => MemorySample::Measured(previous.max(next)),
    }
}

/// One deterministic point in a scale sweep.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ScalePoint {
    symbol_cardinality: u64,
    record_count: u64,
    candidate_count: u64,
    retained_procedures: u64,
}

impl ScalePoint {
    pub const fn new(
        symbol_cardinality: u64,
        record_count: u64,
        candidate_count: u64,
        retained_procedures: u64,
    ) -> Self {
        Self {
            symbol_cardinality,
            record_count,
            candidate_count,
            retained_procedures,
        }
    }

    pub const fn symbol_cardinality(self) -> u64 {
        self.symbol_cardinality
    }

    pub const fn record_count(self) -> u64 {
        self.record_count
    }

    pub const fn candidate_count(self) -> u64 {
        self.candidate_count
    }

    pub const fn retained_procedures(self) -> u64 {
        self.retained_procedures
    }

    fn to_json(self) -> String {
        format!(
            "{{\"candidate_count\":{},\"record_count\":{},\"retained_procedures\":{},\"symbol_cardinality\":{}}}",
            self.candidate_count,
            self.record_count,
            self.retained_procedures,
            self.symbol_cardinality,
        )
    }
}

/// Build a canonical Cartesian sweep. Each input dimension is sorted and
/// deduplicated first, so caller ordering cannot affect the points or their
/// serialization.
pub fn deterministic_scale_points(
    symbol_cardinalities: &[u64],
    record_counts: &[u64],
    candidate_counts: &[u64],
    retained_procedure_counts: &[u64],
) -> Vec<ScalePoint> {
    let symbols = sorted_unique(symbol_cardinalities);
    let records = sorted_unique(record_counts);
    let candidates = sorted_unique(candidate_counts);
    let procedures = sorted_unique(retained_procedure_counts);
    let mut points = Vec::new();

    for symbol_cardinality in symbols {
        for record_count in &records {
            for candidate_count in &candidates {
                for retained_procedures in &procedures {
                    points.push(ScalePoint::new(
                        symbol_cardinality,
                        *record_count,
                        *candidate_count,
                        *retained_procedures,
                    ));
                }
            }
        }
    }
    points
}

fn sorted_unique(values: &[u64]) -> Vec<u64> {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    sorted
}

/// A measured result associated with one scale point.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScaleResult {
    point: ScalePoint,
    snapshot: ResourceSnapshot,
}

impl ScaleResult {
    pub const fn new(point: ScalePoint, snapshot: ResourceSnapshot) -> Self {
        Self { point, snapshot }
    }

    pub const fn point(self) -> ScalePoint {
        self.point
    }

    pub const fn snapshot(self) -> ResourceSnapshot {
        self.snapshot
    }

    fn to_json(self) -> String {
        format!(
            "{{\"counters\":{},\"peak_resident_memory_bytes\":{},\"peak_resident_memory_state\":\"{}\",\"point\":{},\"stages\":{},\"wall_clock_ns\":{}}}",
            self.snapshot.counters.to_json(),
            self.snapshot.peak_resident_memory.value_json(),
            self.snapshot.peak_resident_memory.state_name(),
            self.point.to_json(),
            stages_json(&self.snapshot.stages),
            self.snapshot.wall_clock_ns,
        )
    }
}

/// Canonically ordered deterministic scale results.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaleSweep {
    results: Vec<ScaleResult>,
}

impl ScaleSweep {
    pub fn new(mut results: Vec<ScaleResult>) -> Self {
        results.sort_by_key(|result| result.point);
        Self { results }
    }

    pub fn try_new(results: Vec<ScaleResult>) -> Result<Self, MeasurementError> {
        let sweep = Self::new(results);
        for pair in sweep.results.windows(2) {
            if pair[0].point == pair[1].point {
                return Err(MeasurementError::DuplicateScalePoint(pair[0].point));
            }
        }
        Ok(sweep)
    }

    pub fn from_points(points: &[ScalePoint]) -> Self {
        Self::new(
            points
                .iter()
                .copied()
                .map(|point| ScaleResult::new(point, ResourceSnapshot::default()))
                .collect(),
        )
    }

    pub fn results(&self) -> &[ScaleResult] {
        &self.results
    }

    pub fn to_json(&self) -> String {
        let mut output = String::from("{\"results\":[");
        for (index, result) in self.results.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            output.push_str(&result.to_json());
        }
        output.push_str("]}");
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_failure_is_atomic_at_the_first_crossing() {
        let mut meter = ResourceMeter::new();
        meter.begin_episode();
        meter.begin_decision();
        meter
            .try_charge(LogicalResource::CandidateGenerationAttempts, 64)
            .unwrap();
        let before = meter.counters();
        let error = meter
            .try_charge(LogicalResource::CandidateGenerationAttempts, 1)
            .unwrap_err();

        assert_eq!(
            error,
            MeasurementError::LogicalLimit(ResourceLimitError {
                resource: LogicalResource::CandidateGenerationAttempts,
                scope: ResourceScope::Decision,
                used: 64,
                requested: 1,
                limit: 64,
            })
        );
        assert_eq!(meter.counters(), before);
    }

    #[test]
    fn missing_memory_is_not_zero() {
        let meter = ResourceMeter::new();
        assert_eq!(meter.peak_resident_memory(), MemorySample::Missing);
        assert_eq!(
            meter.validate_limits(),
            Err(MeasurementError::MissingMemoryMeasurement)
        );
    }
}
