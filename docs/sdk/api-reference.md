# TypeScript SDK API reference

Concise reference for the main `PortalSDK` methods. For examples and workflows, see [Basic Usage](basic-usage.md) and the [Guides](../guides/authentication.md).

## Lifecycle

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

## Payments

| Method | Description |
|--------|-------------|
| `requestSinglePayment(mainKey, subkeys, paymentRequest, onStatusChange): Promise<void>` | Request a one-time Lightning payment. |
| `requestRecurringPayment(mainKey, subkeys, paymentRequest): Promise<RecurringPaymentResponseContent>` | Request a recurring (subscription) payment. |
| `requestInvoicePayment(mainKey, subkeys, paymentRequest, onStatusChange): Promise<void>` | Pay an invoice on behalf of a user. |
| `requestInvoice(recipientKey, subkeys, content): Promise<InvoiceResponseContent>` | Request an invoice. |
| `closeRecurringPayment(mainKey, subkeys, subscriptionId): Promise<string>` | Close a recurring payment subscription. |
| `listenClosedRecurringPayment(onClosed): Promise<() => void>` | Listen for closed recurring payments; returns unsubscribe function. |

## Profiles and identity

| Method | Description |
|--------|-------------|
| `fetchProfile(mainKey)` | Fetch a user's Nostr profile (`Promise<Profile \| null>`). |
| `setProfile(profile): Promise<void>` | Set or update a profile. |
| `fetchNip05Profile(nip05): Promise<Nip05Profile>` | Resolve a NIP-05 identifier. |

## JWT

| Method | Description |
|--------|-------------|
| `issueJwt(target_key, duration_hours): Promise<string>` | Issue a JWT for the given key. |
| `verifyJwt(public_key, token): Promise<{ target_key: string }>` | Verify a JWT and return claims. |

## Relays and Cashu

| Method | Description |
|--------|-------------|
| `addRelay(relay): Promise<string>` | Add a relay. |
| `removeRelay(relay): Promise<string>` | Remove a relay. |
| `requestCashu(...)` | Request Cashu tokens. See [Cashu guide](../guides/cashu-tokens.md). |
| `sendCashuDirect(...)` | Send Cashu tokens. |
| `mintCashu(...)` | Mint Cashu tokens. |
| `burnCashu(...)` | Burn Cashu tokens. |
| `calculateNextOccurrence(calendar, from)` | Compute next occurrence for a recurrence calendar (`Promise<Timestamp \| null>`). |

## Events

| Method | Description |
|--------|-------------|
| `on(eventType \| EventCallbacks, callback?): void` | Register listener, e.g. `on('connected', fn)` or `on({ onConnected, onDisconnected, onError })`. |
| `off(eventType, callback): void` | Remove a listener. |

## Types overview

- **`Currency`** — e.g. `Currency.Millisats`.
- **`Timestamp`** — `Timestamp.fromDate(date)`, `Timestamp.fromNow(seconds)`, `toDate()`, `toJSON()`.
- **`Profile`** — `id`, `pubkey`, `name`, `display_name`, `picture`, `about`, `nip05`.
- **`RecurringPaymentRequestContent`**, **`SinglePaymentRequestContent`**, **`InvoiceRequestContent`** — See TypeScript definitions in the package.
- **`AuthResponseData`**, **`InvoiceStatus`**, **`RecurringPaymentStatus`** — Response and status types.

Full types are exported from `portal-sdk`; use your editor’s IntelliSense or the package source.

---

**Next:** [Error Handling](error-handling.md) for `PortalSDKError` and error codes.
