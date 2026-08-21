use std::collections::{BTreeMap, BTreeSet};

use thiserror::Error;

use crate::{AuthorityFingerprint, ConsequenceCeiling, WorkerRole};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LaneId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskSpec {
    pub id: TaskId,
    pub role: WorkerRole,
    pub dependencies: Vec<TaskId>,
    pub consequence: ConsequenceCeiling,
    pub budget_units: u64,
    pub authority: AuthorityFingerprint,
    pub forbidden_worker: Option<WorkerId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkerCandidate {
    pub id: WorkerId,
    pub roles: Vec<WorkerRole>,
    pub consequence_ceiling: ConsequenceCeiling,
    pub available: bool,
    pub historical_success_bps: u16,
    pub current_load: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SchedulerLimits {
    pub maximum_parallel: usize,
    pub total_budget_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub wave: usize,
    pub task: TaskId,
    pub worker: WorkerId,
    pub role: WorkerRole,
    pub consequence: ConsequenceCeiling,
    pub budget_units: u64,
    pub authority: AuthorityFingerprint,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulerDecision {
    pub task: TaskId,
    pub worker: WorkerId,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchedulePlan {
    pub assignments: Vec<Assignment>,
    pub decisions: Vec<SchedulerDecision>,
    pub allocated_budget_units: u64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SchedulerError {
    #[error("scheduler limits are invalid")]
    InvalidLimits,
    #[error("task or worker identifier is duplicated")]
    DuplicateIdentifier,
    #[error("task dependency is missing or self-referential")]
    InvalidDependency,
    #[error("task graph contains a cycle")]
    Cycle,
    #[error("mission task budget exceeds allocation")]
    BudgetExceeded,
    #[error("no independent compatible worker for task {0:?}")]
    NoCompatibleWorker(TaskId),
    #[error("worker metadata is invalid")]
    InvalidWorkerMetadata,
}

pub fn schedule(
    tasks: &[TaskSpec],
    workers: &[WorkerCandidate],
    limits: SchedulerLimits,
) -> Result<SchedulePlan, SchedulerError> {
    if limits.maximum_parallel == 0 {
        return Err(SchedulerError::InvalidLimits);
    }
    let mut task_map = BTreeMap::new();
    for task in tasks {
        if task_map.insert(task.id, task.clone()).is_some() {
            return Err(SchedulerError::DuplicateIdentifier);
        }
    }
    let mut worker_ids = BTreeSet::new();
    for worker in workers {
        if !worker_ids.insert(worker.id) {
            return Err(SchedulerError::DuplicateIdentifier);
        }
        if worker.historical_success_bps > 10_000 {
            return Err(SchedulerError::InvalidWorkerMetadata);
        }
    }
    for task in task_map.values() {
        let mut dependencies = BTreeSet::new();
        for dependency in &task.dependencies {
            if *dependency == task.id
                || !task_map.contains_key(dependency)
                || !dependencies.insert(*dependency)
            {
                return Err(SchedulerError::InvalidDependency);
            }
        }
    }
    let allocated_budget_units = task_map.values().try_fold(0u64, |total, task| {
        total
            .checked_add(task.budget_units)
            .ok_or(SchedulerError::BudgetExceeded)
    })?;
    if allocated_budget_units > limits.total_budget_units {
        return Err(SchedulerError::BudgetExceeded);
    }

    let mut remaining: BTreeSet<_> = task_map.keys().copied().collect();
    let mut completed = BTreeSet::new();
    let mut assignments = Vec::with_capacity(tasks.len());
    let mut decisions = Vec::with_capacity(tasks.len());
    let mut wave = 0usize;

    while !remaining.is_empty() {
        let ready: Vec<_> = remaining
            .iter()
            .copied()
            .filter(|task_id| {
                task_map[task_id]
                    .dependencies
                    .iter()
                    .all(|dependency| completed.contains(dependency))
            })
            .collect();
        if ready.is_empty() {
            return Err(SchedulerError::Cycle);
        }
        let mut used_workers = BTreeSet::new();
        let mut scheduled = Vec::new();
        for task_id in ready {
            if scheduled.len() == limits.maximum_parallel {
                break;
            }
            let task = &task_map[&task_id];
            let mut compatible: Vec<_> = workers
                .iter()
                .filter(|worker| {
                    worker.available
                        && worker.roles.contains(&task.role)
                        && worker.consequence_ceiling >= task.consequence
                        && task.forbidden_worker != Some(worker.id)
                })
                .collect();
            if compatible.is_empty() {
                return Err(SchedulerError::NoCompatibleWorker(task_id));
            }
            compatible.sort_by(|left, right| {
                right
                    .historical_success_bps
                    .cmp(&left.historical_success_bps)
                    .then_with(|| left.current_load.cmp(&right.current_load))
                    .then_with(|| left.id.cmp(&right.id))
            });
            let Some(worker) = compatible
                .into_iter()
                .find(|worker| !used_workers.contains(&worker.id))
            else {
                continue;
            };
            used_workers.insert(worker.id);
            scheduled.push(task_id);
            assignments.push(Assignment {
                wave,
                task: task_id,
                worker: worker.id,
                role: task.role,
                consequence: task.consequence,
                budget_units: task.budget_units,
                authority: task.authority,
            });
            decisions.push(SchedulerDecision {
                task: task_id,
                worker: worker.id,
                reason: format!(
                    "role={:?}; consequence={:?}; success_bps={}; load={}; deterministic_worker_id={}",
                    task.role,
                    task.consequence,
                    worker.historical_success_bps,
                    worker.current_load,
                    worker.id.0
                ),
            });
        }
        if scheduled.is_empty() {
            let task_id = remaining.first().copied().ok_or(SchedulerError::Cycle)?;
            return Err(SchedulerError::NoCompatibleWorker(task_id));
        }
        for task_id in scheduled {
            remaining.remove(&task_id);
            completed.insert(task_id);
        }
        wave = wave.checked_add(1).ok_or(SchedulerError::InvalidLimits)?;
    }

    Ok(SchedulePlan {
        assignments,
        decisions,
        allocated_budget_units,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchCandidate {
    pub lane: LaneId,
    pub base_revision: [u8; 32],
    pub changed_paths: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchConflict {
    pub lanes: (LaneId, LaneId),
    pub path: String,
}

pub fn detect_patch_conflicts(candidates: &[PatchCandidate]) -> Vec<PatchConflict> {
    let mut sorted = candidates.to_vec();
    sorted.sort_by_key(|candidate| candidate.lane);
    let mut conflicts = Vec::new();
    for left_index in 0..sorted.len() {
        for right_index in left_index + 1..sorted.len() {
            let left = &sorted[left_index];
            let right = &sorted[right_index];
            if left.base_revision != right.base_revision {
                conflicts.push(PatchConflict {
                    lanes: (left.lane, right.lane),
                    path: "<base-revision>".to_owned(),
                });
            }
            for path in left.changed_paths.intersection(&right.changed_paths) {
                conflicts.push(PatchConflict {
                    lanes: (left.lane, right.lane),
                    path: path.clone(),
                });
            }
        }
    }
    conflicts
}

pub fn evidence_is_independent(
    builder: WorkerId,
    verifier: WorkerId,
    consequence: ConsequenceCeiling,
) -> bool {
    consequence < ConsequenceCeiling::DecisionBoundary || builder != verifier
}
