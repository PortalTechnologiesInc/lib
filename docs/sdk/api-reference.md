# SDK API reference

Concise reference for the main `PortalSDK` methods. For examples and workflows, see [Basic Usage](basic-usage.md) and the [Guides](../guides/authentication.md).

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

- **PortalSDK(healthEndpoint, wsEndpoint)** — Create client.
- **connect(authToken)** — Connect and authenticate.
- **sendCommand(request, callback)** — Send any command. Request classes: **AuthRequest**, **KeyHandshakeUrlRequest**, **RequestSinglePaymentRequest**, **MintCashuRequest**, **CalculateNextOccurrenceRequest**, and others in the SDK.

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

Use **RequestSinglePaymentRequest**, **MintCashuRequest**, and other request classes with **sendCommand**. See the [Java SDK repository](https://github.com/PortalTechnologiesInc/java-sdk) for the full list.

</section>

</custom-tabs>

## Profiles and identity

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

| Method | Description |
|--------|-------------|
| `fetchProfile(mainKey)` | Fetch a user's Nostr profile (`Promise<Profile \| null>`). |
| `setProfile(profile): Promise<void>` | Set or update a profile. |
| `fetchNip05Profile(nip05): Promise<Nip05Profile>` | Resolve a NIP-05 identifier. |

</section>

<div slot="title">Java</div>
<section>

Use the appropriate request classes with **sendCommand** for profile and identity operations.

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

Use the JWT-related request classes with **sendCommand**.

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
| `calculateNextOccurrence(calendar, from)` | Compute next occurrence for a recurrence calendar (`Promise<Timestamp \| null>`). |

</section>

<div slot="title">Java</div>
<section>

Use **KeyHandshakeUrlRequest**, **RequestSinglePaymentRequest**, **MintCashuRequest**, relay and Cashu request classes with **sendCommand**.

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

Responses and notifications are delivered in the **sendCommand** callback.

</section>

</custom-tabs>

## Types overview

<custom-tabs category="sdk">

<div slot="title">JavaScript</div>
<section>

- **`Currency`** — e.g. `Currency.Millisats`.
- **`Timestamp`** — `Timestamp.fromDate(date)`, `Timestamp.fromNow(seconds)`, `toDate()`, `toJSON()`.
- **`Profile`** — `id`, `pubkey`, `name`, `display_name`, `picture`, `about`, `nip05`.
- **`RecurringPaymentRequestContent`**, **`SinglePaymentRequestContent`**, **`InvoiceRequestContent`** — See type definitions in the package.
- **`AuthResponseData`**, **`InvoiceStatus`**, **`RecurringPaymentStatus`** — Response and status types.

Full types are exported from `portal-sdk`; use your editor’s IntelliSense or the package source.

</section>

<div slot="title">Java</div>
<section>

**PortalRequest**, **PortalResponse**, **PortalNotification**, and request/response classes (e.g. **CalculateNextOccurrenceRequest**). See the [Java SDK repository](https://github.com/PortalTechnologiesInc/java-sdk) for the full API.

</section>

</custom-tabs>

---

**Next:** [Error Handling](error-handling.md) for `PortalSDKError` and error codes.
