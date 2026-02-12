# Error Handling

The SDK throws `PortalSDKError` with a code property. Check err instanceof PortalSDKError and use err.code:

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

```typescript
import { PortalSDKError } from 'portal-sdk';

try {
  await client.connect();
  await client.authenticate(token);
} catch (err) {
  if (err instanceof PortalSDKError) {
    // err.code: AUTH_FAILED, CONNECTION_TIMEOUT, CONNECTION_CLOSED, NOT_CONNECTED, etc.
  }
  throw err;
}
```

## Error codes

| Code | When |
|------|------|
| `NOT_CONNECTED` | Method called before `connect()` or after disconnect. |
| `CONNECTION_TIMEOUT` | Connection did not open within `connectTimeout`. |
| `CONNECTION_CLOSED` | Socket closed unexpectedly. |
| `AUTH_FAILED` | Invalid or rejected auth token. |
| `UNEXPECTED_RESPONSE` | Server sent unexpected response type. |
| `SERVER_ERROR` | Server returned an error (err.message). |
| `PARSE_ERROR` | Failed to parse a message; optional err.details. |

</section>

<div slot="title">Java</div>
<section>

Check the err parameter in each sendCommand callback; handle connection and auth failures before sending commands.

```java
sdk.sendCommand(someRequest, (response, err) -> {
    if (err != null) {
        System.err.println("Command failed: " + err);
        // or: throw new RuntimeException(err);
        return;
    }
    // use response
});
```

</section>

</custom-tabs>

---

**Next:** [Authentication Guide](../guides/authentication.md)
