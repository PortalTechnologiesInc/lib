# Versioning & Compatibility

Portal follows a simple versioning policy designed to make compatibility obvious.

## Version scheme

All Portal components use **semantic versioning** (`major.minor.patch`):

| Component | Package |
|-----------|---------|
| `portal-rest` (SDK Daemon) | Docker: `getportal/sdk-daemon` |
| TypeScript SDK | npm: `portal-sdk` |
| Java SDK | JitPack: `com.github.PortalTechnologiesInc:java-sdk` |

## Compatibility rule

**`major.minor` must match between the SDK and the SDK Daemon. The patch version is independent.**

In other words:

- SDK `0.3.0` ↔ SDK Daemon `0.3.0` ✅
- SDK `0.3.4` ↔ SDK Daemon `0.3.1` ✅ (patch is irrelevant)
- SDK `0.3.x` ↔ SDK Daemon `0.4.x` ❌ (minor mismatch)

Patch releases contain bug fixes and non-breaking improvements within the same `major.minor`. You can update the SDK or the Daemon independently as long as `major.minor` stays the same.

## Upgrading

When a new `major.minor` version is released:

1. Update your SDK dependency to the matching version.
2. Update the Docker image tag to the matching version.
3. Check the [CHANGELOG](../../CHANGELOG.md) for breaking changes.

**Example — upgrading to 0.4.0:**

```bash
# Docker
docker pull getportal/sdk-daemon:0.4.0
```

```bash
# npm
npm install portal-sdk@0.4.0
```

```groovy
// Gradle
implementation 'com.github.PortalTechnologiesInc:java-sdk:0.4.0'
```

## Current versions

| Component | Version |
|-----------|---------|
| SDK Daemon (`getportal/sdk-daemon`) | `0.3.0` |
| TypeScript SDK (`portal-sdk`) | `0.3.0` |
| Java SDK | `0.3.0` |
