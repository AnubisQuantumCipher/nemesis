use std::collections::BTreeSet;

use nemesis_runtime::{
    AuthorityFingerprint, ConsequenceCeiling, LaneId, PatchCandidate, SchedulerLimits, TaskId,
    TaskSpec, WorkerCandidate, WorkerId, WorkerRole, detect_patch_conflicts,
    evidence_is_independent, schedule,
};

fn worker(id: u32, roles: Vec<WorkerRole>, success: u16, load: u16) -> WorkerCandidate {
    WorkerCandidate {
        id: WorkerId(id),
        roles,
        consequence_ceiling: ConsequenceCeiling::DecisionBoundary,
        available: true,
        historical_success_bps: success,
        current_load: load,
    }
}

#[test]
fn schedule_is_deterministic_across_input_order() {
    let tasks = vec![
        TaskSpec {
            id: TaskId(2),
            role: WorkerRole::Reviewer,
            dependencies: vec![TaskId(1)],
            consequence: ConsequenceCeiling::DecisionBoundary,
            budget_units: 20,
            authority: AuthorityFingerprint([2; 32]),
            forbidden_worker: Some(WorkerId(1)),
        },
        TaskSpec {
            id: TaskId(1),
            role: WorkerRole::Builder,
            dependencies: vec![],
            consequence: ConsequenceCeiling::Advisory,
            budget_units: 30,
            authority: AuthorityFingerprint([1; 32]),
            forbidden_worker: None,
        },
    ];
    let workers = vec![
        worker(2, vec![WorkerRole::Reviewer], 9_200, 0),
        worker(1, vec![WorkerRole::Builder, WorkerRole::Reviewer], 9_500, 0),
    ];
    let limits = SchedulerLimits {
        maximum_parallel: 2,
        total_budget_units: 50,
    };
    let first = schedule(&tasks, &workers, limits).unwrap();
    let mut reversed_tasks = tasks.clone();
    reversed_tasks.reverse();
    let mut reversed_workers = workers.clone();
    reversed_workers.reverse();
    let second = schedule(&reversed_tasks, &reversed_workers, limits).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.assignments[0].wave, 0);
    assert_eq!(first.assignments[1].wave, 1);
    assert_eq!(first.assignments[1].worker, WorkerId(2));
}

#[test]
fn compatible_tasks_serialize_when_only_one_worker_is_available() {
    let tasks = [
        TaskSpec {
            id: TaskId(1),
            role: WorkerRole::Builder,
            dependencies: vec![],
            consequence: ConsequenceCeiling::Advisory,
            budget_units: 1,
            authority: AuthorityFingerprint([1; 32]),
            forbidden_worker: None,
        },
        TaskSpec {
            id: TaskId(2),
            role: WorkerRole::Builder,
            dependencies: vec![],
            consequence: ConsequenceCeiling::Advisory,
            budget_units: 1,
            authority: AuthorityFingerprint([2; 32]),
            forbidden_worker: None,
        },
    ];
    let plan = schedule(
        &tasks,
        &[worker(1, vec![WorkerRole::Builder], 8_000, 0)],
        SchedulerLimits {
            maximum_parallel: 2,
            total_budget_units: 2,
        },
    )
    .unwrap();
    assert_eq!(plan.assignments[0].wave, 0);
    assert_eq!(plan.assignments[1].wave, 1);
}

#[test]
fn scheduler_records_explainable_decisions_and_preserves_authority() {
    let authority = AuthorityFingerprint([0x5a; 32]);
    let plan = schedule(
        &[TaskSpec {
            id: TaskId(7),
            role: WorkerRole::Verifier,
            dependencies: vec![],
            consequence: ConsequenceCeiling::DecisionBoundary,
            budget_units: 10,
            authority,
            forbidden_worker: None,
        }],
        &[worker(4, vec![WorkerRole::Verifier], 9_000, 1)],
        SchedulerLimits {
            maximum_parallel: 1,
            total_budget_units: 10,
        },
    )
    .unwrap();
    assert_eq!(plan.assignments[0].authority, authority);
    assert!(plan.decisions[0].reason.contains("role=Verifier"));
    assert!(plan.decisions[0].reason.contains("success_bps=9000"));
}

#[test]
fn scheduler_refuses_missing_worker_budget_and_cycles() {
    let task = TaskSpec {
        id: TaskId(1),
        role: WorkerRole::Builder,
        dependencies: vec![],
        consequence: ConsequenceCeiling::Advisory,
        budget_units: 11,
        authority: AuthorityFingerprint([1; 32]),
        forbidden_worker: None,
    };
    assert!(
        schedule(
            &[task.clone()],
            &[],
            SchedulerLimits {
                maximum_parallel: 1,
                total_budget_units: 11,
            },
        )
        .is_err()
    );
    assert!(
        schedule(
            &[task.clone()],
            &[worker(1, vec![WorkerRole::Builder], 8_000, 0)],
            SchedulerLimits {
                maximum_parallel: 1,
                total_budget_units: 10,
            },
        )
        .is_err()
    );
    let cyclic = [
        TaskSpec {
            dependencies: vec![TaskId(2)],
            ..task.clone()
        },
        TaskSpec {
            id: TaskId(2),
            dependencies: vec![TaskId(1)],
            ..task
        },
    ];
    assert!(
        schedule(
            &cyclic,
            &[worker(1, vec![WorkerRole::Builder], 8_000, 0)],
            SchedulerLimits {
                maximum_parallel: 2,
                total_budget_units: 22,
            },
        )
        .is_err()
    );
}

#[test]
fn conflicting_lane_patches_are_surfaced_not_merged() {
    let left = PatchCandidate {
        lane: LaneId(1),
        base_revision: [1; 32],
        changed_paths: BTreeSet::from(["src/core.rs".to_owned(), "README.md".to_owned()]),
    };
    let right = PatchCandidate {
        lane: LaneId(2),
        base_revision: [1; 32],
        changed_paths: BTreeSet::from(["src/core.rs".to_owned()]),
    };
    let conflicts = detect_patch_conflicts(&[left, right]);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].path, "src/core.rs");
    assert_eq!(conflicts[0].lanes, (LaneId(1), LaneId(2)));
}

#[test]
fn builder_cannot_accept_own_elevated_evidence() {
    assert!(!evidence_is_independent(
        WorkerId(9),
        WorkerId(9),
        ConsequenceCeiling::DecisionBoundary
    ));
    assert!(evidence_is_independent(
        WorkerId(9),
        WorkerId(10),
        ConsequenceCeiling::DecisionBoundary
    ));
}
