# Running a Custom Cashu Mint

Run your own Cashu mint to issue custom tokens and tickets with Portal's enhanced CDK implementation.

## Why Run Your Own Mint?

Running your own Cashu mint gives you:

- **Custom Units**: Create custom ticket types beyond just "sats"
- **Full Control**: Complete control over issuance and redemption
- **Privacy**: Tokens are untraceable, users maintain privacy
- **Branding**: Add custom images and metadata to tokens
- **Event Tickets**: Perfect for issuing event tickets, vouchers, or access tokens
- **No Intermediaries**: Direct issuance without third parties

## Portal's Enhanced CDK

Portal maintains a fork of Cashu CDK with enhanced features:
- **Custom Units**: Define multiple ticket types with different denominations
- **Metadata**: Add titles, descriptions, and images to each unit
- **Event Information**: Include date and location for event tickets
- **Authentication**: Built-in static token authentication
- **Portal Wallet Backend**: Integration with Portal's Lightning backend

## Quick Start with Docker

### 1. Pull the Docker Image

```bash
docker pull getportal/cdk-mintd:latest
```

### 2. Create Configuration File

#### Simple Configuration (Recommended for Getting Started)

Create `config.toml` with a basic fungible token:

```toml
[info]
url = "https://mint.yourdomain.com"
listen_host = "127.0.0.1"
listen_port = 3338

[mint_info]
name = "My Cashu Mint"
description = "A simple Cashu mint"

[ln]
ln_backend = "portalwallet"
mint_max = 100000
melt_max = 100000

[portal_wallet.supported_units]
sat = 32  # Standard satoshi unit

[portal_wallet.unit_info.sat]
title = "Satoshi"
description = "Standard Bitcoin satoshi token"
show_individually = false  # Show as fungible currency
url = "https://yourdomain.com"

[portal_wallet.unit_info.sat.kind.Event]
date = "01/01/1970"
location = "Worldwide"

[database]
engine = "sqlite"

[auth]
mint_max_bat = 50
enabled_mint = true
enabled_melt = true
enabled_swap = false
enabled_restore = false
enabled_check_proof_state = false

[auth.method.Static]
token = "your-secure-static-token"
```

#### Advanced Configuration (Event Tickets)

For event ticketing with multiple custom units, create `config.toml`:

```toml
[info]
url = "https://mint.yourdomain.com"
listen_host = "0.0.0.0"
listen_port = 3338

[mint_info]
name = "My Custom Mint"
description = "Cashu mint for custom tokens and tickets"

[ln]
ln_backend = "portalwallet"
mint_max = 100000  # Maximum minting amount
melt_max = 100000  # Maximum melting amount

[portal_wallet]
# Define custom units
[portal_wallet.supported_units]
vip = 32       # 32 denomination keysets
general = 32
early_bird = 32

# Configure each unit's metadata
[portal_wallet.unit_info.vip]
title = "VIP Pass"
description = "VIP access with all perks"
show_individually = true
front_card_background = "https://yourdomain.com/images/vip-front.png"
back_card_background = "https://yourdomain.com/images/vip-back.png"

[portal_wallet.unit_info.vip.kind.Event]
date = "2026-12-31"
location = "New York, USA"

[portal_wallet.unit_info.general]
title = "General Admission"
description = "General admission ticket"
show_individually = true
front_card_background = "https://yourdomain.com/images/general-front.png"
back_card_background = "https://yourdomain.com/images/general-back.png"

[portal_wallet.unit_info.general.kind.Event]
date = "2026-12-31"
location = "New York, USA"

[portal_wallet.unit_info.early_bird]
title = "Early Bird"
description = "Special early bird pricing"
show_individually = true
front_card_background = "https://yourdomain.com/images/early-front.png"
back_card_background = "https://yourdomain.com/images/early-back.png"

[portal_wallet.unit_info.early_bird.kind.Event]
date = "2026-12-31"
location = "New York, USA"

[database]
engine = "sqlite"

[auth]
mint_max_bat = 50
enabled_mint = true
enabled_melt = true
enabled_swap = false
enabled_restore = false
enabled_check_proof_state = false

[auth.method.Static]
token = "your-secure-static-token-here"
```

### 3. Run the Mint

The simplest way to run the mint:

