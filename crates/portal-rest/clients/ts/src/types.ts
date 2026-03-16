/** Types for the Portal REST API (requests, responses, domain models). */

// ---- Currency ----

export enum Currency {
  Millisats = "Millisats",
}

/** Currency for payment requests: Millisats or a fiat code string (e.g. "EUR", "USD"). */
export type PaymentCurrency = Currency | string;

// ---- Timestamp ----

/** Custom Timestamp type that serializes to string (unix seconds). */
export class Timestamp {
  private value: bigint;

  constructor(value: bigint | number) {
    this.value = BigInt(value);
  }

  static fromDate(date: Date): Timestamp {
    return new Timestamp(Math.floor(date.getTime() / 1000));
  }

  static fromNow(seconds: number): Timestamp {
    return new Timestamp(Math.floor(Date.now() / 1000) + seconds);
  }

  toDate(): Date {
    return new Date(Number(this.value) * 1000);
  }

  toJSON(): string {
    return this.value.toString();
  }

  toString(): string {
    return this.value.toString();
  }

  valueOf(): bigint {
    return this.value;
  }
}

// ---- Client configuration ----

export interface ClientConfig {
  /** Base URL of the Portal REST API (e.g. http://localhost:3000) */
  baseUrl: string;
  /** Bearer token for authentication. */
  authToken?: string;
  /** Enable debug logging to console. Default false */
  debug?: boolean;
}

// ---- Generic API response ----

export interface ApiResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}

// ---- Polling options ----

export interface PollOptions {
  /** Polling interval in milliseconds. Default 1000. */
  intervalMs?: number;
  /** Maximum time to poll before giving up, in milliseconds. Default: no timeout. */
  timeoutMs?: number;
}

// ---- Key Handshake ----

export interface KeyHandshakeRequest {
  static_token?: string | null;
  no_request?: boolean | null;
}

export interface KeyHandshakeUrlResponse {
  url: string;
  stream_id: string;
}

// ---- Auth ----

export interface AuthenticateKeyRequest {
  main_key: string;
  subkeys: string[];
}

export interface AuthResponseStatus {
  status: 'approved' | 'declined';
  reason?: string;
  granted_permissions?: string[];
  session_token?: string;
}

export interface AuthResponseData {
  user_key: string;
  recipient: string;
  challenge: string;
  status: AuthResponseStatus;
}

export interface AuthKeyResponse {
  event: AuthResponseData;
}

// ---- Payments ----

export interface RecurrenceInfo {
  until?: Timestamp;
  calendar: string;
  max_payments?: number;
  first_payment_due: Timestamp;
}

export interface SinglePaymentRequestContent {
  description: string;
  amount: number;
  currency: PaymentCurrency;
  subscription_id?: string;
  auth_token?: string;
  /** Optional client-provided id for correlating this payment request. */
  request_id?: string;
}

export interface SinglePaymentResponse {
  stream_id: string;
}

export interface RecurringPaymentRequestContent {
  amount: number;
  currency: PaymentCurrency;
  recurrence: RecurrenceInfo;
  description?: string;
  current_exchange_rate?: unknown;
  expires_at: Timestamp;
  auth_token?: string;
}

/** Confirmed variant of recurring payment status */
export interface RecurringPaymentStatusConfirmed {
  status: 'confirmed';
  subscription_id: string;
  authorized_amount: number;
  authorized_currency: Currency;
  authorized_recurrence: RecurrenceInfo;
}

/** Rejected variant of recurring payment status */
export interface RecurringPaymentStatusRejected {
  status: 'rejected';
  reason?: string;
}

export type RecurringPaymentStatus = RecurringPaymentStatusConfirmed | RecurringPaymentStatusRejected;

/** @deprecated Use RecurringPaymentStatusConfirmed for the confirmed case */
export type RecurringPaymentStatusContent = RecurringPaymentStatusConfirmed;

export interface RecurringPaymentResponseContent {
  request_id: string;
  status: RecurringPaymentStatus;
}

