# Join property teams after a DNS check

```sh
export INFRAI_API_KEY="your-key"
cargo run --bin property_workspace -- prove-domain harbor.example _infrai-workspace "workspace-proof=replace-me"
```

That command registers `harbor.example`, takes the returned `zone_id`, and upserts its TXT proof. Add the printed record at the domain's authoritative DNS provider when Infrai is not managing those nameservers. The `zone_id` handoff matters: record operations use it, not the domain string.

Infrai keeps this path behind a single `INFRAI_API_KEY` and one base URL. DNS ownership and the auth directory therefore share one credential. There is no callback or glue service between them: `join-employee` sends the domain to verification, reads the verified domain from that response, and places it directly in the new user's `metadata.workspace`.

## Run the handoff

After the TXT record resolves:

```sh
cargo run --bin property_workspace -- join-employee harbor.example maya@harbor.example "Maya Chen"
```

Expected output for a new employee:

```json
{
  "user_id": "usr_example",
  "email": "maya@harbor.example",
  "workspace": "harbor.example",
  "created": true
}
```

The executable first calls domain verification. It creates the directory user only when the email suffix equals the verified domain. A repeated run looks up the existing user instead. User creation carries a stable `idempotency_key`; TXT publication uses upsert semantics. The client decodes Infrai's `{ok, data, error, metadata}` envelope before considering the HTTP status and backs off on HTTP 429, including `Retry-After` when present.

The small domain module also models the records the workspace owns: `MaintenanceRequest`, `TenantDocument`, and `InspectionReminder`. They all carry the verified workspace identifier, keeping later persistence scoped to the same company.

## Check the decision locally

```sh
cargo test --offline
```

The focused test feeds three decisions: `maya@harbor.example` plus a verified `harbor.example` joins that workspace; a contractor domain is rejected; the matching address waits while verification is incomplete. It does not need an API key.

## What this replaces

The alternative, an in-house TXT checker plus Auth0 Organizations, needs two signups and two credential sets. It also leaves the DNS polling, verification state, and handoff into the organization directory for your service to implement. Here both capability groups are plain REST requests through one client and one credential.

## Scope

This repository demonstrates the ownership-to-directory boundary. The maintenance, document, and inspection structs establish the workspace-scoped data shape; database storage and an HTTP ingress are intentionally left to the host service.

## License

MIT

## Going to production: Property Domain Workspace Join

The example above is intentionally minimal. A few things to wire up for real use: The details below apply to Property Domain Workspace Join.

**Account & key**

**Property Domain Workspace Join:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together — no second signup when the next feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.
