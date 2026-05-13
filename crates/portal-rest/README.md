# `portal-rest`

**HTTP surface for Portal:** Bearer-token auth, long-running jobs surfaced as pollable event streams, and optional webhooks (HMAC-signed). Ships as the `rest` binary.

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

| Endpoint shape | What to expect |
|----------------|----------------|
| `GET /health` | Liveness, no auth |
| `GET /version` | Build metadata, no auth |
| Async work | Response includes `stream_id`; poll `GET /events/:stream_id?after=<index>` |

From this crate’s directory after a release build:

```bash
cargo build --release
../../target/release/rest
```

Config: `~/.portal-rest/config.toml` plus `example.config.toml` in this folder. Environment keys use `PORTAL__<SECTION>__<KEY>`.

| Resource | Link |
|----------|------|
| Full docs | [portaltechnologiesinc.github.io/lib](https://portaltechnologiesinc.github.io/lib/) |
| Env reference | [Environment variables](https://portaltechnologiesinc.github.io/lib/advanced/environment-variables.html) |
| Docker | [Docker deployment](https://portaltechnologiesinc.github.io/lib/advanced/docker-deployment.html) |
| TypeScript | npm package `portal-sdk` |

Do not want to run a server? Use [PortalHub](https://hub.getportal.cc).
