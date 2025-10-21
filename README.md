# Portal

Portal is a Nostr-based authentication and payment SDK allowing applications to authenticate users and process payments through Nostr and Lightning Network.

## Project Overview

Portal provides a comprehensive solution for:

- Nostr-based user authentication
- Single and recurring payment processing
- Profile management and verification
- Cross-platform integration (Rust, TypeScript, and more)

## Repository Structure

- `/app` - API exposed to the app
- `/backend` - Example backend
- `/cli` - Command-line interface tool
- `/rates` - Bitcoin exchange rates from multiple sources
- `/react-native` - React Native bindings for the `app` crate
- `/rest` - SDK wrapper exposing a REST/websocket interface
- `/sdk` - Core SDK implementation
- `/rest/clients/ts` - TypeScript client for the REST API
- `/wallet` - Supported wallets(NWC, Breez)

## Getting Started

### Prerequisites

- Rust toolchain (latest stable)
- Node.js and npm (for TypeScript client)

### Building the Core SDK

```bash
cargo build --release
```

### Running the SDK Daemon with Docker

You can run the SDK Daemon using Docker. The image is published on Docker Hub as `getportal/sdk-daemon:latest`.

### Run with Docker (pre-built image)

```bash
docker run --rm --name portal-sdk-daemon -d \
  -p 3000:3000 \
  -e NOSTR_KEY=<your-nsec-here> \
  getportal/sdk-daemon:latest
```

- The daemon will be available on port 3000.
- The default auth token is 'remember-to-change-this'
- You can override environment variables as needed (see below).

### Environment Variables

- `AUTH_TOKEN`: Authentication token for API access
- `NOSTR_KEY`: Your Nostr private key in hex format
- `NWC_URL`: (Optional) Nostr Wallet Connect URL
- `NOSTR_SUBKEY_PROOF`: (Optional) Nostr subkey proof
- `NOSTR_RELAYS`: (Optional) Comma-separated list of relay URLs

## Using the TypeScript Client

### Installation

```bash
npm install portal-sdk
```

### Basic Usage

```typescript
import { PortalClient } from 'portal-sdk';

// Initialize client
const client = new PortalClient({
  serverUrl: 'ws://localhost:3000/ws'
});

// Authenticate
await client.connect('your-auth-token');

// Generate authentication URL
const { url, stream_id } = await client.getKeyHandshakeUrl();

// Request payment
const paymentResult = await client.requestSinglePayment({
  main_key: 'user-pubkey',
  subkeys: [],
  payment_request: {
    description: 'Product purchase',
    amount: 1000,
    currency: 'Millisats'
  }
});

// Close connection when done
client.disconnect();
```

## Features

### Authentication

Secure user authentication using Nostr protocol, supporting both main keys and delegated subkeys.

### Payment Processing

- **Single Payments**: One-time payments via Lightning Network
- **Recurring Payments**: Subscription-based payments with customizable recurrence patterns
- **Payment Status Tracking**: Real-time updates on payment status

### Profile Management

Fetch and verify user profiles through Nostr's social graph.

## API Documentation

See the `/rest/README.md` file for detailed API documentation.

## License

This project is licensed under the MIT License, except for the app library.