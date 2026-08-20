# NEMESIS Deterministic Scheduler v1

The scheduler converts a validated task DAG and worker metadata into an explainable assignment plan. It does not commit authoritative mission state.

Deterministic ordering uses task identifier, then compatible worker ranking by historical success basis points descending, current load ascending, and worker identifier ascending. Inputs are normalized into ordered maps/sets before scheduling, so caller input order does not change the plan.

Controls:

- unique task and worker identifiers;
- existing, non-self, nonduplicate dependencies and cycle rejection;
- checked total budget allocation;
- maximum parallel assignments per wave;
- one assignment per worker per wave, with compatible tasks serialized when capacity is lower;
- role and consequence-ceiling enforcement;
- explicit forbidden worker for builder/reviewer separation;
- original authority fingerprint copied unchanged to each assignment;
- decision record naming role, consequence, historical score, load, and deterministic worker choice;
- same-path and base-revision patch conflicts surfaced as records, never silently merged;
- builder self-evidence rejected for decision-boundary and safety-critical claims.

Actual state mutations still pass through Core's serialized event commit and SPARK Kernel rules. A scheduler plan is a proposal, not authority.

```sh
./scripts/test_scheduler.sh
```
