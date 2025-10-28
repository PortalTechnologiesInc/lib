# Relay Management

Dynamically manage Nostr relays in your Portal instance.

## Overview

Relays are Nostr servers that store and forward messages. Portal connects to multiple relays for redundancy and better message delivery.

## Adding Relays

```typescript
const relayUrl = 'wss://relay.damus.io';

const addedRelay = await client.addRelay(relayUrl);
console.log('Added relay:', addedRelay);
```

## Removing Relays

```typescript
const relayUrl = 'wss://relay.damus.io';

const removedRelay = await client.removeRelay(relayUrl);
console.log('Removed relay:', removedRelay);
```

## Popular Relays

Here are some reliable public relays:

- `wss://relay.damus.io` - Popular, well-maintained
- `wss://relay.snort.social` - Fast and reliable
- `wss://nos.lol` - Good for payments
- `wss://relay.nostr.band` - Large relay network
- `wss://nostr.wine` - Paid relay (more reliable)

## Best Practices

1. **Use 3-5 relays**: Balance between redundancy and bandwidth
2. **Geographic diversity**: Choose relays in different locations
3. **Mix free and paid**: Paid relays often have better uptime
4. **Monitor connectivity**: Remove relays that are consistently offline
5. **User preferences**: Respect user's preferred relays from handshake

## Relay Configuration Example

```typescript
class RelayManager {
  private client: PortalSDK;
  private activeRelays = new Set<string>();

  async setupDefaultRelays() {
    const defaultRelays = [
      'wss://relay.damus.io',
      'wss://relay.snort.social',
      'wss://nos.lol'
    ];

    for (const relay of defaultRelays) {
      try {
        await this.client.addRelay(relay);
        this.activeRelays.add(relay);
        console.log('✅ Connected to', relay);
      } catch (error) {
        console.error('❌ Failed to connect to', relay);
      }
    }
  }

  async addUserRelays(preferredRelays: string[]) {
    // Add user's preferred relays from handshake
    for (const relay of preferredRelays) {
      if (!this.activeRelays.has(relay)) {
        try {
          await this.client.addRelay(relay);
          this.activeRelays.add(relay);
        } catch (error) {
          console.error('Failed to add user relay:', relay);
        }
      }
    }
  }
}
```

---

**Next**: [API Reference](../api/typescript-sdk.md)

