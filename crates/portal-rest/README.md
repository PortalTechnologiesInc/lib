# Portal REST API

REST API server for Portal — authentication, payments, age verification, and more. Use it via the [TypeScript SDK](https://www.npmjs.com/package/portal-sdk) or call the HTTP endpoints directly.

> 📖 **[Full documentation](https://portaltechnologiesinc.github.io/lib/)** · 🚀 **[Get started with PortalHub](https://hub.getportal.cc)** (no self-hosting needed)

---

## Architecture

- **REST API** with Bearer token authentication
- **Async operations** return a `stream_id`; poll via `GET /events/:stream_id?after=<index>`
- **Webhook delivery** for real-time push notifications (HMAC-SHA256 signed)
- **TypeScript SDK** available on npm (`portal-sdk`)
- **Public endpoints** (no auth): `GET /health`, `GET /version`

## Run the API

**Docker (quick):**

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Check: `curl http://localhost:3000/health` → `OK`, `curl http://localhost:3000/version` → `{"data":{"version":"...","git_commit":"..."}}`

**From source:**

```bash
cargo build --release
./target/release/rest
```

**Configuration:** Config file `~/.portal-rest/config.toml` (see `example.config.toml`) and environment variables `PORTAL__<SECTION>__<KEY>`. Full options: [Environment Variables](https://portaltechnologiesinc.github.io/lib/advanced/environment-variables.html) and [Docker Deployment](https://portaltechnologiesinc.github.io/lib/advanced/docker-deployment.html).

> **Don't want to self-host?** Use [PortalHub](https://hub.getportal.cc) — create a hosted Portal instance in seconds.