```bash
docker run -d \
  --name cashu-mint \
  -p 3338:3338 \
  -v $(pwd)/config.toml:/config.toml:ro \
  -v mint-data:/data \
  -e CDK_MINTD_MNEMONIC="<your mnemonic here>" \
  getportal/cdk-mintd:latest
```

**With Custom Paths:**

```bash
docker run -d \
  --name cashu-mint \
  -p 3338:3338 \
  -v /path/to/config.toml:/config.toml:ro \
  -v /path/to/data:/data \
  -e CDK_MINTD_MNEMONIC="<your mnemonic here>" \
  getportal/cdk-mintd:latest
```

**Options Explained:**
- `-p 3338:3338` - Expose port 3338
- `-v config.toml:/config.toml:ro` - Mount config file (read-only)
- `-v mint-data:/data` - Persist database
- `-e CDK_MINTD_MNEMONIC="<your mnemonic here>"` - Set mnemonic as evironment variable
- `getportal/cdk-mintd:latest` - Use latest image

**Quick Test (Temporary):**

For testing without persistence:

```bash
docker run --rm \
  --name test-mint \
  -p 3338:3338 \
  -v $(pwd)/config.toml:/config.toml:ro \
  -e CDK_MINTD_MNEMONIC="<your mnemonic here>" \
  getportal/cdk-mintd:latest
```

### 4. Verify Mint is Running

```bash
# Check logs
docker logs cashu-mint

# Test mint endpoint (locally)
curl http://localhost:3338/v1/info

# Should return mint info JSON
```

Example response:
```json
{
  "name": "My Cashu Mint",
  "description": "A simple Cashu mint",
  "pubkey": "...",
  "version": "...",
  "nuts": {...}
}
```

## Configuration Options

### Mint Information

```toml
[info]
url = "https://mint.yourdomain.com"  # Public URL of your mint
listen_host = "0.0.0.0"              # IP to bind to (0.0.0.0 for all)
listen_port = 3338                   # Port to listen on

[mint_info]
name = "My Mint"                     # Displayed name
description = "Description of mint"   # Mint description
```

### Lightning Backend

```toml
[ln]
ln_backend = "portalwallet"  # Use Portal's wallet backend
mint_max = 100000            # Max amount per mint operation (msats)
melt_max = 100000            # Max amount per melt operation (msats)
```

### Custom Units

#### Fungible Tokens (like normal currency)

```toml
[portal_wallet.supported_units]
sat = 32  # Or any other name

[portal_wallet.unit_info.sat]
title = "Satoshi"
description = "Standard fungible token"
show_individually = false  # Important: false for fungible tokens
url = "https://yourdomain.com"

[portal_wallet.unit_info.sat.kind.Event]
date = "01/01/1970"
location = "Worldwide"
```

#### Non-Fungible Tickets

Define custom token types for tickets:

```toml
[portal_wallet.supported_units]
vip = 32  # 32 denominations (powers of 2)
general = 32

[portal_wallet.unit_info.vip]
title = "VIP Ticket"
description = "Full access pass"
show_individually = true  # Important: true for individual tickets
front_card_background = "https://example.com/vip-front.png"
back_card_background = "https://example.com/vip-back.png"

# Event-specific metadata
[portal_wallet.unit_info.vip.kind.Event]
date = "2026-12-31"
location = "City, Country"
```

**Key Difference:**
- `show_individually = false` - Tokens are fungible (like money)
- `show_individually = true` - Each token is unique (like tickets)

### Authentication

Protect minting operations with a static token:

```toml
[auth]
[auth.method.Static]
token = "your-secret-token"

mint_max_bat = 50           # Max batch size
enabled_mint = true         # Allow minting
enabled_melt = true         # Allow melting
enabled_swap = false        # Disable swapping
enabled_restore = false     # Disable restore
enabled_check_proof_state = false
```

### Database

```toml
[database]
engine = "sqlite"  # SQLite for simplicity
# Or use PostgreSQL:
# engine = "postgres"
# connection_string = "postgresql://user:pass@localhost/mintdb"
```

## Using Your Custom Mint

### Minting Tokens (Fungible)

```typescript
import { PortalSDK } from 'portal-sdk';

const client = new PortalSDK({
  serverUrl: 'ws://localhost:3000/ws'
});

await client.connect();
await client.authenticate(process.env.AUTH_TOKEN);

// Mint fungible tokens (like satoshis)
const token = await client.mintCashu(
  'http://localhost:3338',
  'your-static-token',  // From config.toml
  'sat',                // Unit name from config
  10,                   // Amount (10 sats worth)
  'Payment for service'
);

// Send to user
await client.sendCashuDirect(userPubkey, [], token);
```

