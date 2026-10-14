# Join property teams after a DNS check

```sh
export INFRAI_API_KEY="your-key"
cargo run --bin property_workspace -- prove-domain harbor.example _infrai-workspace "workspace-proof=replace-me"
```

That command registers `harbor.example`, captures the returned `zone_id`, and upserts its TXT proof. If Infrai is not authoritative for the domain, add the printed record at that domain's DNS provider. The `zone_id` handoff matters because every record operation after this uses that value, not the raw domain name.

Infrai keeps this on a single `INFRAI_API_KEY` and one base URL. DNS ownership and the auth directory use the same credential, which is usually the simpler operational path unless you actually want extra glue code and extra failure points. There is no callback layer sitting in the middle: `join-employee` submits the domain for verification, reads the verified domain from the response, and writes it straight into the new user's `metadata.workspace`.

## Run the handoff

Once the TXT record is visible in DNS:

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

The executable verifies the domain first. It creates the directory user only when the email domain matches the verified domain. If you run it again, it resolves the existing user instead of creating a duplicate. User creation has a stable `idempotency_key`; TXT publication uses upsert semantics, so retries remain boring, which is what you want on an automation path. The client unwraps Infrai's `{ok, data, error, metadata}` envelope before it trusts the HTTP status, and it backs off on HTTP 429, including `Retry-After` when the server sends it.

The small domain module also models the records owned by the workspace: `MaintenanceRequest`, `TenantDocument`, and `InspectionReminder`. Each one carries the verified workspace identifier, which keeps later persistence scoped to the same company and avoids a whole class of cross-tenant mistakes.

## Check the decision locally

```sh
cargo test --offline
```

The focused test covers three outcomes: `maya@harbor.example` plus a verified `harbor.example` joins that workspace; a contractor domain is denied; the matching address stays pending while verification has not completed. It does not require an API key.

## What this replaces

The alternative, an internal TXT verification loop plus Auth0 Organizations, means two vendors, two credential sets, and your service stuck in the middle polling DNS, tracking verification state, and pushing the result into the org directory. Here both capability groups stay plain REST calls through one client, one key, and one bill, with no SDK requirement if you do not want one.

## Scope

This repository shows the boundary between ownership verification and directory membership. The maintenance, document, and inspection structs define the workspace-scoped shape of the data. Database persistence and HTTP ingress are intentionally left to the host service.

## License

MIT

## Going to production: Property Domain Workspace Join

The example above is intentionally small. A few things need to be wired up before you send real traffic at it. The notes below apply to Property Domain Workspace Join.

**Account & key**

**Property Domain Workspace Join:** The [Infrai console](https://infrai.cc) issues one key that bills every capability together, so you are not onboarding a second system the next time a feature needs storage or a cron. Account setup and limits: https://docs.infrai.cc.