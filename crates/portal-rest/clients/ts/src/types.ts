/** Types for the Portal WebSocket API (commands, responses, domain models). */

// Payment
export enum Currency {
  Millisats = "Millisats",
}

// Custom Timestamp type that serializes to string
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

export interface RecurrenceInfo {
  until?: Timestamp;
  calendar: string;
  max_payments?: number;
  first_payment_due: Timestamp;
}

export interface RecurringPaymentRequestContent {
  amount: number;
  currency: Currency;
  recurrence: RecurrenceInfo;
  description?: string;
  current_exchange_rate?: any;
  expires_at: Timestamp;
  auth_token?: string;
}

export interface InvoicePaymentRequestContent {
  amount: number;
  currency: Currency;
  description: string;
  subscription_id?: string;
  auth_token?: string;
  current_exchange_rate?: any;
  expires_at?: Timestamp;
  invoice?: string;
}

export interface SinglePaymentRequestContent {
  description: string;
  amount: number;
  currency: Currency;
  subscription_id?: string;
  auth_token?: string;
}

/** Confirmed variant of recurring payment status (server tag: "confirmed") */
export interface RecurringPaymentStatusConfirmed {
  status: 'confirmed';
  subscription_id: string;
  authorized_amount: number;
  authorized_currency: Currency;
  authorized_recurrence: RecurrenceInfo;
}

/** Rejected variant of recurring payment status (server tag: "rejected") */
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

export interface InvoiceStatus {
  status: 'paid' | 'timeout' | 'error' | 'user_approved' | 'user_success' | 'user_failed' | 'user_rejected';
  preimage?: string;
  reason?: string;
}

// Auth related types
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

// Profile related types
export interface Profile {
  id: string;
  pubkey: string;
  name?: string;
  display_name?: string;
  picture?: string;
  about?: string;
  nip05?: string;
}

// Invoice related types
export interface InvoiceRequestContent {
  request_id: string;
  amount: number;
  currency: Currency;
  current_exchange_rate?: ExchangeRate;
  expires_at: Timestamp;
  description?: string;
  refund_invoice?: string;
}

export interface InvoiceResponseContent {
  invoice: string;
  payment_hash: string | null;
}

export interface ExchangeRate {
  rate: number;
  source: string;
  time: Timestamp; 
}

// JWT related types
export interface JwtClaims {
  target_key: string;
  exp: number;
}

// Command/Request types (must match server command.rs)
export type Command =
  | { cmd: 'Auth'; params: { token: string } }
  | { cmd: 'NewKeyHandshakeUrl'; params?: { static_token?: string | null; no_request?: boolean | null } }
  | { cmd: 'AuthenticateKey'; params: { main_key: string; subkeys: string[] } }
  | { cmd: 'RequestRecurringPayment'; params: { main_key: string; subkeys: string[]; payment_request: RecurringPaymentRequestContent } }
  | { cmd: 'RequestSinglePayment'; params: { main_key: string; subkeys: string[]; payment_request: SinglePaymentRequestContent } }
  | { cmd: 'RequestPaymentRaw'; params: { main_key: string; subkeys: string[]; payment_request: SinglePaymentRequestContent } }
  | { cmd: 'FetchProfile'; params: { main_key: string } }
  | { cmd: 'SetProfile'; params: { profile: Profile } }
  | { cmd: 'CloseRecurringPayment'; params: { main_key: string; subkeys: string[]; subscription_id: string } }
  | { cmd: 'ListenClosedRecurringPayment' }
  | { cmd: 'RequestInvoice'; params: { recipient_key: string; subkeys: string[]; content: InvoiceRequestContent } }
  | { cmd: 'IssueJwt'; params: { target_key: string; duration_hours: number } }
  | { cmd: 'VerifyJwt'; params: { pubkey: string; token: string } }
  | { cmd: 'RequestCashu'; params: { recipient_key: string; subkeys: string[]; mint_url: string; unit: string; amount: number } }
  | { cmd: 'SendCashuDirect'; params: { main_key: string; subkeys: string[]; token: string } }
  | { cmd: 'MintCashu'; params: { mint_url: string; unit: string; amount: number; static_auth_token?: string | null; description?: string | null } }
  | { cmd: 'BurnCashu'; params: { mint_url: string; unit: string; token: string; static_auth_token?: string | null } }
  | { cmd: 'AddRelay'; params: { relay: string } }
  | { cmd: 'RemoveRelay'; params: { relay: string } }
  | { cmd: 'CalculateNextOccurrence'; params: { calendar: string; from: Timestamp } }
  | { cmd: 'FetchNip05Profile'; params: { nip05: string } }
  ;

