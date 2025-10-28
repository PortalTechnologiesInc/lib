# Glossary

## Nostr Terms

**Nostr**: Notes and Other Stuff Transmitted by Relays. A decentralized protocol for social media and messaging.

**npub**: Nostr public key in bech32 format (starts with "npub1..."). This is a user's public identifier.

**nsec**: Nostr secret/private key in bech32 format (starts with "nsec1..."). Must be kept secret.

**Relay**: A server that stores and forwards Nostr events. Anyone can run a relay.

**Event**: A signed message in Nostr. Everything is an event (posts, messages, authentication, etc.).

**NIP**: Nostr Implementation Possibility. These are protocol specifications (like "NIPs" = RFCs for Nostr).

**NIP-05**: A verification method linking a Nostr key to a domain name (like email).

**Subkey**: A delegated key that can act on behalf of a main key with limited permissions.

## Lightning Network Terms

**Lightning Network**: A Layer 2 protocol built on Bitcoin for fast, cheap transactions.

**Satoshi (sat)**: The smallest unit of Bitcoin. 1 BTC = 100,000,000 sats.

**Millisat (msat)**: One thousandth of a satoshi. Lightning Network's smallest unit.

**Invoice**: A payment request in Lightning Network format (starts with "lnbc...").

**Preimage**: Proof of payment in Lightning Network. Hash of this is in the invoice.

**Channel**: A payment channel between two Lightning nodes allowing off-chain transactions.

**NWC**: Nostr Wallet Connect. A protocol for requesting payments via Nostr.

**Routing**: Finding a path through the Lightning Network to deliver a payment.

## Portal Terms

**Portal SDK Daemon**: The WebSocket server that handles Nostr and Lightning operations.

**Auth Token**: Secret token used to authenticate with the Portal daemon API.

**Key Handshake**: Initial exchange where user shares their public key and preferred relays.

**Challenge-Response**: Authentication method where you challenge a key and verify the signature.

**Single Payment**: One-time Lightning payment.

**Recurring Payment**: Subscription-based payment with automatic billing.

**Cashu**: An ecash protocol built on Lightning. Used for tickets/vouchers in Portal.

**Mint**: A Cashu mint that issues and redeems ecash tokens.

## Cashu Terms

**Cashu Token**: A bearer token representing sats, issued by a mint.

**Mint**: A server that issues and redeems Cashu tokens.

**Blind Signature**: Cryptographic technique allowing mints to sign tokens without knowing their value.

**Burn**: Redeeming a Cashu token back to sats at a mint.

## Technical Terms

**WebSocket**: A protocol for real-time bidirectional communication.

**Hex**: Hexadecimal format (base 16). Nostr keys are often shown in hex.

**Bech32**: An encoding format used for Bitcoin addresses and Nostr keys.

**JWT**: JSON Web Token. Used for session management and API authentication.

**Session Token**: A temporary token proving a user's authenticated session.

**Stream ID**: Identifier for a long-running operation (like payment status updates).

---

**Back to**: [Documentation Home](../README.md)

