/**
 * Example: Portal SDK with Express + webhook-based async flow.
 *
 * 1. Start portal-rest with webhook_url pointing to this server.
 * 2. Run: npx ts-node example.ts
 * 3. Open the handshake URL in a browser / scan QR.
 * 4. The webhook fires → handshake promise resolves → payment is initiated.
 */

import { createServer } from 'http';
import { PortalClient, Timestamp } from './src/index';

const PORT = parseInt(process.env.WEBHOOK_PORT ?? '4000', 10);
const PORTAL_URL = process.env.PORTAL_URL ?? 'http://localhost:3000';
const AUTH_TOKEN = process.env.PORTAL_AUTH_TOKEN ?? 'password12345';
const WEBHOOK_SECRET = process.env.PORTAL_WEBHOOK_SECRET ?? 'my-webhook-secret';

async function main() {
  // ---- 1. Create client ----
  const portal = new PortalClient({
    baseUrl: PORTAL_URL,
    authToken: AUTH_TOKEN,
    webhookSecret: WEBHOOK_SECRET,
    debug: true,
  });

  // ---- 2. Mount webhook handler in a plain HTTP server ----
  const handler = portal.webhookHandler();
  const server = createServer((req, res) => {
    if (req.method === 'POST' && req.url === '/portal/webhook') {
      return handler(req, res);
    }
    res.writeHead(404);
    res.end('Not found');
  });

  server.listen(PORT, () => {
    console.log(`Webhook server listening on http://localhost:${PORT}/portal/webhook`);
    console.log(`Make sure portal-rest is configured with webhook_url = http://localhost:${PORT}/portal/webhook`);
  });

  try {
    // ---- 3. Health check ----
    console.log('\n=== Health Check ===');
    const health = await portal.health();
    console.log('Health:', health);

    const ver = await portal.version();
    console.log('Version:', ver.version, 'Commit:', ver.git_commit);

    const serverInfo = await portal.info();
    console.log('Server public key:', serverInfo.public_key);

    // ---- 4. Key Handshake (async — resolved via webhook) ----
    console.log('\n=== Key Handshake ===');
    const handshake = await portal.newKeyHandshakeUrl();
    console.log('Share this URL with the user:', handshake.url);
    console.log('Stream ID:', handshake.streamId);
    console.log('Waiting for handshake completion (via webhook)...');

    const { main_key, preferred_relays } = await handshake.done;
    console.log('Handshake complete!');
    console.log('  Main key:', main_key);
    console.log('  Preferred relays:', preferred_relays);

    // ---- 5. Single Payment (async — resolved via webhook) ----
    console.log('\n=== Single Payment ===');
    const payment = await portal.requestSinglePayment(main_key, [], {
      amount: 200,
      currency: 'EUR',
      description: 'Example payment',
    });
    console.log('Payment stream ID:', payment.streamId);

    // Listen for intermediate events (e.g. user_approved)
    portal.onEvent(payment.streamId, (event) => {
      console.log('  [intermediate]', event.type, event);
    });

    console.log('Waiting for payment to complete (via webhook)...');
    const finalEvent = await payment.done;
    console.log('Payment complete:', finalEvent);

    // ---- 6. Synchronous operations ----
    console.log('\n=== Wallet Info ===');
    const wallet = await portal.getWalletInfo();
    console.log('Wallet:', wallet.wallet_type, '— Balance:', wallet.balance_msat, 'msat');

    console.log('\n=== JWT ===');
    const token = await portal.issueJwt(main_key, 1);
    console.log('JWT:', token.substring(0, 40) + '...');
    const claims = await portal.verifyJwt(serverInfo.public_key, token);
    console.log('Verified target_key:', claims.target_key);

    console.log('\n=== Relay Management ===');
    await portal.addRelay('wss://relay.damus.io');
    console.log('Added relay');
    await portal.removeRelay('wss://relay.damus.io');
    console.log('Removed relay');

    console.log('\n=== Calendar ===');
    const next = await portal.calculateNextOccurrence('daily', Timestamp.fromNow(0));
    console.log('Next daily occurrence:', next?.toDate().toISOString() ?? 'none');

    console.log('\n=== Profile ===');
    const profile = await portal.fetchProfile(main_key);
    console.log('Profile:', profile);

    
    console.log('\nAll done! Pending streams:', portal.pendingCount);
  } finally {
    server.close();
    console.log('Server closed.');
  }
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
