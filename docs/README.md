# Welcome to Portal

Portal is a comprehensive toolkit for businesses to **authenticate**, **get paid**, and **issue tickets** to their customers without any intermediaries while maintaining full privacy.

## What is Portal?

Portal leverages the power of [Nostr](https://github.com/nostr-protocol/nostr) protocol and the [Lightning Network](https://lightning.network/) to provide:

- **Decentralized Authentication**: Secure user authentication without relying on centralized identity providers
- **Lightning Payments**: Process instant payments using Bitcoin's Lightning Network
- **Privacy-First**: No third parties, no data collection, full user privacy
- **No Intermediaries**: Direct peer-to-peer interactions between businesses and customers
- **Ticket Issuance**: Issue Cashu ecash tokens as tickets and vouchers for authorized users

## Key Features

### 🔐 Authentication
- Nostr-based user authentication using cryptographic keys
- Support for both main keys and delegated subkeys
- Secure challenge-response protocol
- No passwords, no email verification needed

### 💳 Payment Processing
- **Single Payments**: One-time Lightning Network payments
- **Recurring Payments**: Subscription-based payments with customizable schedules
- **Real-time Status**: Live payment status updates via WebSocket
- **Currency Support**: Millisats with exchange rate integration
- **Cashu Support**: Issue and accept Cashu ecash tokens

### 🎫 Ticket Issuance
- Issue Cashu tokens (ecash) as tickets to authenticated users
- Request and send Cashu tokens peer-to-peer
- Mint and burn tokens with your own mint
- Perfect for event tickets, access tokens, and vouchers

### 👤 Profile Management
- Fetch user profiles from Nostr
- Update and publish service profiles
- NIP-05 identity verification support

### 🔑 Session Management
- JWT token issuance and verification for API authentication
- Session tokens issued by user's wallet app
- Businesses verify JWT tokens without needing to issue them
- Perfect for stateless API authentication

### 🌐 Multi-Platform Support
- REST API with WebSocket support
- TypeScript/JavaScript SDK
- JVM/Kotlin client
- React Native bindings
- Docker deployment ready

## Use Cases

Portal is perfect for:

- **SaaS Applications**: Authenticate users and process subscriptions
- **Content Creators**: Monetize content with Lightning micropayments
- **Online Services**: Provide privacy-respecting authentication
- **Event Ticketing**: Issue and verify Cashu token-based tickets
- **Membership Sites**: Manage recurring memberships
- **API Services**: Verify JWT tokens issued by user wallets for API access

## How It Works

1. **Deploy the Portal SDK Daemon**: Run the REST API server using Docker
2. **Integrate the TypeScript SDK**: Connect your application to Portal
3. **Authenticate Users**: Generate authentication URLs for users to connect
4. **Process Payments**: Request single or recurring payments
5. **Issue Tickets**: Generate and send Cashu tokens as tickets to users

## Architecture

Portal consists of several components:

- **SDK Core**: Rust-based core implementation handling Nostr protocol and Lightning payments
- **REST API**: WebSocket-based API server for language-agnostic integration
- **TypeScript Client**: High-level SDK for JavaScript/TypeScript applications
- **Nostr Relays**: Distributed network for message passing
- **Lightning Network**: Bitcoin Layer 2 for instant payments

## Getting Started

Ready to integrate Portal into your application? Start with our [Quick Start Guide](getting-started/quick-start.md) or jump directly to:

- [Deploying with Docker](getting-started/docker-deployment.md)
- [TypeScript SDK Setup](sdk/installation.md)
- [Authentication Flow](guides/authentication.md)
- [Static Tokens & Physical Auth](guides/static-tokens.md)
- [Payment Processing](guides/single-payments.md)
- [Cashu Tokens](guides/cashu-tokens.md)
- [Running Your Own Mint](guides/running-a-mint.md)

## Open Source

Portal is open source and available under the MIT License (except for the app library). Contributions are welcome!

---

**Next Steps**: Head over to the [Quick Start Guide](getting-started/quick-start.md) to deploy your first Portal instance.

