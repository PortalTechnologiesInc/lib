# Portal REST API

This crate provides a RESTful API for the Portal SDK, allowing it to be used from any programming language via a local REST API server.

## Getting Started

- [Configuration](#configuration)
- [Start programming](#start-programming)
- [API Endpoints](#api-endpoints)


## Configuration

> **Important**: The rest daemon will create the working directory and configuration file automatically if they do not exist. You may create or customize them manually before starting the daemon if you wish.

### Setup Steps

1. **Create working directory**:
   ```bash
   mkdir -p ~/.portal-rest
   ```

2. **Create configuration file**:
   ```bash
   # Copy and customize the example config
   cp example.config.toml ~/.portal-rest/config.toml
   # Edit ~/.portal-rest/config.toml with your settings
   ```

3. **Start the rest daemon**:
   ```bash
   portal-rest  # Uses ~/.portal-rest/config.toml automatically
   ```

### Configuration File Locations (in order of precedence)

1. **Default location**: `~/.portal-rest/config.toml`
2. **Environment variables**: All config options can be set via environment variables


### Environment Variables

All configuration options can be overridden via environment variables. The format is:

```
PORTAL__<SECTION>__<KEY>=value
```

Use double underscores (`__`) to separate nested keys.

| Config Key | Environment Variable | Description |
|------------|---------------------|-------------|
| `info.listen_port` | `PORTAL__INFO__LISTEN_PORT` | The port on which the REST API will listen |
| `nostr.private_key` | `PORTAL__NOSTR__PRIVATE_KEY` | Nostr private key in hex format |
| `nostr.relays` | `PORTAL__NOSTR__RELAYS` | Comma-separated list of relay URLs |
| `nostr.subkey_proof` | `PORTAL__NOSTR__SUBKEY_PROOF` | Nostr subkey proof (optional) |
| `auth.auth_token` | `PORTAL__AUTH__AUTH_TOKEN` | API authentication token |
| `wallet.ln_backend` | `PORTAL__WALLET__LN_BACKEND` | Wallet type: `none`, `nwc`, or `breez` |
| `wallet.nwc.url` | `PORTAL__WALLET__NWC__URL` | Nostr Wallet Connect URL |
| `wallet.breez.api_key` | `PORTAL__WALLET__BREEZ__API_KEY` | Breez API key |
| `wallet.breez.storage_dir` | `PORTAL__WALLET__BREEZ__STORAGE_DIR` | Breez storage directory |
| `wallet.breez.mnemonic` | `PORTAL__WALLET__BREEZ__MNEMONIC` | Breez mnemonic |

**Example:**

```bash
PORTAL__WALLET__LN_BACKEND=nwc PORTAL__WALLET__NWC__URL="nostr+walletconnect://..." ./portal-rest
```

### Building and Running

```
cargo build --release
./target/release/rest
```

The server will start on `127.0.0.1:3000`.

### Build the Docker Image with Nix

To build the Docker image for your architecture using Nix:

```bash
nix build .#rest-docker
```

This will produce a Docker image tarball in `result/`. You can load it into Docker with:

```bash
docker load < result
```

### Multi-architecture (merged) manifest

To create and push a multi-architecture manifest (for both amd64 and arm64):

1. Build and load both images on their respective architectures (or use emulation):
   - On amd64: `nix build .#rest-docker` (tag as `getportal/sdk-daemon:amd64`)
   - On arm64: `nix build .#rest-docker` (tag as `getportal/sdk-daemon:arm64`)
2. Push both images to Docker Hub:

```bash
docker push getportal/sdk-daemon:amd64
docker push getportal/sdk-daemon:arm64
```

3. Create and push the merged manifest:

```bash
docker manifest create getportal/sdk-daemon:latest \
  --amend getportal/sdk-daemon:amd64 \
  --amend getportal/sdk-daemon:arm64
docker manifest push getportal/sdk-daemon:latest
```

## Start programming

Since this is a REST API, you can use it from any programming language that supports websocket connections.

But best is to use the official SDK for your programming language.

Currently supported SDKs:
- [TypeScript](clients/ts/README.md)
- [Java](https://github.com/PortalTechnologiesInc/jvm-client)


## API Endpoints

### Authentication

All REST API endpoints require a Bearer token for authentication:

```
Authorization: Bearer <AUTH_TOKEN>
```

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

#### `NewKeyHandshakeUrl`

Generate a new authentication initialization URL.

**Request:**
```json
{
  "id": "unique-id",
  "cmd": "NewKeyHandshakeUrl"
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

#### `IssueJwt`

Issue a JWT token for a given public key.

**Request:**
```json
{
  "id": "unique-id",
  "cmd": "IssueJwt",
  "params": {
    "pubkey": "hex_encoded_pub_key",
    "expires_at": 1234567890
  }
}
```

**Response:**
```json
{
  "type": "success",
  "id": "unique-id",
  "data": {
    "type": "issue_jwt",
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }
}
```

#### `VerifyJwt`

Verify a JWT token and return the claims.

**Request:**
```json
{
  "id": "unique-id",
  "cmd": "VerifyJwt",
  "params": {
    "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..."
  }
}
```

**Response:**
```json
{
  "type": "success",
  "id": "unique-id",
  "data": {
    "type": "verify_jwt",
    "pubkey": "02eec5685e141a8fc6ee91e3aad0556bdb4f7b8f3c8c8c8c8c8c8c8c8c8c8c8c8",
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
      cmd: 'NewKeyHandshakeUrl'
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
