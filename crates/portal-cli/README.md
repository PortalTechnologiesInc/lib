# `portal-cli`

**Developer utilities** built on `app`, `portal`, and `portal-sdk`—handy for reproducing flows and inspecting protocol behavior without the full REST stack.

Build every binary from the workspace root:

```bash
cargo build -p portal-cli
```

Artifacts use the **file stem** under `src/bin/` (there is no single binary named `portal-cli`):

| Binary | Rough purpose |
|--------|----------------|
| `main` | Sample / exploratory flow (`main.rs`; often needs `CLI_NWC_URL` and similar env) |
| `single_payment_request` | Single payment request scenario |
| `invoices` | Invoice-related exercises |
| `cashu`, `jwt`, `reconnect`, `macros` | Smaller focused demos and checks |

These are **not** end-user tools. Prefer `portal-rest` + [PortalHub](https://hub.getportal.cc) for production-shaped deployments.
