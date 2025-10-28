# What is Nostr?

Nostr (Notes and Other Stuff Transmitted by Relays) is a simple, open protocol that enables global, decentralized, and censorship-resistant social media.

## Core Concepts

### 1. Identity

In Nostr, your identity is a cryptographic key pair:

- **Private Key (nsec)**: Your secret key that you never share. It proves you are who you say you are.
- **Public Key (npub)**: Your public identifier that you share with others. It's like your username, but cryptographically secure.

No email, no phone number, no centralized authority needed.

### 2. Relays

Relays are simple servers that store and forward messages (called "events"). Unlike traditional social media:

- Anyone can run a relay
- You can connect to multiple relays simultaneously
- Relays don't own your data
- If one relay goes down, you still have your messages on other relays

### 3. Events

Everything in Nostr is an "event" - a signed JSON message. Events include:

- Social media posts
- Direct messages
- Authentication requests
- Payment requests
- And more...

## Why Nostr for Portal?

Portal uses Nostr because it provides:

1. **Decentralized Identity**: Users control their own keys and identity
2. **No Central Server**: Communication happens through distributed relays
3. **Censorship Resistance**: No single point of control
4. **Privacy**: Direct peer-to-peer messaging
5. **Interoperability**: Standard protocol that works across applications

## Nostr in Portal's Authentication Flow

When a user authenticates with Portal:

1. Your application generates an authentication challenge
2. The challenge is published to Nostr relays
3. The user's wallet (like Alby, Mutiny, or others) picks up the challenge
4. The user approves or denies the authentication
5. The response is published back to Nostr
6. Your application receives the response

All of this happens peer-to-peer through Nostr relays, with no central authentication server.

## Learn More

- [Nostr Protocol Specification](https://github.com/nostr-protocol/nostr)
- [Nostr NIPs (Improvement Proposals)](https://github.com/nostr-protocol/nips)
- [Use Nostr](https://usenostr.org/)

---

**Next**: Learn about [Lightning Network](what-is-lightning.md)