### Minting Tickets (Non-Fungible)

```typescript
// Mint a VIP ticket
const vipToken = await client.mintCashu(
  'http://localhost:3338',
  'your-static-token',  // Static token for authentication
  'vip',                // Custom unit
  1,                    // Amount (1 VIP ticket)
  'VIP access for event'
);

// Send to user
await client.sendCashuDirect(userPubkey, [], vipToken);
```

### Burning/Redeeming Tokens

```typescript
// Request token back from user
const result = await client.requestCashu(
  userPubkey,
  [],
  'http://localhost:3338',
  'sat',  // Same unit type as minted
  10      // Amount
);

if (result.status === 'success') {
  // Burn to verify and claim
  const amount = await client.burnCashu(
    'http://localhost:3338',
    'sat',
    result.token,
    'your-static-token'  // From config.toml
  );
  
  console.log('Valid token! Claimed:', amount);
  // Grant access or process payment
}
```

**For Tickets (Non-Fungible):**

```typescript
const result = await client.requestCashu(
  userPubkey,
  [],
  'http://localhost:3338',
  'vip',  // Ticket unit
  1       // Amount
);

if (result.status === 'success') {
  const amount = await client.burnCashu(
    'http://localhost:3338',
    'vip',
    result.token,
    'your-static-token'
  );
  
  console.log('Valid VIP ticket! Granting access...');
  // Grant access to VIP area
}
```

## Building from Source

To build from Portal's CDK fork:

### 1. Clone the Repository

```bash
git clone https://github.com/PortalTechnologiesInc/cdk.git
cd cdk-mintd
```

### 2. Build with Cargo

```bash
cargo build --release
```

### 3. Run the Mint

```bash
MINT_CONFIG=config.toml \
MNEMONIC_FILE=mnemonic.txt \
./target/release/cdk-mintd
```

### 4. Or Build with Nix

```bash
nix build
./result/bin/cdk-mintd
```

## Production Deployment

### With Reverse Proxy (Nginx)

```nginx
server {
    listen 443 ssl http2;
    server_name mint.yourdomain.com;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location / {
        proxy_pass http://localhost:3338;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

### With Docker Compose

```yaml
version: '3.8'

services:
  cashu-mint:
    image: getportal/cdk-mintd:latest
    container_name: cashu-mint
    ports:
      - "3338:3338"
    volumes:
      - ./config.toml:/config.toml:ro
      - mint-data:/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3338/v1/info"]
      interval: 30s
      timeout: 10s
      retries: 3

volumes:
  mint-data:
```

Start with:
```bash
docker-compose up -d
```

### Environment Variables

The mint looks for `/config.toml` by default. You can override with:

```bash
docker run -d \
  -e CONFIG_PATH=/custom/path/config.toml \
  -e RUST_LOG=debug \
  -v $(pwd)/config.toml:/custom/path/config.toml:ro \
  -v mint-data:/data \
  getportal/cdk-mintd:latest
```

**Available Variables:**
- `CONFIG_PATH` - Path to config file (default: `/config.toml`)
- `RUST_LOG` - Log level (`error`, `warn`, `info`, `debug`, `trace`)
- `DATA_DIR` - Data directory (default: `/data`)

## Use Cases

### Event Ticketing

Create different ticket tiers with custom images:

```toml
[portal_wallet.supported_units]
vip = 32
general = 32
student = 32

[portal_wallet.unit_info.vip]
title = "VIP Pass"
description = "Full access with backstage pass"
front_card_background = "https://event.com/vip-front.png"
back_card_background = "https://event.com/vip-back.png"

[portal_wallet.unit_info.vip.kind.Event]
date = "2026-08-15"
location = "Convention Center, NYC"
```

### Gift Vouchers

```toml
[portal_wallet.supported_units]
voucher_50 = 32
voucher_100 = 32

[portal_wallet.unit_info.voucher_50]
title = "$50 Gift Card"
description = "Redeemable for any product"
show_individually = true
```

### Access Tokens

```toml
[portal_wallet.supported_units]
premium = 32
basic = 32

[portal_wallet.unit_info.premium]
title = "Premium Access"
description = "6 months premium membership"
```

## Security Best Practices

### 1. Protect Configuration File

```bash
# Set read-only permissions
chmod 600 config.toml

