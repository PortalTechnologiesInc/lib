import { IncomingMessage, ServerResponse } from 'http';
import {
  ClientConfig,
  ApiResponse,
  PollOptions,
  AsyncOperation,
  KeyHandshakeUrlResponse,
  KeyHandshakeResult,
  AuthResponseData,
  AuthResponseStatus,
  SinglePaymentRequestContent,
  SinglePaymentResponse,
  InvoicePaymentRequestContent,
  RecurringPaymentRequestContent,
  RecurringPaymentStatus,
  RecurringPaymentResponseContent,
  CloseRecurringPaymentRequest,
  Profile,
  RequestInvoiceParams,
  InvoicePaymentResponse,
  PayInvoiceResponse,
  IssueJwtResponse,
  VerifyJwtResponse,
  CashuResponseStatus,
  WalletInfoResponse,
  VersionResponse,
  Nip05Profile,
  StreamEvent,
  EventsResponse,
  WebhookPayload,
  Timestamp,
} from './types';
import { PortalSDKError } from './errors';
import { verifyWebhookSignature } from './webhook';

// ---- Internal helpers ----

/** Terminal event statuses for payment streams. */
const TERMINAL_PAYMENT_STATUSES = new Set([
  'paid', 'timeout', 'error', 'user_success', 'user_failed', 'user_rejected',
]);

/**
 * Checks whether a stream event represents a terminal state
 * (i.e. no more events are expected for this stream).
 */
function isTerminalEvent(event: StreamEvent): boolean {
  switch (event.type) {
    case 'key_handshake':
    case 'authenticate_key':
    case 'recurring_payment_response':
    case 'invoice_response':
    case 'cashu_response':
    case 'error':
      return true;
    case 'payment_status_update': {
      const status = (event as Record<string, unknown>).status as { status: string } | undefined;
      return status != null && TERMINAL_PAYMENT_STATUSES.has(status.status);
    }
    default:
      return false;
  }
}

interface PendingStream {
  resolve: (event: StreamEvent) => void;
  reject: (err: Error) => void;
  listeners: Array<(event: StreamEvent) => void>;
}

// ---- Client ----

/**
 * Portal REST API client (server-side, Node.js 18+).
 *
 * Primary async notification path: **webhooks**.
 * Mount `webhookHandler()` in your HTTP server to receive events.
 * A `poll()` fallback is available for environments without inbound HTTP.
 */
export class PortalClient {
  private baseUrl: string;
  private authToken?: string;
  private webhookSecret?: string;
  private debugEnabled: boolean;

  /** Pending async operations keyed by stream_id. */
  private pending = new Map<string, PendingStream>();

  constructor(config: ClientConfig) {
    this.baseUrl = config.baseUrl.replace(/\/+$/, '');
    this.authToken = config.authToken;
    this.webhookSecret = config.webhookSecret;
    this.debugEnabled = config.debug ?? false;
  }

  /** Update the auth token after construction. */
  public setAuthToken(token: string): void {
    this.authToken = token;
  }

  private debug(message: string, data?: unknown): void {
    if (!this.debugEnabled) return;
    if (data !== undefined) {
      // eslint-disable-next-line no-console
      console.debug(`[PortalClient] ${message}`, data);
    } else {
      // eslint-disable-next-line no-console
      console.debug(`[PortalClient] ${message}`);
    }
  }

  // ========================================================================
  // HTTP helpers
  // ========================================================================

  private headers(hasBody: boolean): Record<string, string> {
    const h: Record<string, string> = {};
    if (this.authToken) {
      h['Authorization'] = `Bearer ${this.authToken}`;
    }
    if (hasBody) {
      h['Content-Type'] = 'application/json';
    }
    return h;
  }

  private async request<T>(method: string, path: string, body?: unknown): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    this.debug(`${method} ${url}`, body);

    let res: Response;
    try {
      res = await fetch(url, {
        method,
        headers: this.headers(body !== undefined),
        body: body !== undefined ? JSON.stringify(body) : undefined,
      });
    } catch (err) {
      throw new PortalSDKError(
        `Network error: ${err instanceof Error ? err.message : String(err)}`,
        'NETWORK_ERROR',
        err
      );
    }

