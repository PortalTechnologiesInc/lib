# SDK API reference

Concise reference for the main PortalSDK methods. For examples and workflows, see [Basic Usage](basic-usage.md) and the [Guides](../guides/authentication.md).

**SDK references:** [JavaScript/TypeScript SDK](https://www.npmjs.com/package/portal-sdk) (npm) · [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk) (GitHub)

## Lifecycle & Auth

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `connect(): Promise<void>` | Connect to Portal. Call once before other methods. |
| `disconnect(): void` | Close connection and clear state. |
| `authenticate(token: string): Promise<void>` | Authenticate with your auth token (required after connect). |

## Auth and users

| Method | Description |
|--------|-------------|
| `newKeyHandshakeUrl(onKeyHandshake, staticToken?, noRequest?): Promise<string>` | Get URL for user key handshake; callback runs when user completes handshake. |
| `authenticateKey(mainKey, subkeys?): Promise<AuthResponseData>` | Authenticate a user key (NIP-46 style). |

</section>

<div slot="title">Java</div>
<section>

| Class / method | Description |
|----------------|-------------|
| `PortalSDK(wsEndpoint)` | Create client with WebSocket URL. |
| `connect()` | Establish WebSocket connection (blocking). |
| `authenticate(authToken)` | Authenticate with your token (sends `AuthRequest` internally). |
| `sendCommand(request, (response, err) -> { ... })` | Send any command. Request classes below. |

Auth and users: KeyHandshakeUrlRequest(notificationCallback) or (staticToken, noRequest, notificationCallback); AuthenticateKeyRequest(mainKey, subkeys). Response: KeyHandshakeUrlResponse.url(), AuthenticateKeyResponse.

</section>

</custom-tabs>

## Payments

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `requestSinglePayment(mainKey, subkeys, paymentRequest, onStatusChange): Promise<void>` | Request a one-time Lightning payment. |
| `requestRecurringPayment(mainKey, subkeys, paymentRequest): Promise<RecurringPaymentResponseContent>` | Request a recurring (subscription) payment. |
| `requestInvoicePayment(mainKey, subkeys, paymentRequest, onStatusChange): Promise<void>` | Pay an invoice on behalf of a user. |
| `requestInvoice(recipientKey, subkeys, content): Promise<InvoiceResponseContent>` | Request an invoice. |
| `closeRecurringPayment(mainKey, subkeys, subscriptionId): Promise<string>` | Close a recurring payment subscription. |
| `listenClosedRecurringPayment(onClosed): Promise<() => void>` | Listen for closed recurring payments; returns unsubscribe function. |

</section>

<div slot="title">Java</div>
<section>

| Request class | Description |
|---------------|-------------|
| `RequestSinglePaymentRequest(mainKey, subkeys, paymentContent, statusNotificationCallback)` | One-time Lightning payment. |
| `RequestRecurringPaymentRequest(mainKey, subkeys, paymentContent)` | Recurring (subscription) payment. |
| `RequestInvoicePaymentRequest(...)` | Pay an invoice. |
| `RequestInvoiceRequest(...)` | Request an invoice. |
| `CloseRecurringPaymentRequest(mainKey, subkeys, subscriptionId)` | Close a subscription. |
| `ListenClosedRecurringPaymentRequest(onClosedCallback)` | Listen for user cancellations. |

Content types: SinglePaymentRequestContent, RecurringPaymentRequestContent (with RecurrenceInfo). See [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

</custom-tabs>

## Profiles and identity

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `fetchProfile(mainKey)` | Fetch a user's Nostr profile (Promise&lt;Profile | null&gt;). |
| `setProfile(profile): Promise<void>` | Set or update a profile. |
| `fetchNip05Profile(nip05): Promise<Nip05Profile>` | Resolve a NIP-05 identifier. |

</section>

<div slot="title">Java</div>
<section>

| Request class | Description |
|---------------|-------------|
| `FetchProfileRequest(mainKey)` | Fetch Nostr profile. Response: `FetchProfileResponse.profile()`. |
| `SetProfileRequest(profile)` | Set or update a profile. `Profile` model: name, displayName, picture, nip05, etc. |
| `FetchNip05ProfileRequest(nip05)` | Resolve NIP-05 identifier. |

</section>

</custom-tabs>

## JWT

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `issueJwt(target_key, duration_hours): Promise<string>` | Issue a JWT for the given key. |
| `verifyJwt(public_key, token): Promise<{ target_key: string }>` | Verify a JWT and return claims. |

</section>

<div slot="title">Java</div>
<section>

| Request class | Description |
|---------------|-------------|
| `IssueJwtRequest(targetKey, durationHours)` | Issue a JWT. Response: `IssueJwtResponse.token()`. |
| `VerifyJwtRequest(publicKey, token)` | Verify a JWT. Response: `VerifyJwtResponse` (claims). |

</section>

</custom-tabs>

## Relays and Cashu

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `addRelay(relay): Promise<string>` | Add a relay. |
| `removeRelay(relay): Promise<string>` | Remove a relay. |
| `requestCashu(...)` | Request Cashu tokens. See [Cashu guide](../guides/cashu-tokens.md). |
| `sendCashuDirect(...)` | Send Cashu tokens. |
| `mintCashu(...)` | Mint Cashu tokens. |
| `burnCashu(...)` | Burn Cashu tokens. |
| `calculateNextOccurrence(calendar, from)` | Compute next occurrence for a recurrence calendar (Promise&lt;Timestamp | null&gt;). |

</section>

<div slot="title">Java</div>
<section>

| Request class | Description |
|---------------|-------------|
| `AddRelayRequest(relayUrl)` | Add a relay. |
| `RemoveRelayRequest(relayUrl)` | Remove a relay. |
| `RequestCashuRequest(mintUrl, unit, amount, recipientKey, subkeys)` | Request Cashu tokens from user. |
| `MintCashuRequest(mintUrl, staticToken?, unit, amount, description?)` | Mint Cashu tokens. |
| `BurnCashuRequest(mintUrl, staticToken?, unit, token)` | Burn (redeem) a token. |
| `SendCashuDirectRequest(mainKey, subkeys, token)` | Send Cashu token to user. |
| `CalculateNextOccurrenceRequest(calendar, fromTimestamp)` | Next occurrence for recurrence. |

See [Cashu guide](../guides/cashu-tokens.md) and [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

</custom-tabs>

## Events

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `on(eventType \| EventCallbacks, callback?): void` | Register listener, e.g. `on('connected', fn)` or `on({ onConnected, onDisconnected, onError })`. |
| `off(eventType, callback): void` | Remove a listener. |

</section>

<div slot="title">Java</div>
<section>

Responses and notifications are delivered in the `sendCommand` callback.

</section>

</custom-tabs>

## Types overview

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

- **Currency** — e.g. Currency.Millisats.
- **Timestamp** — Timestamp.fromDate(date), Timestamp.fromNow(seconds), toDate(), toJSON().
- **Profile** — id, pubkey, name, display_name, picture, about, nip05.
- **RecurringPaymentRequestContent**, **SinglePaymentRequestContent**, **InvoiceRequestContent** — See type definitions in the package.
- **AuthResponseData**, **InvoiceStatus**, **RecurringPaymentStatus** — Response and status types.

Full types are exported from portal-sdk; use your editor’s IntelliSense or the package source.

</section>

<div slot="title">Java</div>
<section>

| Type | Description |
|------|-------------|
| `PortalRequest`, `PortalResponse`, `PortalNotification` | Base types for sendCommand. |
| `Currency` | e.g. `Currency.MILLISATS`. |
| `SinglePaymentRequestContent(description, amount, currency, ...)` | Single payment params. |
| `RecurringPaymentRequestContent(..., RecurrenceInfo, expiresAt)` | Recurring payment params. |
| `RecurrenceInfo(..., calendar, ..., firstPaymentDue)` | Calendar: "weekly", "monthly", etc. |
| `Profile(name, displayName, picture, nip05)` | Nostr profile model. |

All request/response/notification classes in cc.getportal.command.request, cc.getportal.command.response, cc.getportal.command.notification, cc.getportal.model. See [Java SDK](https://github.com/PortalTechnologiesInc/java-sdk).

</section>

</custom-tabs>

---

**Next:** [Error Handling](error-handling.md) for PortalSDKError and error codes.
