# Relay Management

Manage Nostr relays used by your Portal instance. Relays store and forward Nostr messages.

## API

- **addRelay(relay):** Add a relay (e.g. `wss://relay.damus.io`). Returns confirmation.
- **removeRelay(relay):** Remove a relay.

```typescript
await client.addRelay('wss://relay.damus.io');
await client.removeRelay('wss://relay.damus.io');
```

Common relays: `wss://relay.damus.io`, `wss://relay.snort.social`, `wss://nos.lol`, `wss://relay.nostr.band`. Use several for redundancy; respect user preferred relays from the key handshake when relevant.

---

**Next:** [SDK](../sdk/installation.md)
