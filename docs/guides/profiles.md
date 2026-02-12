# Profile Management

Fetch and manage user profiles from the Nostr network.

## Fetching User Profiles

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.FetchProfileRequest;
import cc.getportal.command.response.FetchProfileResponse;

sdk.sendCommand(
    new FetchProfileRequest("user-pubkey-hex"),
    (res, err) -> {
        if (err != null) return;
        System.out.println("profile: " + res.profile());
    }
);
```

</section>

</custom-tabs>

## Setting Your Service Profile

Publish your service's profile to Nostr:

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

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

</section>

<div slot="title">Java</div>
<section>

```java
import cc.getportal.command.request.SetProfileRequest;
import cc.getportal.model.Profile;

sdk.sendCommand(
    new SetProfileRequest(new Profile("alice", "Alice", "https://...", "alice@example.com")),
    (res, err) -> {
        if (err != null) { System.err.println(err); return; }
    }
);
```

</section>

</custom-tabs>

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
