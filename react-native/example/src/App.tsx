import { View, StyleSheet, Button, TextInput } from 'react-native';
import { PortalApp, Keypair, parseAuthInitUrl, initLogger, AuthChallengeEvent, type AuthChallengeListener, Mnemonic, generateMnemonic, AuthResponseStatus, CloseRecurringPaymentResponse, RelayStatus, type PaymentRequestListener, PaymentRequestContent, SinglePaymentRequest, PaymentResponseContent, PaymentStatus, RecurringPaymentRequest, RecurringPaymentResponseContent, RecurringPaymentStatus, type ClosedRecurringPaymentListener } from 'portal-app-lib';
import { useState } from 'react';

async function main(authInitUrl: string) {
  initLogger();

  try {
    // Generate a new random mnemonic
    // const mnemonic = generateMnemonic();
    // Or load a mnemonic from a string
    const mnemonic = new Mnemonic("ocean citizen innocent sheriff kit much involve addict machine fine hill zone");

    // Get a nostr keypair object (right now we only support master keys)
    const keypair = mnemonic.getKeypair();
    console.log(keypair.publicKey());

    // Construct the app
    const app = await PortalApp.create(keypair, ["wss://relay.nostr.net"]);
    app.listen(); // No await here because we want to listen in the background

    // Setup the listener for auth challenges (login requests)
    class AuthChallengeListenerImpl implements AuthChallengeListener {
      async onAuthChallenge(event: AuthChallengeEvent): Promise<AuthResponseStatus> {
        console.log("Auth challenge received", event);

        // return new AuthResponseStatus.Declined({
        //   reason: "Not approved",
        // });

        return new AuthResponseStatus.Approved({
          grantedPermissions: [],
          sessionToken: "jwt-like-session-token",
        });
      }
    }
    app.listenForAuthChallenge(new AuthChallengeListenerImpl());

    class PaymentRequestListenerImpl implements PaymentRequestListener {
      async onSinglePaymentRequest(event: SinglePaymentRequest): Promise<PaymentResponseContent> {
        console.log("Single payment request received", event);
        return {
          requestId: event.content.requestId,
          status: new PaymentStatus.Pending(),
        };
      }

      async onRecurringPaymentRequest(event: RecurringPaymentRequest): Promise<RecurringPaymentResponseContent> {
        console.log("Recurring payment request received", event);
        return {
          requestId: event.content.requestId,
          status: new RecurringPaymentStatus.Confirmed({
            authorizedAmount: event.content.amount,
            authorizedCurrency: event.content.currency,
            authorizedRecurrence: event.content.recurrence,
            subscriptionId: "random-subscription-id",
          }),
        };
      }
    }
    app.listenForPaymentRequest(new PaymentRequestListenerImpl());

    class ClosedRecurringPaymentListenerImpl implements ClosedRecurringPaymentListener {
      async onClosedRecurringPayment(event: CloseRecurringPaymentResponse): Promise<void> {
        console.log("Closed subscription received", event);
      }
    }
    app.listenClosedRecurringPayment(new ClosedRecurringPaymentListenerImpl());

    await app.closeRecurringPayment("npub1ek206p7gwgqzgc6s7sfedmlu87cz9894jzzq0283t72lhz3uuxwsgn9stz", "random-subscription-id");

    // await app.sendAuthInit(parseAuthInitUrl("portal://npub1ek206p7gwgqzgc6s7sfedmlu87cz9894jzzq0283t72lhz3uuxwsgn9stz?relays=wss%3A%2F%2Frelay.damus.io,wss%3A%2F%2Frelay.nostr.net&token=eea752dc-3247-4dfa-8d6d-8304e98b1a75"));

    const status = await app.connectionStatus();
    status.forEach((value, key) => {
      if (value === RelayStatus.Connected) {
        console.log(key);
      }
    });
  } catch (error) {
    console.log(error);
  }
}

export default function App() {
  const [authInitUrl, setAuthInitUrl] = useState("");

  return (
    <View style={styles.container}>
      <TextInput placeholder="Enter the auth init url" value={authInitUrl} onChangeText={setAuthInitUrl} />
      <Button title="Send Auth Init" onPress={() => {
        main(authInitUrl);
      }} />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
  },
});
