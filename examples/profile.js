/**
 * Profile lookup example
 *
 * Fetches a user's Nostr profile by public key.
 *
 * Usage:
 *   PORTAL_URL=http://localhost:3000 PORTAL_TOKEN=your-token MAIN_KEY=user-pubkey-hex node profile.js
 */

import 'dotenv/config';
import { PortalClient } from 'portal-sdk';

const BASE_URL = process.env.PORTAL_URL ?? 'http://localhost:3000';
const AUTH_TOKEN = process.env.PORTAL_TOKEN ?? 'your-secret-token';
const MAIN_KEY = process.env.MAIN_KEY ?? 'replace-with-user-pubkey-hex';

const client = new PortalClient({ baseUrl: BASE_URL, authToken: AUTH_TOKEN });

async function main() {
  console.log(`Fetching profile for ${MAIN_KEY}...`);

  const profile = await client.fetchProfile(MAIN_KEY);

  if (!profile) {
    console.log('No profile found for this key.');
    return;
  }

  console.log('\n✓ Profile found:');
  if (profile.name)         console.log('  Name:        ', profile.name);
  if (profile.display_name) console.log('  Display name:', profile.display_name);
  if (profile.about)        console.log('  About:       ', profile.about);
  if (profile.picture)      console.log('  Picture:     ', profile.picture);
  if (profile.nip05)        console.log('  NIP-05:      ', profile.nip05);
}

main().catch((err) => {
  console.error('Error:', err.message ?? err);
  process.exit(1);
});
