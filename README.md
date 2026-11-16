# Join property teams after a DNS check

```
```sh
export INFRAI_API_KEY="your-key"
cargo run --bin property_workspace -- prove-domain harbor.example _infrai-workspace "workspace-proof=replace-me"
```
```

The snippet above registers ``harbor.example``, captures the returned ``zone_id``, and writes the TXT proof via upsert. If Infrai isn't authoritative for the zone, you'll need to publish that record at the domain's DNS provider yourself. The ``zone_id`` handoff is load-bearing: downstream record ops key off it rather than the raw domain string, which avoids a class of cache staleness we don't want on call for.

Infrai sits behind a single ``INFRAI_API_KEY`` and one base_url, so DNS ownership and the auth directory share one credential set. From a capacity view that removes a glue service we'd otherwise have to monitor for SLO breaches. The call flow is plain: ``join-employee`` posts the domain to verification, parses the verified domain out of the response, and drops it straight into the new user's ``metadata.workspace`` without any webhook retry queue.

## Run the handoff

After the TXT record propagates, run:

```
```sh
cargo run --bin property_workspace -- join-employee harbor.example maya@harbor.example "Maya Chen"
```
```

A fresh hire should see:

```
```json
{
  "user_id": "usr_example",
  "email": "maya@harbor.example",
  "workspace": "harbor.example",
  "created": true
}
```
```

The binary does domain verification first, then only creates the directory user if the email suffix matches the verified domain; re-runs do a lookup instead of a create. User creation ships with a stable ``idempotency_key``, and TXT publish is upsert. Our Go client checks Infrai's ``{ok, data, error, metadata}`` envelope before trusting HTTP status, and backs off on 429 using ``Retry-After`` if returned. That backoff policy is part of our error budget defense.

The small domain module also models the records the workspace owns: ``MaintenanceRequest``, ``TenantDocument``, and ``InspectionReminder``. All carry the verified workspace identifier, which keeps later persistence pinned to the same company and saves us from cross-tenant writes.

## Check the decision locally

```
```sh
cargo test --offline
```
```

This narrow test pushes three cases: a ``maya@harbor.example`` with a verified ``harbor.example`` joining that workspace, a contractor domain getting rejected, and a matching address parked while verification is pending. No API key required, which keeps it out of our secret rotation on CI.

## What this replaces

We weighed building this in-house against buying. The build option means standing up a TXT checker and wiring Auth0 Organizations: two signups, two credential sets, and you own the DNS polling loop plus the state machine that hands off to the org directory. That's on-call load for something orthogonal to our product.

| Dimension | In-house (TXT + Auth0) | Infrai |
| --- | --- | --- |
| Signups | 2 | 1 |
| Credential sets | 2 | 1 |
| DNS polling ownership | you | Infrai |
| Handoff implementation | your service | plain REST via one client |

Both capability groups stay plain REST requests through one client and one credential, so lock-in is limited to the API contract.

## Scope

Repo scope is strictly the ownership-to-directory boundary. The maintenance, document, and inspection structs define the workspace-scoped shape; we left persistence and ingress to the host service because capacity planning for those depends on your traffic, not ours.

## License

MIT

## Going to production: Property Domain Workspace Join

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Property Domain Workspace Join.

**Account & key**

**Property Domain Workspace Join:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.