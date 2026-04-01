/**
 * Age verification example
 *
 * Initiates a browser-based age verification session via Portal's verification
 * service. A single call handles the full flow:
 *
 *   1. Create a verification session → get a `session_url` to open in the browser
 *   2. Automatically starts listening for the verification token
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

const client = new PortalClient({ baseUrl: BASE_URL, authToken: AUTH_TOKEN });

async function main() {
  // ── Single call: create session + start listening for token ──────────────
  console.log('Creating age verification session...');
  const session = await client.createVerificationSession();

  const sessionUrl = session.session_url.startsWith('http') ? session.session_url : `https://verify.getportal.cc${session.session_url}`;
  const expiresAtMs = session.expires_at * 1000;
  const timeoutMs = expiresAtMs - Date.now();

  console.log('');
  console.log('┌─────────────────────────────────────────────────────┐');
  console.log('│  Open this URL in a browser to complete verification │');
  console.log('├─────────────────────────────────────────────────────┤');
  console.log(`│  ${sessionUrl}`);
  console.log('└─────────────────────────────────────────────────────┘');
  console.log('');
  console.log(`Session ID:      ${session.session_id}`);
  console.log(`Ephemeral npub:  ${session.ephemeral_npub}`);
  console.log(`Stream ID:       ${session.streamId}`);
  console.log(`Expires at:      ${new Date(expiresAtMs).toISOString()}`);
  console.log('');
  console.log('Waiting for user to complete verification in browser...');

  // ── Wait for the result ──────────────────────────────────────────────────
  const result = await client.poll(session, { intervalMs: 1000, timeoutMs });

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
