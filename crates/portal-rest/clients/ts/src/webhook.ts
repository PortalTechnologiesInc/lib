import { createHmac, timingSafeEqual } from 'crypto';
import { WebhookPayload } from './types';
import { PortalSDKError } from './errors';

/**
 * Verify the HMAC-SHA256 signature of a webhook payload.
 *
 * @param rawBody - The raw request body as a Buffer or string.
 * @param signature - The value of the `X-Portal-Signature` header (hex digest).
 * @param secret - The shared webhook secret.
 * @returns `true` if valid.
 * @throws PortalSDKError with code `SIGNATURE_INVALID` if verification fails.
 */
export function verifyWebhookSignature(
  rawBody: Buffer | string,
  signature: string,
  secret: string
): true {
  const expected = createHmac('sha256', secret)
    .update(rawBody)
    .digest('hex');

  const sigBuf = Buffer.from(signature, 'hex');
  const expBuf = Buffer.from(expected, 'hex');

  if (sigBuf.length !== expBuf.length || !timingSafeEqual(sigBuf, expBuf)) {
    throw new PortalSDKError(
      'Invalid webhook signature',
      'SIGNATURE_INVALID'
    );
  }

  return true;
}

/**
 * Parse and verify a webhook payload from raw body + signature.
 *
 * @param rawBody - The raw request body (Buffer or string).
 * @param signature - The `X-Portal-Signature` header value.
 * @param secret - The shared webhook secret.
 * @returns The parsed WebhookPayload.
 */
export function constructWebhookEvent(
  rawBody: Buffer | string,
  signature: string,
  secret: string
): WebhookPayload {
  verifyWebhookSignature(rawBody, signature, secret);

  const bodyStr = typeof rawBody === 'string' ? rawBody : rawBody.toString('utf-8');
  try {
    return JSON.parse(bodyStr) as WebhookPayload;
  } catch {
    throw new PortalSDKError('Failed to parse webhook body as JSON', 'PARSE_ERROR');
  }
}
