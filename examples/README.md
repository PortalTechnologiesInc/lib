# Portal Examples

Runnable JavaScript examples for the Portal REST API.

## Setup

```bash
cd examples
npm install
```

Copy `.env.example` to `.env` and fill in your values:

```
PORTAL_URL=http://localhost:3000
PORTAL_TOKEN=your-secret-token
MAIN_KEY=replace-with-user-pubkey-hex
```

## Run

```bash
node auth.js
node single-payment.js
node profile.js
```

## Requirements

- Node.js 18+
- A running Portal daemon (`getportal/sdk-daemon`) — see [Docker Deployment](../docs/getting-started/docker-deployment.md)
