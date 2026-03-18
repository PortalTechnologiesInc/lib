# REST API

The SDK Daemon exposes a standard HTTP REST API. You don't need the JavaScript or Java SDK — any HTTP client works: curl, Python requests, Go's net/http, Ruby's Faraday, etc.

## Base URL & Auth

```
BASE_URL=http://localhost:3000
AUTH_TOKEN=your-secret-token
```

All requests require:
```
Authorization: Bearer $AUTH_TOKEN
Content-Type: application/json
```

## Async operations

Most Portal operations (payments, auth, key handshake) are async — the user must approve in their wallet before a result is available. The pattern is always:

1. **Start the operation** → receive a `stream_id`
2. **Poll for events** until the operation completes

```bash
# Step 1: start operation (example: authenticate a key)
STREAM=$(curl -s -X POST $BASE_URL/authenticate-key \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"main_key": "...", "subkeys": []}' \
  | jq -r .stream_id)

# Step 2: poll until done
while true; do
  EVENTS=$(curl -s "$BASE_URL/events/$STREAM?after=0" \
    -H "Authorization: Bearer $AUTH_TOKEN")
  echo $EVENTS | jq .
  # check if terminal event received, then break
  sleep 1
done
```

### Event polling

```
GET /events/{stream_id}?after={index}
```

Returns events published since `after`. Start at `after=0`, then pass the last received `index + 1` on subsequent calls.

Response:
```json
{
  "events": [
    { "index": 0, "type": "StatusUpdate", "timestamp": 1234567890, "data": { ... } }
  ]
}
```

Terminal events (no more polling needed): status `paid`, `approved`, `declined`, `user_rejected`, `timeout`, `error`.

### Webhooks (alternative to polling)

Instead of polling, configure a webhook URL and the daemon will `POST` events to your endpoint as they arrive. See [Configuration](../getting-started/environment-variables.md) for `PORTAL__WEBHOOK_URL` and `PORTAL__WEBHOOK_SECRET`.

## Key endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/version` | GET | Daemon version |
| `/key-handshake` | POST | Generate auth URL for user |
| `/authenticate-key` | POST | Authenticate a key |
| `/payments/single` | POST | Request single payment |
| `/payments/recurring` | POST | Request recurring payment |
| `/payments/recurring/close` | POST | Close recurring subscription |
| `/invoices/request` | POST | Request an invoice |
| `/invoices/pay` | POST | Pay a BOLT11 invoice |
| `/cashu/request` | POST | Request Cashu tokens |
| `/profile/{main_key}` | GET | Fetch user profile |
| `/events/{stream_id}` | GET | Poll async operation events |

Full schema and request/response types: [API Reference](api-reference-rest.md).

## Examples

### Authentication flow

```bash
# 1. Get a key handshake URL (show this to your user as QR or link)
curl -s -X POST $BASE_URL/key-handshake \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{}'

# Response: { "stream_id": "abc123", "url": "nostr+walletconnect://..." }
# → Show the URL to the user. Poll the stream for the user's key.
```

### Single payment

```bash
# 1. Request payment (amount in millisats)
curl -s -X POST $BASE_URL/payments/single \
  -H "Authorization: Bearer $AUTH_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "main_key": "USER_PUBKEY_HEX",
    "subkeys": [],
    "description": "Premium - 1 month",
    "amount": 10000,
    "currency": "millisats"
  }'

# Response: { "stream_id": "xyz789" }

# 2. Poll for result
curl -s "$BASE_URL/events/xyz789?after=0" \
  -H "Authorization: Bearer $AUTH_TOKEN"

# Response: { "events": [{ "index": 0, "type": "StatusUpdate", "data": { "status": "paid", "preimage": "..." } }] }
```

### Profile lookup

```bash
curl -s $BASE_URL/profile/USER_PUBKEY_HEX \
  -H "Authorization: Bearer $AUTH_TOKEN"
```

---

- [API Reference (OpenAPI)](api-reference-rest.md) · [Configuration](../getting-started/environment-variables.md) · [SDK Installation](installation.md)
