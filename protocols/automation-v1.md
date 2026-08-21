# NEMESIS Local Automation v1

Automation is a bounded local queue of mission-template requests triggered by a local schedule, local file event, or manual action. Public webhooks, messaging delivery, push notification, and remote approval are not implemented.

The unattended policy has an explicit operation allowlist. Network, secrets, approvals, push, publish, and external delivery are permanently denied in this desktop mission even if mistakenly included in that allowlist. A request that requires approval becomes terminal `Denied`; it does not wait forever and does not proceed silently.

Queue controls:

- unique request and trigger keys for idempotency;
- fixed capacity;
- due-time ordering by committed queue order;
- per-minute dispatch limit;
- typed queued/running/completed/failed/denied states;
- only running entries may complete;
- denied authority boundaries do not consume dispatch rate;
- canonical JSON checkpoint bound by SHA-256 and Ed25519;
- exact recovery checks capacity, ordering, trigger-key membership, digest, public key, and signature;
- one-byte checkpoint mutation rejects.

```sh
./scripts/test_automation.sh
```

This gate establishes local automation policy and durable queue behavior. It sends no external communication and changes no Login Item or system scheduler.
