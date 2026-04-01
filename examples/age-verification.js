/**
 * Age verification example
 *
 * Initiates a browser-based age verification session via Portal's verification
 * service. The flow is:
 *
 *   1. Create a verification session → get a `session_url` to open in the browser
 *   2. Send a Cashu token request to the session's ephemeral key
 *   3. The user completes age verification in the browser
 *   4. The verification service mints a Cashu token and sends it back
 *   5. Receiving the token = proof of age ✅
 *
 * Usage:
 *   PORTAL_URL=http://localhost:7000 PORTAL_TOKEN=your-token node age-verification.js
 *
 * Requirements:
 *   - portal-rest must have [verification] api_key configured
 *   - portal-rest must have a wallet configured (NWC or Breez) — needed for relay connectivity
 */

import 'dotenv/config';
import { PortalClient } from 'portal-sdk';

const BASE_URL   = process.env.PORTAL_URL   ?? 'http://localhost:7000';
const AUTH_TOKEN = process.env.PORTAL_TOKEN ?? 'your-secret-token';

// Amount of sats to send as part of the Cashu request (matches Portal's default).
const VERIFICATION_AMOUNT = 500;

const client = new PortalClient({ baseUrl: BASE_URL, authToken: AUTH_TOKEN });

async function main() {
  // ── Step 1: create verification session ─────────────────────────────────
  console.log('Creating age verification session...');
  const session = await client.createVerificationSession();

  console.log('');
  console.log('┌─────────────────────────────────────────────────────┐');
  console.log('│  Open this URL in a browser to complete verification │');
  console.log('├─────────────────────────────────────────────────────┤');
  console.log(`│  ${session.session_url}`);
  console.log('└─────────────────────────────────────────────────────┘');
  console.log('');
  console.log(`Session ID:      ${session.session_id}`);
  console.log(`Ephemeral npub:  ${session.ephemeral_npub}`);
  console.log(`Expires at:      ${new Date(session.expires_at * 1000).toISOString()}`);
  console.log('');

  // ── Step 2: send Cashu request to the ephemeral key ─────────────────────
  // This signals to the verification worker that we're ready to receive the token.
  // The worker will hold the request until the user completes verification in the browser.
  console.log('Sending Cashu token request to verification worker...');
  const op = await client.requestPortalToken(
    session.ephemeral_npub,
    [], // no subkeys
    VERIFICATION_AMOUNT,
  );

  console.log(`Stream ID: ${op.streamId}`);
  console.log('Waiting for user to complete verification in browser...');

  // ── Step 3: wait for the result ──────────────────────────────────────────
  // This example runs as a plain CLI script (no webhook server, no auto-poller),
  // so we must explicitly poll for events until the stream reaches a terminal state.
  const result = await client.poll(op, { intervalMs: 1000, timeoutMs: 5 * 60 * 1000 });

  console.log('');
  if (result.status === 'success') {
    console.log('✅ Age verification successful!');
    console.log(`   Cashu token: ${result.token}`);
  } else if (result.status === 'rejected') {
    console.log(`❌ Verification rejected: ${result.reason ?? '(no reason)'}`);
  } else if (result.status === 'insufficient_funds') {
    console.log('❌ Verification failed: insufficient funds in Portal mint');
  } else {
    console.log('❌ Verification failed:', result);
  }
}

main().catch((err) => {
  console.error('Error:', err.message ?? err);
  process.exit(1);
});
