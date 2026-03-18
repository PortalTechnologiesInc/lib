# Error Handling

<custom-tabs category="sdk">

<div slot="title">HTTP</div>
<section>

The API returns standard HTTP status codes:

| Status | Meaning |
|--------|---------|
| `200` | Success |
| `400` | Bad request (invalid parameters) |
| `401` | Unauthorized (wrong or missing auth token) |
| `404` | Not found (stream_id expired or unknown) |
| `409` | Conflict (operation already in progress) |
| `500` | Internal server error |

Error responses include a JSON body:
```json
{ "error": "Invalid or unsupported version." }
```

For async operations, terminal error events arrive in the polling stream:
```bash
curl -s "$BASE_URL/events/$STREAM?after=0" -H "Authorization: Bearer $AUTH_TOKEN"
# → { "events": [{ "data": { "status": "error", "reason": "user_rejected" } }] }
```

</section>

<div slot="title">JavaScript</div>
<section>

The SDK throws `PortalSDKError` with a `code` property:

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

Check the `err` parameter in each `sendCommand` callback; handle connection and auth failures before sending commands.

```java
sdk.sendCommand(someRequest, (response, err) -> {
    if (err != null) {
        System.err.println("Command failed: " + err);
        return;
    }
    // use response
});
```

</section>

</custom-tabs>

---

**Next:** [Authentication Guide](../guides/authentication.md)
