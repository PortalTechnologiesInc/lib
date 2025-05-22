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

## API Endpoints

### REST Endpoints

- `GET /health`: Health check endpoint, returns "OK" when the server is running.
- `GET /ws`: WebSocket endpoint for real-time operations.

### WebSocket Commands

The WebSocket API is a command-based system: a command is sent, and a response is received.

Each command must be assigned a unique ID, generated on the client side, which is used to match the response to the corresponding command.

The first command **must** be an authentication command.


### Available Commands

#### `Auth`

Authentication command.

**Request:**
```json
{
  "id": "unique-id",
  "cmd": "Auth",
  "params": {
    "token": "<AUTH_TOKEN>"
  }
}
```

#### `NewAuthInitUrl`

Generate a new authentication initialization URL.

**Request:**
```json
{
  "id": "unique-id",
  "cmd": "NewAuthInitUrl"
}
```

#### `AuthenticateKey`

Authenticate a key.

**Request:**
```json
{
  "id": "unique-id",
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
  "id": "unique-id",
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
  "id": "unique-id",
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
  "id": "unique-id",
  "cmd": "FetchProfile",
  "params": {
    "main_key": "hex_encoded_pub_key"
  }
}
```