export interface InvoicePaymentRequestContent {
  amount: number;
  currency: PaymentCurrency;
  description: string;
  subscription_id?: string;
  auth_token?: string;
  current_exchange_rate?: unknown;
  expires_at?: Timestamp;
  invoice?: string;
}

export interface CloseRecurringPaymentRequest {
  main_key: string;
  subkeys: string[];
  subscription_id: string;
}

// ---- Invoice Status ----

export interface InvoiceStatus {
  status: 'paid' | 'timeout' | 'error' | 'user_approved' | 'user_success' | 'user_failed' | 'user_rejected';
  preimage?: string;
  reason?: string;
}

// ---- Profile ----

export interface Profile {
  id: string;
  pubkey: string;
  name?: string;
  display_name?: string;
  picture?: string;
  about?: string;
  nip05?: string;
}

export interface SetProfileRequest {
  profile: Profile;
}

// ---- Invoice ----

export interface RequestInvoiceParams {
  amount: number;
  currency: PaymentCurrency;
  expires_at: Timestamp;
  description?: string | null;
  refund_invoice?: string | null;
  /** Optional request ID. If not provided, a UUID is generated server-side. */
  request_id?: string | null;
}

export interface InvoicePaymentResponse {
  invoice: string;
  payment_hash: string | null;
}

export interface PayInvoiceRequest {
  invoice: string;
}

export interface PayInvoiceResponse {
  preimage: string;
  fees_paid_msat: number;
}

// ---- JWT ----

export interface IssueJwtRequest {
  target_key: string;
  duration_hours: number;
}

export interface IssueJwtResponse {
  token: string;
}

export interface VerifyJwtRequest {
  pubkey: string;
  token: string;
}

export interface VerifyJwtResponse {
  target_key: string;
}

// ---- Cashu ----

export interface RequestCashuRequest {
  recipient_key: string;
  subkeys: string[];
  mint_url: string;
  unit: string;
  amount: number;
}

export interface SendCashuDirectRequest {
  main_key: string;
  subkeys: string[];
  token: string;
}

export interface MintCashuRequest {
  mint_url: string;
  unit: string;
  static_auth_token?: string | null;
  amount: number;
  description?: string | null;
}

export interface BurnCashuRequest {
  mint_url: string;
  unit: string;
  static_auth_token?: string | null;
  token: string;
}

export type CashuResponseStatus =
  | { status: 'success'; token: string }
  | { status: 'insufficient_funds' }
  | { status: 'rejected'; reason?: string };

// ---- Relays ----

export interface RelayRequest {
  relay: string;
}

// ---- Calendar ----

export interface CalculateNextOccurrenceRequest {
  calendar: string;
  from: Timestamp;
}

// ---- NIP-05 ----

export interface Nip05Profile {
  public_key: string;
  relays?: string[];
}

// ---- Wallet ----

export interface WalletInfoResponse {
  wallet_type: string;
  balance_msat: number;
}

// ---- Version ----

export interface VersionResponse {
  version: string;
  git_commit: string;
}

// ---- Events / Streaming ----

export interface StreamEvent {
  /** Monotonically increasing index within this stream. */
  index: number;
  /** ISO-8601 timestamp of when the event was created. */
  timestamp: string;
  /** Event type discriminator. */
  type: string;
  /** Additional event-specific fields (flattened). */
  [key: string]: unknown;
}

export interface EventsResponse {
  stream_id: string;
  events: StreamEvent[];
}

// ---- Notification data (event variants) ----

export type NotificationData =
  | { type: 'key_handshake'; main_key: string; preferred_relays: string[] }
  | { type: 'payment_status_update'; status: InvoiceStatus }
  | { type: 'closed_recurring_payment'; reason: string | null; subscription_id: string; main_key: string; recipient: string };

export type CloseRecurringPaymentNotification = {
  reason: string | null;
  subscription_id: string;
  main_key: string;
  recipient: string;
};

// ---- Webhook payload ----

/**
 * Webhook POST body shape. Same as StreamEvent but includes stream_id.
 * The server signs this with HMAC-SHA256 (header: X-Portal-Signature)
 * if a webhook_secret is configured.
 */
export interface WebhookPayload extends StreamEvent {
  stream_id: string;
}