    // Text-only endpoints (e.g. /health)
    const contentType = res.headers.get('content-type') ?? '';
    if (!contentType.includes('application/json')) {
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new PortalSDKError(`HTTP ${res.status}: ${text}`, 'HTTP_ERROR', undefined, res.status);
      }
      return (await res.text()) as unknown as T;
    }

    let json: ApiResponse<T>;
    try {
      json = await res.json() as ApiResponse<T>;
    } catch {
      throw new PortalSDKError(
        `Failed to parse JSON response (HTTP ${res.status})`,
        'PARSE_ERROR',
        undefined,
        res.status
      );
    }

    this.debug('response', json);

    if (!res.ok || !json.success) {
      throw new PortalSDKError(
        json.error ?? `HTTP ${res.status}`,
        'API_ERROR',
        json,
        res.status
      );
    }

    return json.data as T;
  }

  private get<T>(path: string): Promise<T> {
    return this.request<T>('GET', path);
  }

  private post<T>(path: string, body?: unknown): Promise<T> {
    return this.request<T>('POST', path, body ?? {});
  }

  private put<T>(path: string, body: unknown): Promise<T> {
    return this.request<T>('PUT', path, body);
  }

  private del<T>(path: string, body: unknown): Promise<T> {
    return this.request<T>('DELETE', path, body);
  }

  // ========================================================================
  // Pending stream management
  // ========================================================================

  /**
   * Register a stream and return a Promise that resolves on terminal webhook event.
   * The returned `done` promise will reject if `destroy(streamId)` is called.
   */
  private registerStream(streamId: string): Promise<StreamEvent> {
    return new Promise<StreamEvent>((resolve, reject) => {
      this.pending.set(streamId, { resolve, reject, listeners: [] });
    });
  }

  /**
   * Deliver a webhook event to the pending stream (called by the webhook handler).
   */
  private deliverEvent(streamId: string, event: StreamEvent): void {
    const entry = this.pending.get(streamId);
    if (!entry) {
      this.debug(`No pending stream for ${streamId}, ignoring event`);
      return;
    }

    // Notify intermediate listeners
    for (const listener of entry.listeners) {
      try { listener(event); } catch { /* listener errors don't propagate */ }
    }

    // Resolve on terminal event
    if (isTerminalEvent(event)) {
      this.pending.delete(streamId);
      entry.resolve(event);
    }
  }

  /**
   * Register a callback for **every** event on a stream (including non-terminal ones).
   * Useful for tracking intermediate status changes (e.g. `user_approved` before `paid`).
   *
   * @returns A function to unsubscribe.
   */
  public onEvent(streamId: string, callback: (event: StreamEvent) => void): () => void {
    const entry = this.pending.get(streamId);
    if (!entry) {
      this.debug(`onEvent: no pending stream for ${streamId}`);
      return () => { /* noop */ };
    }
    entry.listeners.push(callback);
    return () => {
      const idx = entry.listeners.indexOf(callback);
      if (idx !== -1) entry.listeners.splice(idx, 1);
    };
  }

  /**
   * Cancel a pending stream. The `done` promise will reject with the given reason.
   */
  public destroy(streamId: string, reason = 'Stream destroyed by caller'): void {
    const entry = this.pending.get(streamId);
    if (entry) {
      this.pending.delete(streamId);
      entry.reject(new PortalSDKError(reason, 'API_ERROR'));
    }
  }

  /** Number of currently pending streams (useful for diagnostics). */
  public get pendingCount(): number {
    return this.pending.size;
  }

  // ========================================================================
  // Webhook handler
  // ========================================================================

  /**
   * Returns an HTTP request handler to mount in your server (Express, Koa, raw http, etc.).
   *
   * The handler reads the raw body from the request stream, verifies the
   * `X-Portal-Signature` HMAC-SHA256 signature, parses the JSON payload,
   * and routes the event to the matching pending stream.
   *
   * **Important:** Do NOT use `express.json()` or any body-parsing middleware
   * on the webhook route — the handler needs the raw body for signature verification.
   *
   * @example
   * ```ts
   * // Express
   * app.post('/portal/webhook', portal.webhookHandler());
   *
   * // Raw http
   * http.createServer((req, res) => {
   *   if (req.url === '/portal/webhook' && req.method === 'POST') {
   *     return portal.webhookHandler()(req, res);
   *   }
   * });
   * ```
   */
  public webhookHandler(): (req: IncomingMessage, res: ServerResponse) => void {
    const secret = this.webhookSecret;
    if (!secret) {
      throw new PortalSDKError(
        'Cannot create webhook handler without webhookSecret in client config',
        'API_ERROR'
      );
    }

    return (req: IncomingMessage, res: ServerResponse) => {
      const chunks: Buffer[] = [];

      req.on('data', (chunk: Buffer) => chunks.push(chunk));
      req.on('end', () => {
        const rawBody = Buffer.concat(chunks);
        const signature = req.headers['x-portal-signature'] as string | undefined;

        if (!signature) {
          res.writeHead(401, { 'Content-Type': 'text/plain' });
          res.end('Missing X-Portal-Signature header');
          return;
        }

        try {
          verifyWebhookSignature(rawBody, signature, secret);
        } catch {
          res.writeHead(401, { 'Content-Type': 'text/plain' });
          res.end('Invalid signature');
          return;
        }

        let payload: WebhookPayload;
        try {
          payload = JSON.parse(rawBody.toString('utf-8')) as WebhookPayload;
        } catch {
          res.writeHead(400, { 'Content-Type': 'text/plain' });
          res.end('Invalid JSON');
          return;
        }

        this.debug('webhook received', payload);
        this.deliverEvent(payload.stream_id, payload);

        res.writeHead(200, { 'Content-Type': 'text/plain' });
        res.end('OK');
      });

      req.on('error', () => {
        res.writeHead(500, { 'Content-Type': 'text/plain' });
        res.end('Internal error');
      });
    };
  }

  // ========================================================================
  // Polling fallback
  // ========================================================================

  /**
   * Poll `GET /events/{stream_id}` until a terminal event arrives.
   * Use this as a **fallback** when webhooks are not available.
   *
   * If a pending stream is registered (from an async operation), the terminal
   * event also resolves that pending promise.
   */
  public async poll(streamId: string, options?: PollOptions): Promise<StreamEvent> {
    const intervalMs = options?.intervalMs ?? 1000;
    const timeoutMs = options?.timeoutMs;
    const onEvent = options?.onEvent;
    const startTime = Date.now();
    let lastIndex: number | undefined;

    // eslint-disable-next-line no-constant-condition
    while (true) {
      if (timeoutMs !== undefined && Date.now() - startTime > timeoutMs) {
        throw new PortalSDKError('Polling timed out', 'POLL_TIMEOUT');
      }

      const response = await this.getEvents(streamId, lastIndex);

      for (const event of response.events) {
        lastIndex = event.index;
        onEvent?.(event);
        // Also deliver to pending stream if registered
        this.deliverEvent(streamId, event);
        if (isTerminalEvent(event)) {
          return event;
        }
      }

      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }

  // ========================================================================
  // REST endpoints
  // ========================================================================

  // ---- Health / Version ----

  /** Health check (returns "OK"). No auth required. */
  public async health(): Promise<string> {
    return this.get<string>('/health');
  }

  /** Get server version and git commit. No auth required. */
  public async version(): Promise<VersionResponse> {
    return this.get<VersionResponse>('/version');
  }

  // ---- Key Handshake ----

  /**
   * Create a new key handshake URL.
   *
   * Returns `{ url, streamId, done }`. The `url` should be shown to the user
   * (e.g. as QR code). `done` resolves when the handshake completes via webhook.
   */
  public async newKeyHandshakeUrl(
    staticToken: string | null = null,
    noRequest: boolean | null = false
  ): Promise<{ url: string } & AsyncOperation<KeyHandshakeResult>> {
    const resp = await this.post<KeyHandshakeUrlResponse>('/key-handshake', {
      static_token: staticToken,
      no_request: noRequest,
    });

    const rawDone = this.registerStream(resp.stream_id);
    const done = rawDone.then((event) => ({
      main_key: event.main_key as string,
      preferred_relays: event.preferred_relays as string[],
    }));

    return { url: resp.url, streamId: resp.stream_id, done };
  }

  /** Authenticate a key (NIP-46 style). Returns an async operation. */
  public async authenticateKey(
    mainKey: string,
    subkeys: string[] = []
  ): Promise<AsyncOperation<AuthResponseData>> {
    const resp = await this.post<{ stream_id: string }>('/authenticate-key', {
      main_key: mainKey,
      subkeys,
    });
    const done = this.registerStream(resp.stream_id).then((event) => ({
      user_key: event.user_key as string,
      recipient: event.recipient as string,
      challenge: event.challenge as string,
      status: event.status as AuthResponseStatus,
    }));
    return { streamId: resp.stream_id, done };
  }

  // ---- Payments ----

  /**
   * Request a single payment.
   *
   * Returns `{ streamId, done }`. `done` resolves when the payment reaches a
   * terminal state (paid, failed, rejected, timeout) via webhook.
   * Use `onEvent(streamId, cb)` to track intermediate states (e.g. `user_approved`).
   */
  public async requestSinglePayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: SinglePaymentRequestContent
  ): Promise<AsyncOperation> {
    const resp = await this.post<SinglePaymentResponse>('/payments/single', {
      main_key: mainKey,
      subkeys,
      payment_request: paymentRequest,
    });
    const done = this.registerStream(resp.stream_id);
    return { streamId: resp.stream_id, done };
  }

  /**
   * Request a payment with raw SinglePaymentRequestContent.
   * Returns `{ streamId, done }`.
   */
  public async requestPaymentRaw(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: InvoicePaymentRequestContent
  ): Promise<AsyncOperation> {
    const resp = await this.post<SinglePaymentResponse>('/payments/raw', {
      main_key: mainKey,
      subkeys,
      payment_request: paymentRequest,
    });
    const done = this.registerStream(resp.stream_id);
    return { streamId: resp.stream_id, done };
  }

  /** Request a recurring payment. Returns an async operation. */
  public async requestRecurringPayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: RecurringPaymentRequestContent
  ): Promise<AsyncOperation<RecurringPaymentResponseContent>> {
    const resp = await this.post<{ stream_id: string }>('/payments/recurring', {
      main_key: mainKey,
      subkeys,
      payment_request: paymentRequest,
    });
    const done = this.registerStream(resp.stream_id).then((event) => ({
      request_id: event.request_id as string,
      status: event.status as RecurringPaymentStatus,
    }));
    return { streamId: resp.stream_id, done };
  }

  /** Close a recurring payment subscription. */
  public async closeRecurringPayment(mainKey: string, subkeys: string[], subscriptionId: string): Promise<string> {
    const response = await this.post<{ message: string }>('/payments/recurring/close', {
      main_key: mainKey,
      subkeys,
      subscription_id: subscriptionId,
    } as CloseRecurringPaymentRequest);
    return response.message;
  }



  // ---- Profile ----

  /** Fetch a user profile by public key. */
  public async fetchProfile(mainKey: string): Promise<Profile | null> {
    const response = await this.get<{ profile: Profile | null }>(`/profile/${encodeURIComponent(mainKey)}`);
    return response.profile;
  }

  // ---- Invoices ----

  /** Request an invoice from a recipient. Returns an async operation. */
  public async requestInvoice(
    recipientKey: string,
    subkeys: string[],
    content: RequestInvoiceParams
  ): Promise<AsyncOperation<InvoicePaymentResponse>> {
    const resp = await this.post<{ stream_id: string }>('/invoices/request', {
      recipient_key: recipientKey,
      subkeys,
      content,
    });
    const done = this.registerStream(resp.stream_id).then((event) => ({
      invoice: event.invoice as string,
      payment_hash: (event.payment_hash as string) ?? null,
    }));
    return { streamId: resp.stream_id, done };
  }

  /** Pay a BOLT11 invoice. Returns preimage and fees paid. */
  public async payInvoice(invoice: string): Promise<PayInvoiceResponse> {
    return this.post<PayInvoiceResponse>('/invoices/pay', { invoice });
  }

  // ---- JWT ----

  /** Issue a JWT for the given target key. */
  public async issueJwt(targetKey: string, durationHours: number): Promise<string> {
    const response = await this.post<IssueJwtResponse>('/jwt/issue', {
      target_key: targetKey,
      duration_hours: durationHours,
    });
    return response.token;
  }

  /** Verify a JWT and return claims. */
  public async verifyJwt(pubkey: string, token: string): Promise<{ target_key: string }> {
    const response = await this.post<VerifyJwtResponse>('/jwt/verify', { pubkey, token });
    return { target_key: response.target_key };
  }

  // ---- Cashu ----

  /** Request Cashu tokens from a recipient. Returns an async operation. */
  public async requestCashu(
    recipientKey: string,
    subkeys: string[],
    mintUrl: string,
    unit: string,
    amount: number
  ): Promise<AsyncOperation<CashuResponseStatus>> {
    const resp = await this.post<{ stream_id: string }>('/cashu/request', {
      recipient_key: recipientKey,
      subkeys,
      mint_url: mintUrl,
      unit,
      amount,
    });
    const done = this.registerStream(resp.stream_id).then((event) =>
      event.status as CashuResponseStatus
    );
    return { streamId: resp.stream_id, done };
  }

  /** Send Cashu tokens directly to a recipient. */
  public async sendCashuDirect(mainKey: string, subkeys: string[], token: string): Promise<string> {
    const response = await this.post<{ message: string }>('/cashu/send-direct', {
      main_key: mainKey,
      subkeys,
      token,
    });
    return response.message;
  }

  /** Mint Cashu tokens from a mint. */
  public async mintCashu(
    mintUrl: string,
    unit: string,
    amount: number,
    staticAuthToken?: string,
    description?: string
  ): Promise<string> {
    const response = await this.post<{ token: string }>('/cashu/mint', {
      mint_url: mintUrl,
      unit,
      amount,
      static_auth_token: staticAuthToken ?? null,
      description: description ?? null,
    });
    return response.token;
  }

  /** Burn (receive) a Cashu token at a mint. */
  public async burnCashu(mintUrl: string, unit: string, token: string, staticAuthToken?: string): Promise<number> {
    const response = await this.post<{ amount: number }>('/cashu/burn', {
      mint_url: mintUrl,
      unit,
      token,
      static_auth_token: staticAuthToken ?? null,
    });
    return response.amount;
  }

  // ---- Relays ----

  /** Add a relay to the pool. */
  public async addRelay(relay: string): Promise<string> {
    const response = await this.post<{ relay: string }>('/relays', { relay });
    return response.relay;
  }

  /** Remove a relay from the pool. */
  public async removeRelay(relay: string): Promise<string> {
    const response = await this.del<{ relay: string }>('/relays', { relay });
    return response.relay;
  }

  // ---- Calendar ----

  /** Calculate next occurrence for a calendar (e.g. "daily", "monthly"). */
  public async calculateNextOccurrence(calendar: string, from: Timestamp): Promise<Timestamp | null> {
    const response = await this.post<{ next_occurrence: string | number | null }>('/calendar/next-occurrence', {
      calendar,
      from,
    });
    const next = response.next_occurrence;
    return next != null ? new Timestamp(typeof next === 'string' ? BigInt(next) : next) : null;
  }

  // ---- NIP-05 ----

  /** Fetch NIP-05 profile (e.g. user@domain.com). */
  public async fetchNip05Profile(nip05: string): Promise<Nip05Profile> {
    const response = await this.get<{ profile: Nip05Profile }>(`/nip05/${encodeURIComponent(nip05)}`);
    return response.profile;
  }

  // ---- Wallet ----

  /** Get wallet type and balance (msat). */
  public async getWalletInfo(): Promise<WalletInfoResponse> {
    return this.get<WalletInfoResponse>('/wallet/info');
  }

  // ---- Events (low-level) ----

  /**
   * Fetch events for a stream. Pass `after` to get only events with index > after.
   * Prefer `webhookHandler()` or `poll()` over calling this directly.
   */
  public async getEvents(streamId: string, after?: number): Promise<EventsResponse> {
    const q = after !== undefined ? `?after=${after}` : '';
    return this.get<EventsResponse>(`/events/${encodeURIComponent(streamId)}${q}`);
  }
}
