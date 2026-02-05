# Portal

[![Documentation](https://img.shields.io/badge/docs-portaltechnologiesinc.github.io-blue)](https://portaltechnologiesinc.github.io/lib/)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

Portal is a Nostr-based authentication and payment SDK allowing applications to authenticate users and process payments through Nostr and Lightning Network.

Use the [TypeScript](https://www.npmjs.com/package/portal-sdk) or [Java](https://github.com/PortalTechnologiesInc/java-sdk) SDK, or run the API yourself.

**[Official Documentation](https://portaltechnologiesinc.github.io/lib/)**

---

### Quick start (Docker)

```bash
docker run -d -p 3000:3000 \
  -e PORTAL__AUTH__AUTH_TOKEN=your-secret-token \
  -e PORTAL__NOSTR__PRIVATE_KEY=your-nostr-private-key-hex \
  getportal/sdk-daemon:latest
```

For setup, SDK usage, and guides, see the [documentation](https://portaltechnologiesinc.github.io/lib/).

---

**License:** MIT. See [LICENSE](LICENSE).
