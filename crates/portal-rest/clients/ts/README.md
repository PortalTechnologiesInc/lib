# Portal TypeScript SDK

Official **TypeScript/JavaScript client** for the Portal API: authenticate users, process payments, manage profiles, issue JWTs, and more.

**Full documentation:** [https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/) — installation, basic usage, configuration, error handling, and all workflows (auth, single/recurring payments, profiles, JWT, Cashu, relays).

---

## Install

```bash
npm install portal-sdk
```

## Quick start

You need a **Portal endpoint URL** and an **auth token**. Use a hosted endpoint or run Portal locally (see [Quick Start](https://portaltechnologiesinc.github.io/lib/getting-started/quick-start.html)).

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: process.env.PORTAL_URL ?? 'ws://localhost:3000/ws',
});

await client.connect();
await client.authenticate(process.env.PORTAL_AUTH_TOKEN!);

const url = await client.newKeyHandshakeUrl((mainKey) => {
  console.log('User authenticated:', mainKey);
});
console.log('Share this URL with your user:', url);
```

For workflows (auth, payments, profiles, JWT), configuration, and error handling, see the [TypeScript SDK docs](https://portaltechnologiesinc.github.io/lib/sdk/installation.html) and [Guides](https://portaltechnologiesinc.github.io/lib/guides/authentication.html).

---

## License

MIT — see [LICENSE](../../LICENSE) in the repo.