# Mount as read-only in Docker
docker run -v $(pwd)/config.toml:/config.toml:ro ...
```

### 2. Rotate Static Tokens

Regularly update your authentication token:

```toml
[auth.method.Static]
token = "new-secure-token"
```

### 3. Use HTTPS

Always run behind HTTPS in production:
- Let's Encrypt certificates
- Reverse proxy (Nginx, Caddy)
- Valid SSL/TLS configuration

### 4. Rate Limiting

Implement rate limiting at the reverse proxy level:

```nginx
limit_req_zone $binary_remote_addr zone=mint:10m rate=10r/s;

location / {
    limit_req zone=mint burst=20;
    proxy_pass http://localhost:3338;
}
```

### 5. Monitor the Mint

```bash
# Check mint logs
docker logs -f cashu-mint

# Monitor database size
du -sh mint-data/

# Watch for errors
docker logs cashu-mint 2>&1 | grep ERROR
```

## Monitoring & Maintenance

### Health Checks

```bash
# Check mint info
curl https://mint.yourdomain.com/v1/info

# Check keysets
curl https://mint.yourdomain.com/v1/keys

# Check specific unit
curl https://mint.yourdomain.com/v1/keys/vip
```

### Backup

```bash
# Backup database (most important!)
docker exec cashu-mint sqlite3 /data/mint.db ".backup '/data/backup.db'"
docker cp cashu-mint:/data/backup.db ./backup.db

# Or backup entire data directory
docker run --rm \
  -v mint-data:/data \
  -v $(pwd)/backups:/backup \
  alpine tar czf /backup/mint-backup-$(date +%Y%m%d).tar.gz /data

# Backup config
cp config.toml /secure/backup/location/
```

### Logs

```bash
# View logs
docker logs cashu-mint

# Follow logs
docker logs -f cashu-mint

# Save logs
docker logs cashu-mint > mint.log 2>&1
```

## Troubleshooting

### Mint Won't Start

```bash
# Check logs
docker logs cashu-mint

# Verify config is mounted
docker exec cashu-mint cat /config.toml

# Check permissions
docker exec cashu-mint ls -la /config.toml /data
```

### Can't Mint Tokens

- Verify static token is correct
- Check `enabled_mint = true` in config
- Ensure Lightning backend is configured
- Check mint hasn't reached `mint_max`

### Authentication Errors

```bash
# Test with curl
curl -X POST https://mint.yourdomain.com/v1/mint/quote/bolt11 \
  -H "Authorization: Bearer your-static-token" \
  -H "Content-Type: application/json" \
  -d '{"amount": 100, "unit": "sat"}'
```

### Database Issues

```bash
# Check database file
docker exec cashu-mint ls -lh /app/data/

# Verify permissions
docker exec cashu-mint ls -la /app/data/
```

## Advanced: Multiple Units

Create a complex ticket system:

```toml
[portal_wallet.supported_units]
early_bird = 32
regular = 32
vip = 32
sponsor = 32

[portal_wallet.unit_info.early_bird]
title = "Early Bird Special"
description = "Limited early bird pricing"
show_individually = true
front_card_background = "https://event.com/early-front.png"
back_card_background = "https://event.com/early-back.png"

[portal_wallet.unit_info.early_bird.kind.Event]
date = "2026-06-01"
location = "Conference Center"

[portal_wallet.unit_info.regular]
title = "Regular Admission"
description = "Standard entry ticket"
show_individually = true
front_card_background = "https://event.com/regular-front.png"
back_card_background = "https://event.com/regular-back.png"

[portal_wallet.unit_info.regular.kind.Event]
date = "2026-06-01"
location = "Conference Center"

# ... VIP and Sponsor configurations ...
```

## Resources

- **Portal's CDK Fork**: [github.com/PortalTechnologiesInc/cdk](https://github.com/PortalTechnologiesInc/cdk)
- **Cashu Protocol**: [cashu.space](https://cashu.space)
- **Docker Image**: [hub.docker.com/r/getportal/cdk-mintd](https://hub.docker.com/r/getportal/cdk-mintd)

---

**Next Steps**:
- [Cashu Tokens Guide](cashu-tokens.md) - Using tokens with Portal SDK
- [Docker Deployment](../getting-started/docker-deployment.md) - Deploy securely
- [Troubleshooting](../advanced/troubleshooting.md) - Common issues

