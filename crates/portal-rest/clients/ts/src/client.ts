import {
  ClientConfig,
  ApiResponse,
  PollOptions,
  KeyHandshakeUrlResponse,
  AuthKeyResponse,
  AuthResponseData,
  SinglePaymentRequestContent,
  SinglePaymentResponse,
  InvoicePaymentRequestContent,
  RecurringPaymentRequestContent,
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
  NotificationData,
  Timestamp,
} from './types';
import { PortalSDKError } from './errors';

/**
 * Checks whether a stream event represents a terminal state
 * (i.e. no more events are expected for this stream).
 */
function isTerminalEvent(event: StreamEvent): boolean {
  switch (event.type) {
    case 'key_handshake':
      return true;
    case 'payment_status_update': {
      const status = (event as Record<string, unknown>).status as { status: string } | undefined;
      if (!status) return false;
      return ['paid', 'timeout', 'error', 'user_success', 'user_failed', 'user_rejected'].includes(status.status);
    }
    case 'closed_recurring_payment':
      // Individual notifications are not terminal for the listener stream itself,
      // but callers can decide to stop. We treat them as non-terminal by default.
      return false;
    default:
      return false;
  }
}

/** Portal REST API client: auth, payments, profiles, JWT, Cashu, relays, polling. */
export class PortalSDK {
  private baseUrl: string;
  private authToken?: string;
  private debugEnabled: boolean;

  constructor(config: ClientConfig) {
    // Strip trailing slash
    this.baseUrl = config.baseUrl.replace(/\/+$/, '');
    this.authToken = config.authToken;
    this.debugEnabled = config.debug ?? false;
  }

  /** Update the auth token after construction. */
  public setAuthToken(token: string): void {
    this.authToken = token;
  }

  private debug(message: string, data?: unknown): void {
    if (this.debugEnabled) {
      if (data !== undefined) {
        // eslint-disable-next-line no-console
        console.debug(`[PortalSDK] ${message}`, data);
      } else {
        // eslint-disable-next-line no-console
        console.debug(`[PortalSDK] ${message}`);
      }
    }
  }

  // ---- HTTP helpers ----

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

    // For text-only endpoints (e.g. /health)
    const contentType = res.headers.get('content-type') ?? '';
    if (!contentType.includes('application/json')) {
      if (!res.ok) {
        const text = await res.text().catch(() => '');
        throw new PortalSDKError(`HTTP ${res.status}: ${text}`, 'HTTP_ERROR', undefined, res.status);
      }
      const text = await res.text();
      return text as unknown as T;
    }

    let json: ApiResponse<T>;
    try {
      json = await res.json() as ApiResponse<T>;
    } catch {
      throw new PortalSDKError(`Failed to parse JSON response (HTTP ${res.status})`, 'PARSE_ERROR', undefined, res.status);
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
   * Returns `{ url, stream_id }`. Poll the stream_id for handshake completion.
   */
  public async newKeyHandshakeUrl(
    staticToken: string | null = null,
    noRequest: boolean | null = false
  ): Promise<KeyHandshakeUrlResponse> {
    return this.post<KeyHandshakeUrlResponse>('/key-handshake', {
      static_token: staticToken,
      no_request: noRequest,
    });
  }

  /** Authenticate a key (NIP-46 style). */
  public async authenticateKey(mainKey: string, subkeys: string[] = []): Promise<AuthResponseData> {
    const response = await this.post<AuthKeyResponse>('/authenticate-key', {
      main_key: mainKey,
      subkeys,
    });
    return response.event;
  }

  // ---- Payments ----

  /**
   * Request a single payment. Returns `{ stream_id }`.
   * Poll the stream_id for payment status updates.
   */
  public async requestSinglePayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: SinglePaymentRequestContent
  ): Promise<SinglePaymentResponse> {
    return this.post<SinglePaymentResponse>('/payments/single', {
      main_key: mainKey,
      subkeys,
      payment_request: paymentRequest,
    });
  }

  /**
   * Request a payment with raw SinglePaymentRequestContent. Returns `{ stream_id }`.
   */
  public async requestPaymentRaw(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: InvoicePaymentRequestContent
  ): Promise<SinglePaymentResponse> {
    return this.post<SinglePaymentResponse>('/payments/raw', {
      main_key: mainKey,
      subkeys,
      payment_request: paymentRequest,
    });
  }

  /** Request a recurring payment. */
  public async requestRecurringPayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: RecurringPaymentRequestContent
  ): Promise<RecurringPaymentResponseContent> {
    const response = await this.post<{ status: RecurringPaymentResponseContent }>('/payments/recurring', {
      main_key: mainKey,
      subkeys,
      payment_request: paymentRequest,
    });
    return response.status;
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

  /**
   * Start listening for closed recurring payment events. Returns `{ stream_id }`.
   * Poll the stream_id for closed recurring payment notifications.
   */
  public async listenClosedRecurringPayment(): Promise<{ stream_id: string }> {
    return this.post<{ stream_id: string }>('/payments/recurring/listen');
  }

  // ---- Profile ----

  /** Fetch a user profile by public key. */
  public async fetchProfile(mainKey: string): Promise<Profile | null> {
    const response = await this.get<{ profile: Profile | null }>(`/profile/${encodeURIComponent(mainKey)}`);
    return response.profile;
  }

  /** Set the current user's profile. */
  public async setProfile(profile: Profile): Promise<void> {
    await this.put<unknown>('/profile', { profile });
  }

  // ---- Invoices ----

  /** Request an invoice from a recipient. */
  public async requestInvoice(
    recipientKey: string,
    subkeys: string[],
    content: RequestInvoiceParams
  ): Promise<InvoicePaymentResponse> {
    return this.post<InvoicePaymentResponse>('/invoices/request', {
      recipient_key: recipientKey,
      subkeys,
      content,
    });
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

  /** Request Cashu tokens from a recipient. */
  public async requestCashu(
    recipientKey: string,
    subkeys: string[],
    mintUrl: string,
    unit: string,
    amount: number
  ): Promise<CashuResponseStatus> {
    const response = await this.post<{ status: CashuResponseStatus }>('/cashu/request', {
      recipient_key: recipientKey,
      subkeys,
      mint_url: mintUrl,
      unit,
      amount,
    });
    return response.status;
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

  // ---- Events / Polling ----

  /**
   * Fetch events for a stream. Pass `after` to get only events with index > after.
   */
  public async getEvents(streamId: string, after?: number): Promise<EventsResponse> {
    const q = after !== undefined ? `?after=${after}` : '';
    return this.get<EventsResponse>(`/events/${encodeURIComponent(streamId)}${q}`);
  }

  /**
   * Poll an event stream until a terminal event is received (or timeout).
   *
   * @param streamId - The stream ID to poll.
   * @param onEvent - Called for each new event as it arrives.
   * @param options - `intervalMs` (default 1000), `timeoutMs` (default: no timeout).
   * @returns The terminal event that ended the stream.
   */
  public async pollUntilDone(
    streamId: string,
    onEvent?: (event: StreamEvent) => void,
    options?: PollOptions
  ): Promise<StreamEvent> {
    const intervalMs = options?.intervalMs ?? 1000;
    const timeoutMs = options?.timeoutMs;
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
        if (isTerminalEvent(event)) {
          return event;
        }
      }

      // Wait before next poll
      await new Promise((resolve) => setTimeout(resolve, intervalMs));
    }
  }
}
