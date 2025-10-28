# Profile Management

Fetch and manage user profiles from the Nostr network.

## Fetching User Profiles

```typescript
const profile = await client.fetchProfile(userPubkey);

if (profile) {
  console.log('Name:', profile.name);
  console.log('Display Name:', profile.display_name);
  console.log('Picture:', profile.picture);
  console.log('About:', profile.about);
  console.log('NIP-05:', profile.nip05);
}
```

## Setting Your Service Profile

Publish your service's profile to Nostr:

```typescript
await client.setProfile({
  id: 'your-service-id',
  pubkey: 'your-service-pubkey',
  name: 'myservice',
  display_name: 'My Awesome Service',
  picture: 'https://myservice.com/logo.png',
  about: 'Premium service powered by Portal',
  nip05: 'verify@myservice.com'
});
```

## Profile Fields

- **id**: Unique identifier
- **pubkey**: Nostr public key (hex)
- **name**: Username (no spaces)
- **display_name**: Display name (can have spaces)
- **picture**: Profile picture URL
- **about**: Bio/description
- **nip05**: Nostr verified identifier (like email)

---

**Next**: [JWT Tokens](jwt-tokens.md)

