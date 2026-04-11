# Age Verification — API Reference

The age verification endpoints. For the full API, see the [OpenAPI Reference](../sdk/api-reference-rest.md).

All requests require:
```
Authorization: Bearer $AUTH_TOKEN
Content-Type: application/json
```

## Create verification session

```
POST /verification/sessions
```

Creates a new verification session. Returns a URL to redirect the user to.

**Request body:**
```json
{}
```

**Response:**
```json
{
  "session_id": "abc-123",
  "session_url": "https://verify.getportal.cc/?id=abc-123",
  "ephemeral_npub": "npub1...",
  "expires_at": 1234567890,
  "stream_id": "def-456"
}
```

| Field | Description |
|-------|-------------|
| `session_url` | URL to redirect the user to for verification |
| `stream_id` | Use this to poll for the verification result |
| `expires_at` | Unix timestamp when the session expires |

## Request verification token from verified user

```
POST /verification/token
```

Request a verification proof from a user who has already verified (e.g. through the Portal mobile app). No redirect needed.

**Request body:**
```json
{
  "recipient_key": "USER_PUBKEY_HEX",
  "subkeys": []
}
```

**Response:**
```json
{
  "stream_id": "ghi-789"
}
```

Poll the event stream for the verification proof.

## Poll for verification result

```
GET /events/{stream_id}?after={index}
```

Poll for verification events. Start with `after=0`, then use the last received `index + 1`.

**Response:**
```json
{
  "events": [
    {
      "index": 0,
      "type": "CashuResponse",
      "timestamp": 1234567890,
      "data": {
        "status": "success",
        "token": "cashuA..."
      }
    }
  ]
}
```

### Result statuses

| Status | Description |
|--------|-------------|
| `success` | Verification passed. `token` field contains the verification proof. |
| `rejected` | Verification failed. `reason` may contain details. |
| `insufficient_funds` | Service could not issue the proof. Retry later. |

## Redeem verification proof

After receiving a verification proof, redeem it to prevent replay attacks:

```
POST /cashu/burn
```

**Request body:**
```json
{
  "mint_url": "https://mint.getportal.cc",
  "unit": "multi",
  "token": "cashuA..."
}
```

**Response:**
```json
{
  "amount": 1
}
```

---

**Full API:** [OpenAPI Reference](../sdk/api-reference-rest.md) · [REST API Guide](../sdk/rest-api.md)
