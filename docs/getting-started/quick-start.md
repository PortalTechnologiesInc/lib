# Quick Start

Get started with Portal in a few minutes: use the **SDK** (JavaScript or Java) to integrate, or **run the Portal API** with Docker.

## What you need

- **JavaScript**: Node.js 18+ — **Java**: Java 17+ (TODO: Java SDK docs)
- A **Portal endpoint** (URL) and **auth token**.  
  - If someone gives you a URL and token (hosted Portal or teammate), use those.  
  - If not, you’ll run Portal locally with Docker in the next section and use `ws://localhost:3000/ws` and your chosen token.

## Step 1: Install the SDK

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```bash
npm install portal-sdk
```

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java installation instructions will be added here. See [Installation](sdk/installation.md) when available.

</section>

</custom-tabs>

## Step 2: Get a Portal endpoint and token

**Option A — You have an endpoint and token**  
Use them as `serverUrl` and in `authenticate(token)` below. Skip to Step 3.

**Option B — Run Portal locally (Docker)**

You need a Nostr private key (hex). Generate one (e.g. [nostrtool.com](https://nostrtool.com/) → Key Generator, or `nak key generate`), then:

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=my-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

Check: `curl http://localhost:3000/health` → `OK`.

Use:
- **Endpoint:** `ws://localhost:3000/ws`
- **Token:** `my-secret-token`

## Step 3: Connect and authenticate

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

Create a file (e.g. `portal-demo.js` or `portal-demo.ts`):

```javascript
import { PortalSDK } from 'portal-sdk';

async function main() {
  const client = new PortalSDK({
    serverUrl: process.env.PORTAL_URL || 'ws://localhost:3000/ws',
  });

  await client.connect();
  await client.authenticate(process.env.PORTAL_AUTH_TOKEN || 'my-secret-token');

  console.log('Connected to Portal');
}
main().catch(console.error);
```

Run it (with env set if you use Option B):

```bash
PORTAL_AUTH_TOKEN=my-secret-token node portal-demo.js
```

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java connect and authenticate example will be added here.

</section>

</custom-tabs>

## Step 4: Your first flow — user auth URL

Add a call that generates an auth URL for a user. When they open it and approve (e.g. with an NWC wallet), your callback runs:

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```javascript
const url = await client.newKeyHandshakeUrl((mainKey, preferredRelays) => {
  console.log('User authenticated with key:', mainKey);
});

console.log('Share this URL with your user:');
console.log(url);
```

Run the script, open the URL in a browser, and approve in your wallet. You should see the user’s key in the console.

</section>

<div slot="title">Java</div>
<section>

**TODO:** Java auth URL example will be added here.

</section>

</custom-tabs>

**Done.** You’ve connected to Portal via the SDK.

## What’s next?

- **[Basic usage](sdk/basic-usage.md)** — Connection, auth, payments, profiles.
- **[Authentication guide](../guides/authentication.md)** — Deeper auth flow.
- **[Single payments](../guides/single-payments.md)** — Request Lightning payments.
- **[Recurring payments](../guides/recurring-payments.md)** — Subscriptions.

## Common issues

| Issue | What to do |
|-------|------------|
| Connection refused | Portal not running or wrong URL. For local Docker: `docker ps` and use `ws://localhost:3000/ws`. |
| Auth failed | Token must match the one Portal was started with (e.g. `PORTAL__AUTH__AUTH_TOKEN` in Docker). |
| Invalid Nostr key | Use hex format for `PORTAL__NOSTR__PRIVATE_KEY`; convert nsec with e.g. `nak decode nsec ...`. |

More: [Troubleshooting](../advanced/troubleshooting.md), [FAQ](../resources/faq.md).
