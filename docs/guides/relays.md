# Relay Management

Manage Nostr relays used by your Portal instance. Relays store and forward Nostr messages.

## API

- **addRelay(relay):** Add a relay (e.g. wss://relay.damus.io). Returns confirmation.
- **removeRelay(relay):** Remove a relay.

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
await client.addRelay('wss://relay.damus.io');
await client.removeRelay('wss://relay.damus.io');
```

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.AddRelayRequest;
import cc.getportal.command.request.RemoveRelayRequest;

sdk.sendCommand(new AddRelayRequest("wss://relay.damus.io"), (res, err) -> {
    if (err != null) System.err.println(err);
});
sdk.sendCommand(new RemoveRelayRequest("wss://relay.damus.io"), (res, err) -> {
    if (err != null) System.err.println(err);
});
```

</section>

</custom-tabs>

Common relays: `wss://relay.damus.io`, `wss://relay.snort.social`, `wss://nos.lol`, `wss://relay.nostr.band`. Use several for redundancy; respect user preferred relays from the key handshake when relevant.

---

**Next:** [SDK](../sdk/installation.md)
