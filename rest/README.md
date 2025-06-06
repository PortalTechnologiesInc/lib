# Portal REST API

This crate provides a RESTful API for the Portal SDK, allowing it to be used from any programming language via a local REST API server.

## Setup

### Environment Variables

The following environment variables need to be set:

- `AUTH_TOKEN`: Required. The authentication token used to authenticate with the API.
- `NOSTR_KEY`: Required. Your Nostr private key in hex format.
- `NWC_URL`: Optional. The Nostr Wallet Connect URL.
- `NOSTR_SUBKEY_PROOF`: Optional. The Nostr subkey proof if using subkeys.
- `NOSTR_RELAYS`: Optional. Comma-separated list of relay URLs. Defaults to common relays if not provided.

### Building and Running

```
cargo build --release
./target/release/rest
```

The server will start on `127.0.0.1:3000`.

## Authentication

All REST API endpoints require a Bearer token for authentication:

```
Authorization: Bearer <AUTH_TOKEN>
```

For WebSocket connections, the first message must be an authentication command:

```json
{
  "cmd": "Auth",
  "params": {
    "token": "<AUTH_TOKEN>"
  }
}
```

## API Endpoints

### REST Endpoints

- `GET /health`: Health check endpoint, returns "OK" when the server is running.
- `GET /ws`: WebSocket endpoint for real-time operations.

### WebSocket Commands

All WebSocket messages use the following format:

**Request:**
```json
{
  "cmd": "CommandName",
  "params": {
    // Command-specific parameters
  }
}
```

**Success Response:**
```json
{
  "type": "success",
  "id": "request-uuid",
  "data": {
    // Command-specific response data
  }
}
```

**Error Response:**
```json
{
  "type": "error",
  "id": "request-uuid",
  "message": "Error message"
}
```

### Available Commands

#### `NewAuthInitUrl`

Generate a new authentication initialization URL.

**Request:**
```json
{
  "cmd": "NewAuthInitUrl"
}
```

#### `AuthenticateKey`

Authenticate a key.

**Request:**
```json
{
  "cmd": "AuthenticateKey",
  "params": {
    "main_key": "hex_encoded_pub_key",
    "subkeys": ["hex_encoded_pub_key", ...]
  }
}
```

#### `RequestRecurringPayment`

Request a recurring payment.

**Request:**
```json
{
  "cmd": "RequestRecurringPayment",
  "params": {
    "main_key": "hex_encoded_pub_key",
    "subkeys": ["hex_encoded_pub_key", ...],
    "payment_request": {
      // Recurring payment request details
    }
  }
}
```

#### `RequestSinglePayment`

Request a single payment.

**Request:**
```json
{
  "cmd": "RequestSinglePayment",
  "params": {
    "main_key": "hex_encoded_pub_key",
    "subkeys": ["hex_encoded_pub_key", ...],
    "payment_request": {
      // Single payment request details
    }
  }
}
```

#### `FetchProfile`

Fetch a profile for a public key.

**Request:**
```json
{
  "cmd": "FetchProfile",
  "params": {
    "main_key": "hex_encoded_pub_key"
  }
}
```

#### `CloseSubscription`

Close a recurring payment for a recipient.

**Request:**
```json
{
  "cmd": "CloseSubscription",
  "params": {
    "recipient_key": "hex_encoded_pub_key",
    "subscription_id": ""
  }
}
```

## Example Integration (JavaScript)

```javascript
// Connect to WebSocket
const ws = new WebSocket('ws://localhost:3000/ws');

// Send authentication when connection opens
ws.onopen = () => {
  ws.send(JSON.stringify({
    cmd: 'Auth',
    params: {
      token: 'your-auth-token'
    }
  }));
};

// Handle messages
ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log('Received:', response);
  
  if (response.type === 'success' && response.data.message === 'Authenticated successfully') {
    // Now authenticated, can send commands
    ws.send(JSON.stringify({
      cmd: 'NewAuthInitUrl'
    }));
  }
};

// Handle errors
ws.onerror = (error) => {
  console.error('WebSocket error:', error);
};

// Handle disconnection
ws.onclose = () => {
  console.log('WebSocket connection closed');
};
``` 