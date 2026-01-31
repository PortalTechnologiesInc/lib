# Portal

Portal is a **Nostr-based authentication and payment SDK** for authenticating users and processing payments via Nostr and the Lightning Network.

- **SDKs:** TypeScript/JavaScript ([npm](https://www.npmjs.com/package/portal-sdk)), [Java](https://github.com/PortalTechnologiesInc/java-sdk)
- **API:** REST + WebSocket server in this repo (`portal-rest`); run it yourself or use a hosted endpoint

**Documentation:** **[https://portaltechnologiesinc.github.io/lib/](https://portaltechnologiesinc.github.io/lib/)** — Quick Start, TypeScript SDK, guides (auth, payments, profiles, Cashu, JWT), and API reference.

---

## Quick links

| Topic | Link |
|-------|------|
| Quick Start, Docker, env vars, building from source | [Getting Started](https://portaltechnologiesinc.github.io/lib/getting-started/quick-start.html) |
| TypeScript SDK install, usage, config, errors | [TypeScript SDK](https://portaltechnologiesinc.github.io/lib/sdk/installation.html) |
| Auth, payments, profiles, Cashu, JWT, relays | [Guides](https://portaltechnologiesinc.github.io/lib/guides/authentication.html) |
| FAQ, glossary, troubleshooting | [Resources](https://portaltechnologiesinc.github.io/lib/resources/faq.html) |

---

## Repository structure

| Path | Description |
|------|-------------|
| `crates/portal-rest` | Portal API server; [README](crates/portal-rest/README.md) for running it |
| `crates/portal-rest/clients/ts` | TypeScript SDK source; [README](crates/portal-rest/clients/ts/README.md) |
| `crates/portal` | Core Portal library (Rust) |
| `crates/portal-app`, `portal-sdk`, `portal-wallet`, `portal-cli`, `portal-rates` | App, SDK core, wallets, CLI, rates |
| `react-native` | React Native bindings |
| `backend` | Example backend |
| `docs` | Documentation source (mdBook); published to [GitHub Pages](https://portaltechnologiesinc.github.io/lib/) |

---

## License

MIT where noted. See [LICENSE](LICENSE).
