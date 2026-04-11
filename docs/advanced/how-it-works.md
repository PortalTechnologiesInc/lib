# How Portal Works (Nostr & Lightning)

Portal is built on two open protocols: **Nostr** for identity and messaging, and the **Lightning Network** for payments. This page explains the technology under the hood — you don't need to understand any of this to use Portal, but it's here if you're curious.

## Nostr — Decentralized Identity

[Nostr](https://github.com/nostr-protocol/nostr) (Notes and Other Stuff Transmitted by Relays) is a simple, open protocol for decentralized communication.

### Identity is a key pair

In Nostr, your identity is a cryptographic key pair:

- **Private Key (nsec)**: Your secret key that you never share. It proves you are who you say you are.
- **Public Key (npub)**: Your public identifier — like a username, but cryptographically secure.

No email, no phone number, no centralized authority needed. Portal uses Nostr keys for passwordless authentication.

### Relays

Relays are simple servers that store and forward messages (called "events"). Unlike traditional services:

- Anyone can run a relay
- You can connect to multiple relays simultaneously
- Relays don't own your data
- If one relay goes down, your messages exist on other relays

Portal uses relays to communicate between your application and the user's wallet.

### Events

Everything in Nostr is a signed JSON message called an "event" — social media posts, direct messages, authentication requests, payment requests, and more.

### How Portal uses Nostr

When a user authenticates with Portal:

1. Your application generates an authentication challenge
2. The challenge is published to Nostr relays
3. The user's wallet picks up the challenge
4. The user approves or denies
5. The response is published back via relays
6. Your application receives the response

All peer-to-peer through relays, with no central authentication server.

## Lightning Network — Instant Payments

The [Lightning Network](https://lightning.network/) is a Layer 2 payment protocol built on Bitcoin that enables fast, low-cost transactions.

### Why Lightning?

| | Bitcoin (on-chain) | Lightning |
|--|-------------------|-----------|
| Speed | 10+ minutes | Sub-second |
| Fees | Variable, can be high | Minimal (< 1 sat) |
| Privacy | Public blockchain | Off-chain |
| Micropayments | Impractical | Native support |

### How Portal uses Lightning

Portal uses **Nostr Wallet Connect (NWC)**, a protocol that allows:

- Requesting payments through Nostr messages
- User approval through their Lightning wallet
- Real-time payment status updates
- Non-custodial flow (users keep control of their funds)

The payment flow:

1. **Payment request**: Your app requests a payment through Portal
2. **Nostr message**: Request is sent to the user via Nostr
3. **Wallet notification**: User's wallet shows the payment request
4. **User approval**: User approves or denies
5. **Lightning payment**: Wallet sends payment via Lightning
6. **Confirmation**: Your app receives real-time confirmation

Compatible wallets include [Alby](https://getalby.com/), [Mutiny](https://www.mutinywallet.com/), [Breez](https://breez.technology/), and any NWC-compatible wallet.

## Cashu — Digital Tickets & Vouchers

[Cashu](https://cashu.space/) is an ecash protocol. Portal uses Cashu tokens as **digital tickets and vouchers** — for example, age verification proofs are Cashu tokens.

Key properties:
- **Bearer tokens**: Whoever holds the token can use it
- **Privacy**: Blind signatures mean the mint can't track who uses what
- **Programmable**: Custom units, metadata, expiration

In Portal, Cashu tokens are used for:
- Age verification proofs
- Event tickets
- Access vouchers
- Any transferable digital credential

## Learn more

- [Nostr Protocol](https://github.com/nostr-protocol/nostr) · [NIPs](https://github.com/nostr-protocol/nips)
- [Lightning Network](https://lightning.network/)
- [Nostr Wallet Connect](https://nwc.getalby.com/)
- [Cashu Protocol](https://cashu.space/)

---

**Back to:** [Documentation Home](../README.md)
