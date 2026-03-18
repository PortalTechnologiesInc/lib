# Portal Examples

Runnable JavaScript examples for the Portal REST API.

## Setup

```bash
cd examples
npm install
```

Set your Portal endpoint and auth token:

```bash
export PORTAL_URL=http://localhost:3000
export PORTAL_TOKEN=your-secret-token
```

Or edit the `BASE_URL` / `AUTH_TOKEN` constants at the top of each file.

## Run

```bash
node auth.js
node single-payment.js
node profile.js
```

## Requirements

- Node.js 18+
- A running Portal daemon (`getportal/sdk-daemon`) — see [Docker Deployment](../docs/getting-started/docker-deployment.md)
