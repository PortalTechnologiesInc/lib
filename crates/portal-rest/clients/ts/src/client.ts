import WebSocket from 'ws';
import {
  ClientConfig,
  Response,
  ResponseData,
  NotificationData,
  EventCallbacks,
  RecurringPaymentRequestContent,
  SinglePaymentRequestContent,
  Profile,
  AuthResponseData,
  Event,
  InvoicePaymentRequestContent,
  InvoiceResponseContent,
  RequestInvoiceParams,
  RecurringPaymentResponseContent,
  CloseRecurringPaymentNotification,
  InvoiceStatus,
  CashuResponseStatus,
  Timestamp,
  Nip05Profile,
  isResponseType,
} from './types';
import { PortalSDKError } from './errors';

type CommandCallback = { resolve: (value: unknown) => void; reject: (reason: PortalSDKError) => void };

/** Portal WebSocket API client: auth, payments, profiles, JWT, Cashu, relays. */
export class PortalSDK {
  private config: Required<Pick<ClientConfig, 'serverUrl' | 'connectTimeout'>> & ClientConfig;
  private socket: WebSocket | null = null;
  private connected = false;
  private commandCallbacks = new Map<string, CommandCallback>();
  private eventListeners = new Map<string, ((data: unknown) => void)[]>();
  private isAuthenticated = false;
  private reconnectAttempts = 0;
  private eventCallbacks: EventCallbacks = {};
  private activeStreams = new Map<string, (data: NotificationData) => void>();

  constructor(config: ClientConfig) {
    this.config = {
      connectTimeout: 10000,
      debug: false,
      ...config
    };
  }

  private debug(message: string, data?: unknown): void {
    if (this.config.debug) {
      if (data !== undefined) {
        // eslint-disable-next-line no-console
        console.debug(`[PortalSDK] ${message}`, data);
      } else {
        // eslint-disable-next-line no-console
        console.debug(`[PortalSDK] ${message}`);
      }
    }
  }
  
  /**
   * Connect to the Portal server
   */
  public async connect(): Promise<void> {
    if (this.connected) {
      return;
    }

    return new Promise((resolve, reject) => {
      try {
        this.socket = new WebSocket(this.config.serverUrl);
        
        const timeout = setTimeout(() => {
          if (this.socket && this.socket.readyState !== WebSocket.OPEN) {
            this.socket.close();
            reject(new PortalSDKError('Connection timeout', 'CONNECTION_TIMEOUT'));
          }
        }, this.config.connectTimeout);

        this.socket.onopen = () => {
          this.connected = true;
          clearTimeout(timeout);
          resolve();
        };

        this.socket.onclose = () => {
          this.connected = false;
          this.socket = null;
          const err = new PortalSDKError('Connection closed', 'CONNECTION_CLOSED');
          this.commandCallbacks.forEach(({ reject }) => reject(err));
          this.commandCallbacks.clear();
        };

        this.socket.onerror = (error: any) => {
          if (!this.connected) {
            clearTimeout(timeout);
            reject(error);
          }
        };

        this.socket.onmessage = (event: WebSocket.MessageEvent) => this.handleMessage(event);
      } catch (error) {
        reject(error);
      }
    });
  }
  
  /**
   * Disconnect from the Portal server
   */
  public disconnect(): void {
    if (this.socket) {
      this.socket.close();
      this.socket = null;
      this.connected = false;
      this.isAuthenticated = false;
      
      // Clear all active streams and callbacks
      this.activeStreams.clear();
      this.commandCallbacks.clear();
      this.eventListeners.clear();
    }
  }
  
