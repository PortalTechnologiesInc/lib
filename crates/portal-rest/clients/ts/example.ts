import { PortalSDK, Currency, SinglePaymentRequestContent, Timestamp, StreamEvent } from './src/index';

async function main() {
  // Create client — no WebSocket, just REST
  const client = new PortalSDK({
    baseUrl: process.env.PORTAL_URL ?? 'http://localhost:3000',
    authToken: process.env.PORTAL_AUTH_TOKEN ?? 'password12345',
    debug: false,
  });

  // ---- Health & Version ----
  console.log('=== Health Check ===');
  const health = await client.health();
  console.log('Health:', health);

  console.log('\n=== Version ===');
  const ver = await client.version();
  console.log('Version:', ver.version, 'Commit:', ver.git_commit);

  // ---- Key Handshake Flow ----
  console.log('\n=== Key Handshake ===');
  const handshake = await client.newKeyHandshakeUrl();
  console.log('Share this URL with your user:', handshake.url);
  console.log('Stream ID:', handshake.stream_id);

  // Poll until the user completes the handshake
  console.log('Waiting for handshake completion (poll)...');
  const handshakeResult = await client.pollUntilDone(
    handshake.stream_id,
    (event: StreamEvent) => {
      console.log('  Handshake event:', event.type);
    },
    { timeoutMs: 120_000 } // 2 minute timeout
  );
  const mainKey = (handshakeResult as unknown as { main_key: string }).main_key;
  console.log('User authenticated! Main key:', mainKey);

  // ---- Single Payment Flow ----
  console.log('\n=== Single Payment ===');
  const paymentReq: SinglePaymentRequestContent = {
    amount: 200,
    currency: 'EUR',
    description: 'Test payment',
  };
  const payment = await client.requestSinglePayment(mainKey, [], paymentReq);
  console.log('Payment stream ID:', payment.stream_id);

  // Poll for payment status updates
  console.log('Waiting for payment...');
  const paymentResult = await client.pollUntilDone(
    payment.stream_id,
    (event: StreamEvent) => {
      if (event.type === 'payment_status_update') {
        const status = (event as unknown as { status: { status: string } }).status;
        console.log('  Payment status:', status.status);
      }
    },
    { timeoutMs: 300_000 } // 5 minute timeout
  );
  console.log('Payment final event:', paymentResult);

  // ---- JWT Operations ----
  console.log('\n=== JWT Operations ===');
  const targetKey = mainKey;
  const token = await client.issueJwt(targetKey, 1);
  console.log('Issued JWT:', token.substring(0, 30) + '...');

  const claims = await client.verifyJwt(targetKey, token);
  console.log('Verified JWT target_key:', claims.target_key);

  // ---- Relay Management ----
  console.log('\n=== Relay Management ===');
  const relayUrl = 'wss://relay.damus.io';
  const added = await client.addRelay(relayUrl);
  console.log('Added relay:', added);
  const removed = await client.removeRelay(relayUrl);
  console.log('Removed relay:', removed);

  // ---- Calendar ----
  console.log('\n=== Calendar Next Occurrence ===');
  const next = await client.calculateNextOccurrence('daily', Timestamp.fromNow(0));
  if (next) {
    console.log('Next occurrence:', next.toDate().toISOString());
  }

  // ---- NIP-05 ----
  console.log('\n=== NIP-05 Lookup ===');
  try {
    const nip05 = await client.fetchNip05Profile('ancientdragon913@getportal.cc');
    console.log('NIP-05 pubkey:', nip05.public_key);
  } catch (e) {
    console.log('NIP-05 lookup failed:', e);
  }

  // ---- Wallet Info ----
  console.log('\n=== Wallet Info ===');
  const walletInfo = await client.getWalletInfo();
  console.log('Wallet type:', walletInfo.wallet_type, 'Balance:', walletInfo.balance_msat, 'msat');

  // ---- Profile ----
  console.log('\n=== Fetch Profile ===');
  const profile = await client.fetchProfile(mainKey);
  console.log('Profile:', profile);

  // ---- Request Invoice ----
  console.log('\n=== Request Invoice ===');
  try {
    const invoice = await client.requestInvoice(mainKey, [], {
      amount: 599,
      currency: 'EUR',
      expires_at: Timestamp.fromNow(3600),
      description: 'Test invoice request',
    });
    console.log('Invoice:', invoice.invoice);
    console.log('Payment hash:', invoice.payment_hash);
  } catch (e) {
    console.error('Request invoice failed:', e);
  }

  console.log('\nDone!');
}

main().catch(console.error);
