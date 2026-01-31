# Portal API

This crate is the **Portal API** server (REST + WebSocket). Use it via the [TypeScript SDK](https://www.npmjs.com/package/portal-sdk) or [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk), or connect to the WebSocket API directly.

**Full documentation:** [https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/) — Quick Start, SDK usage, configuration, and [API reference](https://portaltechnologiesinc.github.io/lib/) (WebSocket commands).

---

## Run the API

**Docker (quick):**

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Use `ws://localhost:3000/ws` as the SDK `serverUrl` and your token in `client.authenticate(...)`. Check: `curl http://localhost:3000/health` → `OK`.

**From source:**

```bash
cargo build --release
./target/release/portal-rest
```

**Configuration:** Config file `~/.portal-rest/config.toml` (see `example.config.toml` in this crate) and env vars `PORTAL__<SECTION>__<KEY>`. Full options: [Environment Variables](https://portaltechnologiesinc.github.io/lib/getting-started/environment-variables.html) and [Docker / Building](https://portaltechnologiesinc.github.io/lib/getting-started/docker-deployment.html).

---

## API reference (advanced)

Without an SDK: connect to `GET /ws` (WebSocket), send an `Auth` command first, then JSON commands (`id`, `cmd`, `params`). Full command list and request/response shapes: see the [documentation](https://portaltechnologiesinc.github.io/lib/) and the TypeScript SDK types in `clients/ts`.