  /** Send a command and wait for raw response (used internally). */
  public async sendCommand<T = unknown>(cmd: string, params: Record<string, unknown> = {}): Promise<T> {
    if (!this.connected || !this.socket) {
      throw new PortalSDKError('Not connected to server', 'NOT_CONNECTED');
    }

    const id = this.generateId();
    const command = {
      id,
      cmd,
      ...(Object.keys(params).length > 0 ? { params } : {}),
    };

    this.debug('send', command);

    return new Promise<T>((resolve, reject) => {
      this.commandCallbacks.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject: (err: PortalSDKError) => reject(err),
      });
      this.socket!.send(JSON.stringify(command));
    });
  }

  /**
   * Send command and expect a specific response type. Throws on error or wrong type.
   * Return type is narrowed from expectedType (e.g. 'key_handshake_url' → { url, stream_id }).
   */
  private async sendExpect<K extends ResponseData['type']>(
    cmd: string,
    params: Record<string, unknown>,
    expectedType: K
  ): Promise<Extract<ResponseData, { type: K }>> {
    const data = await this.sendCommand<ResponseData>(cmd, params);
    if (isResponseType(data, expectedType)) {
      return data as Extract<ResponseData, { type: K }>;
    }
    throw new PortalSDKError(`Unexpected response type: ${(data as ResponseData).type}`, 'UNEXPECTED_RESPONSE');
  }

  /** Register a stream handler; returns a function to unregister. */
  private withStream(streamId: string, handler: (data: NotificationData) => void): () => void {
    this.activeStreams.set(streamId, handler);
    return () => this.activeStreams.delete(streamId);
  }
  
  /** Register event listener: on('connect', fn) or on({ onError: fn }). */
  public on(eventType: string | EventCallbacks, callback?: (data: unknown) => void): void {
    if (typeof eventType === 'object') {
      this.eventCallbacks = { ...this.eventCallbacks, ...eventType };
      return;
    }
    if (typeof eventType === 'string' && callback) {
      const list = this.eventListeners.get(eventType) ?? [];
      list.push(callback);
      this.eventListeners.set(eventType, list);
    }
  }

  /** Remove an event listener. */
  public off(eventType: string, callback: (data: unknown) => void): void {
    if (!this.eventListeners.has(eventType)) {
      return;
    }
    
    const listeners = this.eventListeners.get(eventType)!;
    const index = listeners.indexOf(callback);
    if (index !== -1) {
      listeners.splice(index, 1);
    }
  }
  
  /**
   * Handle messages from the server
   */
  private handleMessage(event: WebSocket.MessageEvent): void {
    try {
      const raw = JSON.parse(event.data.toString());
      const data = raw as object;
      this.debug('message', data);

      if (data !== null && typeof data === 'object' && 'id' in data) {
        const response = data as Response;
        this.debug('response id', response.id);
        const callback = this.commandCallbacks.get(response.id);
        this.commandCallbacks.delete(response.id);

        if (callback) {
          if (response.type === 'error') {
            const code = /auth|token|authenticated/i.test(response.message) ? 'AUTH_FAILED' : 'SERVER_ERROR';
            callback.reject(new PortalSDKError(response.message, code));
          } else if (response.type === 'success') {
            callback.resolve(response.data);
          }
        } else if (response.type === 'notification') {
          const streamId = response.id;
          const handler = this.activeStreams.get(streamId);
          if (handler) {
            handler(response.data as NotificationData);
          } else {
            this.debug('no handler for stream', streamId);
          }
        } else {
          this.debug('no callback for id', response.id);
        }
        return;
      }

      if (data !== null && typeof data === 'object' && 'type' in data) {
        const eventData = data as Event;
        this.eventListeners.get(eventData.type)?.forEach((listener) => listener(eventData.data));
        this.eventListeners.get('all')?.forEach((listener) => listener(eventData));
      }
    } catch (error) {
      this.debug('parse error', error);
      const err = error instanceof Error ? error : new PortalSDKError(String(error), 'PARSE_ERROR', error);
      this.eventCallbacks.onError?.(err);
    }
  }
  
  /**
   * Generate a unique ID for commands
   */
  private generateId(): string {
    return Math.random().toString(36).substring(2, 15) + 
           Math.random().toString(36).substring(2, 15);
  }
  
  /**
   * Authenticate with the server using a token
   */
  public async authenticate(token: string): Promise<void> {
    await this.sendCommand('Auth', { token });
    this.isAuthenticated = true;
    this.reconnectAttempts = 0;
  }
  
  /** Generate a new key handshake URL; onKeyHandshake is called when the user completes handshake. */
  public async newKeyHandshakeUrl(
    onKeyHandshake: (mainKey: string, preferredRelays: string[]) => void,
    staticToken: string | null = null,
    noRequest: boolean | null = false
  ): Promise<string> {
    const response = await this.sendExpect(
      'NewKeyHandshakeUrl',
      { static_token: staticToken, no_request: noRequest },
      'key_handshake_url'
    );
    const handler = (data: NotificationData) => {
      if (data.type === 'key_handshake') {
        onKeyHandshake(data.main_key, data.preferred_relays);
        this.activeStreams.delete(response.stream_id);
      }
    };
    this.withStream(response.stream_id, handler);
    return response.url;
  }

  /** Authenticate with a key (NIP-46 style). */
  public async authenticateKey(mainKey: string, subkeys: string[] = []): Promise<AuthResponseData> {
    const response = await this.sendExpect('AuthenticateKey', { main_key: mainKey, subkeys }, 'auth_response');
    return response.event;
  }

  /** Request a recurring payment. */
  public async requestRecurringPayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: RecurringPaymentRequestContent
  ): Promise<RecurringPaymentResponseContent> {
    const response = await this.sendExpect('RequestRecurringPayment', { main_key: mainKey, subkeys, payment_request: paymentRequest }, 'recurring_payment');
    return response.status;
  }

  /** Request a single payment; onStatusChange is called for status updates until user_failed/user_rejected. */
  public async requestSinglePayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: SinglePaymentRequestContent,
    onStatusChange: (status: InvoiceStatus) => void
  ): Promise<void> {
    const response = await this.sendExpect('RequestSinglePayment', { main_key: mainKey, subkeys, payment_request: paymentRequest }, 'single_payment');
    const handler = (data: NotificationData) => {
      if (data.type === 'payment_status_update') {
        onStatusChange(data.status);
        const s = data.status as InvoiceStatus;
        if (s.status === 'user_failed' || s.status === 'user_rejected') {
          this.activeStreams.delete(response.stream_id);
        }
      }
    };
    this.withStream(response.stream_id, handler);
  }

  /** Request the user to pay an invoice; onStatusChange receives status updates. */
  public async requestInvoicePayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: InvoicePaymentRequestContent,
    onStatusChange: (status: InvoiceStatus) => void
  ): Promise<void> {
    const response = await this.sendExpect('RequestPaymentRaw', { main_key: mainKey, subkeys, payment_request: paymentRequest }, 'single_payment');
    const handler = (data: NotificationData) => {
      if (data.type === 'payment_status_update') {
        onStatusChange(data.status);
        const s = data.status as InvoiceStatus;
        if (s.status === 'user_failed' || s.status === 'user_rejected') {
          this.activeStreams.delete(response.stream_id);
        }
      }
    };
    this.withStream(response.stream_id, handler);
  }

  /** Fetch a user profile. */
  public async fetchProfile(mainKey: string): Promise<Profile | null> {
    const response = await this.sendExpect('FetchProfile', { main_key: mainKey }, 'profile');
    return response.profile;
  }

  /** Set the current user's profile. */
  public async setProfile(profile: Profile): Promise<void> {
    await this.sendCommand('SetProfile', { profile });
  }

  /** Close a recurring payment subscription. */
  public async closeRecurringPayment(mainKey: string, subkeys: string[], subscriptionId: string): Promise<string> {
    const response = await this.sendExpect('CloseRecurringPayment', { main_key: mainKey, subkeys, subscription_id: subscriptionId }, 'close_recurring_payment_success');
    return response.message;
  }

  /** Listen for closed recurring payment events. Call the returned function to stop listening. */
  public async listenClosedRecurringPayment(onClosed: (data: CloseRecurringPaymentNotification) => void): Promise<() => void> {
    const response = await this.sendExpect('ListenClosedRecurringPayment', {}, 'listen_closed_recurring_payment');
    const handler = (data: NotificationData) => {
      if (data.type === 'closed_recurring_payment') {
        onClosed({ reason: data.reason, subscription_id: data.subscription_id, main_key: data.main_key, recipient: data.recipient });
      }
    };
    return this.withStream(response.stream_id, handler);
  }

  /** Request an invoice from a recipient. Server computes exchange rate from amount/currency. */
  public async requestInvoice(
    recipientKey: string,
    subkeys: string[],
    content: RequestInvoiceParams
  ): Promise<InvoiceResponseContent> {
    const response = await this.sendExpect('RequestInvoice', { recipient_key: recipientKey, subkeys, content }, 'invoice_payment');
    return { invoice: response.invoice, payment_hash: response.payment_hash ?? null };
  }

  /** Issue a JWT for the given target key. */
  public async issueJwt(target_key: string, duration_hours: number): Promise<string> {
    const response = await this.sendExpect('IssueJwt', { target_key, duration_hours }, 'issue_jwt');
    return response.token;
  }

  /** Verify a JWT and return claims. */
  public async verifyJwt(public_key: string, token: string): Promise<{ target_key: string }> {
    const response = await this.sendExpect('VerifyJwt', { pubkey: public_key, token }, 'verify_jwt');
    return { target_key: response.target_key };
  }

  /** Request a Cashu token from a recipient. */
  public async requestCashu(
    recipientKey: string,
    subkeys: string[],
    mint_url: string,
    unit: string,
    amount: number
  ): Promise<CashuResponseStatus> {
    const response = await this.sendExpect('RequestCashu', { recipient_key: recipientKey, subkeys, mint_url, unit, amount }, 'cashu_response');
    return response.status;
  }

  /** Send a Cashu token directly to a recipient. */
  public async sendCashuDirect(mainKey: string, subkeys: string[], token: string): Promise<string> {
    const response = await this.sendExpect('SendCashuDirect', { main_key: mainKey, subkeys, token }, 'send_cashu_direct_success');
    return response.message;
  }

  /** Mint Cashu tokens from a mint. */
  public async mintCashu(
    mint_url: string,
    static_auth_token: string | undefined,
    unit: string,
    amount: number,
    description?: string
  ): Promise<string> {
    const response = await this.sendExpect('MintCashu', { mint_url, static_auth_token, unit, amount, description }, 'cashu_mint');
    return response.token;
  }

  /** Burn a Cashu token at a mint. */
  public async burnCashu(mint_url: string, unit: string, token: string, static_auth_token?: string): Promise<number> {
    const response = await this.sendExpect('BurnCashu', { mint_url, unit, token, static_auth_token }, 'cashu_burn');
    return response.amount;
  }

  /** Add a relay to the pool. */
  public async addRelay(relay: string): Promise<string> {
    const response = await this.sendExpect('AddRelay', { relay }, 'add_relay');
    return response.relay;
  }

  /** Remove a relay from the pool. */
  public async removeRelay(relay: string): Promise<string> {
    const response = await this.sendExpect('RemoveRelay', { relay }, 'remove_relay');
    return response.relay;
  }

  /** Calculate next occurrence for a calendar (e.g. "daily", "monthly"). */
  public async calculateNextOccurrence(calendar: string, from: Timestamp): Promise<Timestamp | null> {
    const response = await this.sendExpect('CalculateNextOccurrence', { calendar, from }, 'calculate_next_occurrence');
    const next = response.next_occurrence;
    return next != null ? new Timestamp(typeof next === 'string' ? BigInt(next) : next) : null;
  }

  /** Fetch NIP-05 profile (e.g. user@domain.com). */
  public async fetchNip05Profile(nip05: string): Promise<Nip05Profile> {
    const response = await this.sendExpect('FetchNip05Profile', { nip05 }, 'fetch_nip05_profile');
    return response.profile;
  }

  /** Get wallet type and balance (msat). Fails if no wallet is configured or balance fetch fails. */
  public async getWalletInfo(): Promise<{ wallet_type: string; balance_msat: number }> {
    const response = await this.sendExpect('GetWalletInfo', {}, 'wallet_info');
    return { wallet_type: response.wallet_type, balance_msat: response.balance_msat };
  }

  /** Pay a Lightning invoice (BOLT-11). Returns the payment preimage and fees paid on success. Requires backend wallet. */
  public async payInvoice(invoice: string): Promise<{ preimage: string; fees_paid_msat: number }> {
    const response = await this.sendExpect('PayInvoice', { invoice }, 'pay_invoice');
    return { preimage: response.preimage, fees_paid_msat: response.fees_paid_msat };
  }
}
