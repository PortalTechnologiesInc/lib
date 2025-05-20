# portal-app-lib

React Native bindings for the Portal App library

## Building

The recommended way to build the library is to use the provided `flake.nix` which includes all the required depdendencies. You can load the flake by using `direnv` or manually running `nix develop` within this directory.

Then install the node deps with `yarn`. Once that's done you can build the library by running `yarn ubrn:android` and `yarn ubrn:ios`.

## Usage

The library needs to be initialized by first constructing a `Keypair` instance. The `Keypair` is generally constructed from a mnemonic as such:

```ts
// New random menmonic
const mnemonicObj = generateMnemonic();
// OR: Parse existing mnemonic
const mnemonicObj = new Mnemonic("...");

const keypair = mnemonicObj.getKeypair();

// You can get the public key of a keypair with the getter
const publicKey = keypair.publicKey();
// The public key can be serialized to a npub string with `toString()`:
const publicKeyString = publicKey.toString();
```

Then with a keypair you can construct a `PortalApp` instance which is the main object used to interact with the protocol. After construcing the instance you will need to "spawn" the background task by calling `listen()` on the instance. Since this needs to run in background, DO NOT call `await` on this. We just need the task to keep going.

```ts
const portalInstance = await PortalApp.create(keypair, ["wss://relay.nostr.net"]);
portalInstance.listen(); // Notice the missing await here
```

The `PortalApp` instance exposes a few methods to interact with the protocol. First, when scanning a QR code or receiving a `portal://` deep link, you can use the library to send the "AUTH_INIT" ping to the service like this:

```ts
const parsedUrl = parseAuthInitUrl(url);
await portalInstance.sendAuthInit(parsedUrl);
```

After sending this ping, the service will discover the user's public key, and will decide how to move forward. Generally it will send an authentication challenge or a payment request.

You can setup listeners for those requests as follows. Notice that those listeners will live forever once added and listen for requests continuously.

```ts
class LocalAuthChallengeListener implements AuthChallengeListener {
  onAuthChallenge(event: AuthChallengeEvent): Promise<boolean> {
    // Do something with the event. Return true/false to approve/reject the request
    return Promise.resolve(true);
  }
}
await portalInstance.listenForAuthChallenge(new LocalAuthChallengeListener());
```

```ts
class LocalPaymentRequestListener implements PaymentRequestListener {
  onSinglePaymentRequest(event: SinglePaymentRequest): Promise<PaymentStatusContent> {
    // Do something with the event. Return a `PaymentStatusContent` to signal the service what you intend to do
    // with this payment. If you intend to accept the payment, send `new PaymentStatusContent.Pending()`. This signals
    // the service that the payment is pending, and you will have to send it via NWC (see below).
    // If you want to reject the payment send `new PaymentStatusContent.Rejected({ reason: 'User rejected' })`.
    return Promise.resolve(new PaymentStatusContent.Pending());
  }

  onRecurringPaymentRequest(event: RecurringPaymentRequest): Promise<RecurringPaymentStatusContent> {
    // Do something with the event. Return a `RecurringPaymentStatusContent`. If the user accepts the request, generate a new
    // random `subscriptionId` and send the following:
    // new RecurringPaymentStatusContent.Confirmed({
    //   subscriptionId,
    //   authorizedAmount: event.content.amount,
    //   authorizedCurrency: event.content.currency,
    //   authorizedRecurrence: event.content.recurrence,
    // })
    // The `subscriptionId` will be sent in future `SinglePaymentRequest` when those payment requests are part of a subscription.
    // If you want to reject the request send a `new RecurringPaymentStatusContent.Rejected({ reason: 'User rejected' })`.
    return Promise.resolve(new RecurringPaymentStatusContent.Confirmed({
      subscriptionId: "randomsubscriptionid",
      authorizedAmount: event.content.amount,
      authorizedCurrency: event.content.currency,
      authorizedRecurrence: event.content.recurrence,
    }));
  }
}
await portalInstance.listenForPaymentRequest(new LocalPaymentRequestListener());
```

You can fetch/set Nostr profiles using the following APIs:

```ts
const maybeProfile = await portalInstance.fetchProfile(publicKey);

// Set a profile using this API. Note that all fields are optional and could be omitted
await portalInstance.setProfile({
    name: "Name",
    displayName: "Display Name",
    picture: "https://url-of-the-picture",
    nip05: "satoshi@getportal.cc",
});
```

The library also provides utilities to interact with `Calendar` objects, which are used to express complex recurring events. The protocol
follows the calendar format as described by [systemd timers](https://www.freedesktop.org/software/systemd/man/latest/systemd.time.html#Calendar%20Events).

You will find those calendar objects inside `RecurringPaymentRequest` events (`event.content.recurrence.calendar`).

```ts
// Receive an object inside a request
const calendarObj = event.content.recurrence.calendar;
// OR: parse from string
const calendarObj = parseCalendar("daily");

// You can also get the calendar string by calling `.toString()`, which is useful to store the subscription in a database
// since you cannot store calendar objects directly
const calendarString = calendarObj.toString();

// You can use `nextOccurrence(from: Timestamp)` to calculate the timestamp of the next occurrence after a point in time. For example, a `daily` event
// triggers every day at midnight. If it's now 11pm calling `calendar.nextOccurrence(nowTimestamp)` will return a timestamp which is 1h in the future (at
// midnight the following day). If there is no occurrence it will return null.
// NOTE: timestamps are expressed in seconds, not milliseconds. In JS/TS if you use `(new Date()).getTime()` it generally returns the timestamp in ms, so
// you will have to divide by 1000.
const nextOccurrence = calendarObj.nextOccurrence(lastOccurrenceTs);

// You can also print the calendar string in a human readable format (for example "every day", "every month at 3pm") by using:
const humanReadableString = calendarObj.toHumanReadable(false);
```

You can interact with a NWC wallet using the `NWC` structure:

```ts
const wallet = new NWC("nostr+walletconnect://url-for-nwc");

// Pay an invoice
const preimage = await wallet.payInvoice("lnbc...");
// Lookup invoice
const status = await wallet.lookupInvoice("lnbc...");
```