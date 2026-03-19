/**
 * Error handling example
 *
 * Verifies that the Portal REST API returns correct HTTP error codes
 * and error messages for invalid inputs. Useful for testing server behaviour
 * without a wallet or Nostr interaction.
 *
 * Usage:
 *   PORTAL_URL=http://localhost:3000 PORTAL_TOKEN=your-token node error-handling.js
 */

import 'dotenv/config';

const BASE_URL = process.env.PORTAL_URL ?? 'http://localhost:3000';
const AUTH_TOKEN = process.env.PORTAL_TOKEN ?? 'your-secret-token';

let passed = 0;
let failed = 0;

async function req(method, path, body, token = AUTH_TOKEN) {
  const opts = {
    method,
    headers: {
      'Authorization': `Bearer ${token}`,
      'Content-Type': 'application/json',
    },
  };
  if (body !== undefined) opts.body = JSON.stringify(body);
  const res = await fetch(`${BASE_URL}${path}`, opts);
  let json = null;
  try { json = await res.json(); } catch {}
  return { status: res.status, json };
}

async function expect(description, fn) {
  try {
    await fn();
    console.log(`  ✓ ${description}`);
    passed++;
  } catch (err) {
    console.log(`  ✗ ${description}`);
    console.log(`      ${err.message}`);
    failed++;
  }
}

function assert(condition, msg) {
  if (!condition) throw new Error(msg);
}

// ---- Test suites ----

async function testAuth() {
  console.log('\n[Auth]');

  await expect('no auth header → 401', async () => {
    const res = await fetch(`${BASE_URL}/version`);
    // /version is public — but test a protected endpoint
    const r = await fetch(`${BASE_URL}/info`);
    assert(r.status === 401, `expected 401, got ${r.status}`);
  });

  await expect('wrong token → 401', async () => {
    const { status } = await req('GET', '/info', undefined, 'wrong-token');
    assert(status === 401, `expected 401, got ${status}`);
  });

  await expect('valid token → 200', async () => {
    const { status } = await req('GET', '/info');
    assert(status === 200, `expected 200, got ${status}`);
  });
}

async function testPublicEndpoints() {
  console.log('\n[Public endpoints — no auth required]');

  await expect('GET /health → 200', async () => {
    const res = await fetch(`${BASE_URL}/health`);
    assert(res.status === 200, `expected 200, got ${res.status}`);
    const text = await res.text();
    assert(text === 'OK', `expected "OK", got "${text}"`);
  });

  await expect('GET /version → 200 with version field', async () => {
    const res = await fetch(`${BASE_URL}/version`);
    assert(res.status === 200, `expected 200, got ${res.status}`);
    const json = await res.json();
    assert(typeof json.version === 'string', `expected version string, got ${JSON.stringify(json)}`);
  });

  await expect('GET /.well-known/nostr.json → 200 (no auth)', async () => {
    const res = await fetch(`${BASE_URL}/well-known/nostr.json`);
    assert(res.status === 200, `expected 200, got ${res.status}`);
  });
}

async function testInvalidKeys() {
  console.log('\n[Invalid keys]');

  await expect('authenticate-key with invalid main_key → 400', async () => {
    const { status, json } = await req('POST', '/authenticate-key', {
      main_key: 'not-a-valid-hex-key',
      subkeys: [],
    });
    assert(status === 400, `expected 400, got ${status}`);
    assert(json?.error, `expected error message, got ${JSON.stringify(json)}`);
  });

  await expect('profile with invalid main_key → 400', async () => {
    const { status } = await req('GET', '/profile/not-a-pubkey');
    assert(status === 400, `expected 400, got ${status}`);
  });

  await expect('payments/single with invalid main_key → 400', async () => {
    const { status, json } = await req('POST', '/payments/single', {
      main_key: 'invalid',
      subkeys: [],
      payment_request: { description: 'test', amount: 1000, currency: 'millisats' },
    });
    assert(status === 400, `expected 400, got ${status}`);
  });

  await expect('jwt/issue with invalid target_key → 400', async () => {
    const { status } = await req('POST', '/jwt/issue', {
      target_key: 'not-hex',
      duration_hours: 24,
    });
    assert(status === 400, `expected 400, got ${status}`);
  });
}

async function testMissingFields() {
  console.log('\n[Missing required fields]');

  await expect('authenticate-key missing main_key → 400/422', async () => {
    const { status } = await req('POST', '/authenticate-key', {});
    assert([400, 422].includes(status), `expected 400 or 422, got ${status}`);
  });

  await expect('jwt/verify missing token → 400/422', async () => {
    const { status } = await req('POST', '/jwt/verify', { pubkey: 'abc' });
    assert([400, 422].includes(status), `expected 400 or 422, got ${status}`);
  });
}

async function testEventPolling() {
  console.log('\n[Event polling]');

  await expect('GET /events/nonexistent-stream → 404', async () => {
    const { status } = await req('GET', '/events/this-stream-does-not-exist-xyz123');
    assert(status === 404, `expected 404, got ${status}`);
  });

  await expect('GET /events/:id returns events array', async () => {
    // Create a real stream first via key-handshake
    const { status: s, json: j } = await req('POST', '/key-handshake', {});
    assert(s === 201, `expected 201 from key-handshake, got ${s}`);
    const streamId = j?.stream_id;
    assert(streamId, 'expected stream_id in response');

    // Poll it immediately — should return empty events array (no user yet)
    const { status, json } = await req('GET', `/events/${streamId}?after=0`);
    assert(status === 200, `expected 200, got ${status}`);
    assert(Array.isArray(json?.events), `expected events array, got ${JSON.stringify(json)}`);
  });
}

async function testCashuErrors() {
  console.log('\n[Cashu errors]');

  await expect('cashu/burn with invalid token → 4xx', async () => {
    const { status } = await req('POST', '/cashu/burn', {
      mint_url: 'https://mint.example.com',
      unit: 'sat',
      token: 'not-a-real-cashu-token',
    });
    assert(status >= 400, `expected 4xx, got ${status}`);
  });

  await expect('cashu/mint with invalid mint URL → 400', async () => {
    const { status } = await req('POST', '/cashu/mint', {
      mint_url: 'not-a-url',
      unit: 'sat',
      amount: 10,
    });
    assert(status === 400, `expected 400, got ${status}`);
  });
}

// ---- Run all suites ----

async function main() {
  console.log(`Portal REST API — error handling tests`);
  console.log(`Target: ${BASE_URL}\n`);

  await testPublicEndpoints();
  await testAuth();
  await testInvalidKeys();
  await testMissingFields();
  await testEventPolling();
  await testCashuErrors();

  console.log(`\n${'─'.repeat(40)}`);
  console.log(`Results: ${passed} passed, ${failed} failed`);

  if (failed > 0) process.exit(1);
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});
