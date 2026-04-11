# Java SDK

The official Portal SDK for JVM apps (Android, Spring, etc.).

**Source:** [GitHub](https://github.com/PortalTechnologiesInc/java-sdk)

## Installation

**Gradle** (build.gradle):
```groovy
repositories {
    maven { url 'https://jitpack.io' }
}
dependencies {
    implementation 'com.github.PortalTechnologiesInc:java-sdk:0.4.1'
}
```

**Maven** (pom.xml):
```xml
<repository>
    <id>jitpack.io</id>
    <url>https://jitpack.io</url>
</repository>
<dependency>
    <groupId>com.github.PortalTechnologiesInc</groupId>
    <artifactId>java-sdk</artifactId>
    <version>0.4.1</version>
</dependency>
```

Requires Java 17+.

> **Compatibility:** The SDK `major.minor` version must match the SDK Daemon (`getportal/sdk-daemon`). Patch versions are independent. See [Versioning](../resources/versioning.md).

## Quick start

```java
import cc.getportal.PortalClient;
import cc.getportal.PortalClientConfig;

PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
);

// Authenticate a user
var operation = client.newKeyHandshakeUrl();
System.out.println("Share with user: " + operation.url());

var result = client.pollUntilComplete(operation);
System.out.println("User key: " + result.main_key());
```

## Configuration

```java
// Manual polling (default)
PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
);

// Auto-polling every 2 seconds
PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
                      .autoPolling(2000)
);

// Webhook mode
PortalClient client = new PortalClient(
    PortalClientConfig.create("http://localhost:3000", "your-auth-token")
                      .webhookSecret("your-webhook-secret")
);
```

## API Reference

All commands use `sdk.sendCommand(request, (response, err) -> { ... })`.

### Auth & Users

| Request class | Description |
|---------------|-------------|
| `KeyHandshakeUrlRequest(notificationCallback)` | Get URL for user key handshake. `KeyHandshakeUrlResponse.url()` |
| `KeyHandshakeUrlRequest(staticToken, noRequest, callback)` | With static token and/or no-request mode. |
| `AuthenticateKeyRequest(mainKey, subkeys)` | Authenticate a user key. |

### Payments

| Request class | Description |
|---------------|-------------|
| `RequestSinglePaymentRequest(mainKey, subkeys, paymentContent, statusCallback)` | One-time Lightning payment. |
| `RequestRecurringPaymentRequest(mainKey, subkeys, paymentContent)` | Recurring (subscription) payment. |
| `RequestInvoicePaymentRequest(...)` | Pay an invoice. |
| `RequestInvoiceRequest(...)` | Request an invoice. |
| `CloseRecurringPaymentRequest(mainKey, subkeys, subscriptionId)` | Close a subscription. |
| `ListenClosedRecurringPaymentRequest(onClosedCallback)` | Listen for user cancellations. |

### Profiles & Identity

| Request class | Description |
|---------------|-------------|
| `FetchProfileRequest(mainKey)` | Fetch profile. Response: `FetchProfileResponse.profile()` |
| `SetProfileRequest(profile)` | Set or update profile. `Profile(name, displayName, picture, nip05)` |
| `FetchNip05ProfileRequest(nip05)` | Resolve NIP-05 identifier. |

### JWT

| Request class | Description |
|---------------|-------------|
| `IssueJwtRequest(targetKey, durationHours)` | Issue a JWT. Response: `IssueJwtResponse.token()` |
| `VerifyJwtRequest(publicKey, token)` | Verify a JWT. Response: `VerifyJwtResponse` |

### Verification

| Request class | Description |
|---------------|-------------|
| `CreateVerificationSessionRequest(relays?)` | Create an age verification session. |
| `RequestVerificationTokenRequest(recipientKey, subkeys)` | Request a verification token from a user. |

### Cashu & Relays

| Request class | Description |
|---------------|-------------|
| `RequestCashuRequest(mintUrl, unit, amount, recipientKey, subkeys)` | Request Cashu tokens from user. |
| `MintCashuRequest(mintUrl, staticToken?, unit, amount, description?)` | Mint Cashu tokens. |
| `BurnCashuRequest(mintUrl, staticToken?, unit, token)` | Burn (redeem) a token. |
| `SendCashuDirectRequest(mainKey, subkeys, token)` | Send Cashu token to user. |
| `AddRelayRequest(relayUrl)` | Add a relay. |
| `RemoveRelayRequest(relayUrl)` | Remove a relay. |

## Error Handling

Check the `err` parameter in each `sendCommand` callback:

```java
sdk.sendCommand(someRequest, (response, err) -> {
    if (err != null) {
        System.err.println("Command failed: " + err);
        return;
    }
    // use response
});
```

## Types

| Type | Description |
|------|-------------|
| `Currency` | e.g. `Currency.MILLISATS` |
| `SinglePaymentRequestContent(description, amount, currency, ...)` | Single payment params |
| `RecurringPaymentRequestContent(..., RecurrenceInfo, expiresAt)` | Recurring payment params |
| `RecurrenceInfo(..., calendar, ..., firstPaymentDue)` | Calendar: `"weekly"`, `"monthly"`, etc. |
| `Profile(name, displayName, picture, nip05)` | Profile model |

All classes in `cc.getportal.command.request`, `cc.getportal.command.response`, `cc.getportal.command.notification`, `cc.getportal.model`.

---

**See also:** [JavaScript SDK](javascript.md) · [REST API](rest-api.md) · [OpenAPI Reference](api-reference-rest.md)
