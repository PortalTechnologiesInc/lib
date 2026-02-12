# Running a Custom Cashu Mint

Portal uses a Cashu CDK fork. Run your own mint to issue custom tokens (e.g. event tickets, vouchers) with full control and optional custom units/metadata.

## Docker

```bash
docker pull getportal/cdk-mintd:latest
```

Create config.toml (minimal):

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
sat = 32

[portal_wallet.unit_info.sat]
title = "Satoshi"
description = "Standard Bitcoin satoshi token"
show_individually = false
url = "https://yourdomain.com"

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

Run:

```bash
docker run -d \
  --name cashu-mint \
  -p 3338:3338 \
  -v $(pwd)/config.toml:/config.toml:ro \
  -v mint-data:/data \
  -e CDK_MINTD_MNEMONIC="<your mnemonic>" \
  getportal/cdk-mintd:latest
```

Verify: `curl http://localhost:3338/v1/info`

## Config reference

- **info** — url, listen_host, listen_port
- **mint_info** — name, description
- **ln** — ln_backend = "portalwallet", mint_max, melt_max
- **portal_wallet.supported_units** — unit name = keyset size (e.g. sat = 32)
- **portal_wallet.unit_info.<unit>** — title, description, show_individually (false = fungible, true = tickets), optional front_card_background, back_card_background; kind.Event with date, location
- **auth** — enabled_mint, enabled_melt, etc.; auth.method.Static — token for mint auth

Use the static token from config when calling `mintCashu` / `burnCashu` with your mint URL (see [Cashu Tokens](cashu-tokens.md)).

## Build from source

Portal CDK: [github.com/PortalTechnologiesInc/cdk](https://github.com/PortalTechnologiesInc/cdk). Build with Cargo or Nix; run with MINT_CONFIG and MNEMONIC_FILE or equivalent env.

## Production

Run behind HTTPS (reverse proxy). Use Docker Compose or mount config and data; set CONFIG_PATH, RUST_LOG, DATA_DIR if needed.

---

**Next:** [Cashu Tokens](cashu-tokens.md) · [Docker Deployment](../getting-started/docker-deployment.md)
