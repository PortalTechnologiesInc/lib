# SDK Configuration

Constructor options for PortalSDK:

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Option | Required | Description |
|--------|----------|-------------|
| `serverUrl` | Yes | WebSocket URL (e.g. `ws://localhost:3000/ws` or `wss://...`) |
| `connectTimeout` | No | Connection timeout in ms (default 10000) |
| `debug` | No | When `true`, log requests/responses to console (default `false`) |

</section>

<div slot="title">Java</div>
<section>

Pass WebSocket URL to new PortalSDK(wsUrl); pass auth token to authenticate(authToken) after connect().

</section>

</custom-tabs>

---

**Next:** [Error Handling](error-handling.md)