// Response types (must match server response.rs)
export type ResponseData =
  | { type: 'auth_success'; message: string }
  | { type: 'key_handshake_url'; url: string; stream_id: string }
  | { type: 'auth_response'; event: AuthResponseData }
  | { type: 'recurring_payment'; status: RecurringPaymentResponseContent }
  | { type: 'single_payment'; stream_id: string }
  | { type: 'profile'; profile: Profile | null }
  | { type: 'close_recurring_payment_success'; message: string }
  | { type: 'listen_closed_recurring_payment'; stream_id: string }
  | { type: 'invoice_payment'; invoice: string; payment_hash: string | null }
  | { type: 'issue_jwt'; token: string }
  | { type: 'verify_jwt'; target_key: string }
  | { type: 'cashu_response'; status: CashuResponseStatus }
  | { type: 'send_cashu_direct_success'; message: string }
  | { type: 'cashu_mint'; token: string }
  | { type: 'cashu_burn'; amount: number }
  | { type: 'add_relay'; relay: string }
  | { type: 'remove_relay'; relay: string }
  | { type: 'calculate_next_occurrence'; next_occurrence: string | number | null }
  | { type: 'fetch_nip05_profile'; profile: Nip05Profile }
  ;

/** NIP-05 profile (matches nostr Nip05Profile serialization) */
export interface Nip05Profile {
  pubkey: string;
  relays?: string[];
}

export type Response =
  | { type: 'error'; id: string; message: string }
  | { type: 'success'; id: string; data: ResponseData }
  | { type: 'notification'; id: string; data: NotificationData };

/** Type guard: response is success with given data type */
export function isResponseType<T extends ResponseData>(
  data: ResponseData,
  type: T['type']
): data is T {
  return data.type === type;
}

// Notification data types
export type NotificationData = 
  | { type: 'key_handshake', main_key: string, preferred_relays: string[] }
  | { type: 'payment_status_update', status: InvoiceStatus }
  | { type: 'closed_recurring_payment', reason: string | null, subscription_id: string, main_key: string, recipient: string }
  | { type: 'cashu_request', request: CashuRequestContentWithKey }
  ;

export type CloseRecurringPaymentNotification = {
  reason: string | null;
  subscription_id: string;
  main_key: string;
  recipient: string;
}

// Events 
export interface EventCallbacks {
  onKeyHandshake?: (mainKey: string) => void;
  onError?: (error: Error) => void;
  onConnected?: () => void;
  onDisconnected?: () => void;
}

// Client configuration
export interface ClientConfig {
  /** WebSocket server URL (e.g. ws://localhost:3000/ws) */
  serverUrl: string;
  /** Connection timeout in ms. Default 10000 */
  connectTimeout?: number;
  /** Enable debug logging to console. Default false in production */
  debug?: boolean;
}

export interface Event {
  type: string;
  data: any;
}

export interface PaymentRequest {
  pr: string;
  hash: string;
  amount: number;
  description: string;
  status: string;
  expiry: number;
}

export interface KeyHandshakeUrlResponse {
  url: string;
  stream_id: string;
} 

export interface CashuRequestContent {
  request_id: string;
  mint_url: string;
  unit: string;
  amount: number;
}

export interface CashuRequestContentWithKey {
  inner: CashuRequestContent;
  main_key: string;
  recipient: string;
}

export interface CashuResponseContent {
  request: CashuRequestContentWithKey;
  token: string;
}

export interface CashuDirectContent {
  token: string;
} 

export type CashuResponseStatus =
  | { status: 'success', token: string }
  | { status: 'insufficient_funds' }
  | { status: 'rejected', reason?: string }; 