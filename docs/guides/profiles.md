# Profile Management

Fetch and manage user profiles from the Nostr network.

## Fetching User Profiles

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

```bash
curl -s $BASE_URL/profile/USER_PUBKEY_HEX \
  -H "Authorization: Bearer $AUTH_TOKEN"
# → { "name": "alice", "display_name": "Alice", "picture": "https://...", "nip05": "alice@example.com", ... }
```

</section>

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

## Profile Fields

- **name**: Username (no spaces)
- **display_name**: Display name (can have spaces)
- **picture**: Profile picture URL
- **about**: Bio/description
- **nip05**: Nostr verified identifier (like email)

---

**Next**: [JWT Tokens](jwt-tokens.md)
