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
  InvoiceRequestContent,
  InvoiceResponseContent,
  RecurringPaymentResponseContent,
  CloseRecurringPaymentNotification,
  InvoiceStatus,
  CashuResponseStatus,
  Timestamp,
  Nip05Profile,
} from './types';
import { PortalSDKError } from './errors';

type CommandCallback = { resolve: (value: unknown) => void; reject: (reason: PortalSDKError) => void };

/**
 * Official TypeScript/JavaScript client for the Portal WebSocket API.
 * Handles authentication, payments, profiles, JWT, relays, and Cashu.
 *
 * @example
 * ```ts
 * const client = new PortalSDK({ serverUrl: 'ws://localhost:3000/ws' });
 * await client.connect();
 * await client.authenticate(process.env.AUTH_TOKEN!);
 * const url = await client.newKeyHandshakeUrl((mainKey) => { ... });
 * ```
 */
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
  
  /**
   * Send a command to the server and wait for the response
   */
  public async sendCommand<T = unknown>(cmd: string, params: Record<string, unknown> = {}): Promise<T> {
    if (!this.connected || !this.socket) {
      throw new PortalSDKError('Not connected to server', 'NOT_CONNECTED');
    }

    const id = this.generateId();
    const command = {
      id,
      cmd,
      ...(Object.keys(params).length > 0 ? { params } : {})
    };

    this.debug('send', command);

    return new Promise<T>((resolve, reject) => {
      this.commandCallbacks.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject: (err: PortalSDKError) => reject(err)
      });
      this.socket!.send(JSON.stringify(command));
    });
  }
  
  /**
   * Register an event listener or event callbacks
   */
  public on(eventType: string | EventCallbacks, callback?: (data: any) => void): void {
    // Handle object form (EventCallbacks)
    if (typeof eventType === 'object') {
      this.eventCallbacks = { ...this.eventCallbacks, ...eventType };
      return;
    }
    
    // Handle string form with callback
    if (typeof eventType === 'string' && callback) {
      if (!this.eventListeners.has(eventType)) {
        this.eventListeners.set(eventType, []);
      }
      this.eventListeners.get(eventType)!.push(callback);
    }
  }
  
  /**
   * Remove an event listener
   */
  public off(eventType: string, callback: (data: any) => void): void {
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
            callback.resolve(response.data as ResponseData);
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
        const listeners = this.eventListeners.get(eventData.type);
        if (listeners) {
          listeners.forEach((listener) => listener(eventData.data as unknown));
        }
        const allListeners = this.eventListeners.get('all');
        if (allListeners) {
          allListeners.forEach((listener) => listener(eventData as unknown));
        }
      }
    } catch (error) {
      this.debug('parse error', error);
      this.eventCallbacks.onError?.(error instanceof Error ? error : new PortalSDKError(String(error), 'PARSE_ERROR', error));
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
  
  /**
   * Generate a new key handshake URL
   */
  public async newKeyHandshakeUrl(onKeyHandshake: (mainKey: string, preferredRelays: string[]) => void, staticToken: string | null = null, noRequest: boolean | null = false): Promise<string> {
    const _self = this;
    let streamId = '';

    const handler = (data: NotificationData) => {
      if (data.type === 'key_handshake') {
        onKeyHandshake(data.main_key, data.preferred_relays);
        _self.activeStreams.delete(streamId);
      }
    };
    
    const response = await this.sendCommand<ResponseData>('NewKeyHandshakeUrl', { static_token: staticToken, no_request: noRequest });
    if (response.type === 'key_handshake_url') {
      const { url, stream_id } = response;

      streamId = stream_id;
      this.activeStreams.set(stream_id, handler);

      return url;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }
  
  /**
   * Authenticate a key with the server
   */
  public async authenticateKey(mainKey: string, subkeys: string[] = []): Promise<AuthResponseData> {
    const response = await this.sendCommand<ResponseData>('AuthenticateKey', { main_key: mainKey, subkeys });
    if (response.type === 'auth_response') {
      return response.event;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }
  
  /**
   * Request a recurring payment
   */
  public async requestRecurringPayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: RecurringPaymentRequestContent
  ): Promise<RecurringPaymentResponseContent> {
    const response = await this.sendCommand<ResponseData>('RequestRecurringPayment', { main_key: mainKey, subkeys, payment_request: paymentRequest });
    if (response.type === 'recurring_payment') {
      return response.status as RecurringPaymentResponseContent;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }
  
  /**
   * Request a single payment
   * @param mainKey The main key to use for authentication
   * @param subkeys Optional subkeys for authentication
   * @param paymentRequest The payment request details
   * @param onStatusChange Callback function to handle payment status updates
   * @returns The initial payment status
   */
  public async requestSinglePayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: SinglePaymentRequestContent,
    onStatusChange: (status: InvoiceStatus) => void
  ): Promise<void> {
    const _self = this;
    let streamId: string;

    const handler = (data: NotificationData) => {
      if (data.type === 'payment_status_update') {
        onStatusChange(data.status as InvoiceStatus);

        if (data.status.status === 'user_failed' || data.status.status === 'user_rejected') {
          _self.activeStreams.delete(streamId);
        }
      }
    };

    const response = await this.sendCommand<ResponseData>('RequestSinglePayment', { main_key: mainKey, subkeys, payment_request: paymentRequest });
    if (response.type === 'single_payment') {
      streamId = response.stream_id;
      this.activeStreams.set(streamId, handler);

      return;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Request the user to pay an invoice
   * @param mainKey The main key to use for authentication
   * @param subkeys Optional subkeys for authentication
   * @param paymentRequest The payment request details
   * @returns The initial payment status
   */
  public async requestInvoicePayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: InvoicePaymentRequestContent,
    onStatusChange: (status: InvoiceStatus) => void
  ): Promise<void> {
    const _self = this;
    let streamId: string;

    const handler = (data: NotificationData) => {
      if (data.type === 'payment_status_update') {
        onStatusChange(data.status as InvoiceStatus);

        if (data.status.status === 'user_failed' || data.status.status === 'user_rejected') {
          _self.activeStreams.delete(streamId);
        }
      }
    };

    const response = await this.sendCommand<ResponseData>('RequestPaymentRaw', { main_key: mainKey, subkeys, payment_request: paymentRequest });
    if (response.type === 'single_payment') {
      streamId = response.stream_id;
      this.activeStreams.set(streamId, handler);

      return;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }
 
  
  /**
   * Fetch a user profile
   */
  public async fetchProfile(mainKey: string): Promise<Profile | null> {
    const response = await this.sendCommand<ResponseData>('FetchProfile', { main_key: mainKey });
    
    if (response.type === 'profile') {
      return response.profile;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Set a user profile
   */
  public async setProfile(profile: Profile): Promise<void> {
    await this.sendCommand('SetProfile', { profile });
  }

  /**
   * Close a recurring payment
   */
  public async closeRecurringPayment(mainKey: string, subkeys: string[], subscriptionId: string): Promise<string> {
    const response = await this.sendCommand<ResponseData>('CloseRecurringPayment', { main_key: mainKey, subkeys, subscription_id: subscriptionId });
    
    if (response.type === 'close_recurring_payment_success') {
      return response.message;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Listen for closed recurring payments
   */
  public async listenClosedRecurringPayment(onClosed: (data: CloseRecurringPaymentNotification) => void): Promise<void> {
    const handler = (data: NotificationData) => {
      if (data.type === 'closed_recurring_payment') {
        onClosed({
          reason: data.reason,
          subscription_id: data.subscription_id,
          main_key: data.main_key,
          recipient: data.recipient
        });
        // _self.activeStreams.delete(streamId);
      }
    };

    const response = await this.sendCommand<ResponseData>('ListenClosedRecurringPayment');
    
    if (response.type === 'listen_closed_recurring_payment') {
      this.activeStreams.set(response.stream_id, handler);
      return;
    }
    
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Request an invoice from a recipient
   */
  public async requestInvoice(
    recipientKey: string,
    subkeys: string[],
    content: InvoiceRequestContent
  ): Promise<InvoiceResponseContent> {
    const response = await this.sendCommand<{ type: 'invoice_payment'; invoice: string; payment_hash: string | null }>('RequestInvoice', {
      recipient_key: recipientKey,
      subkeys,
      content
    });
    return { invoice: response.invoice, payment_hash: response.payment_hash ?? null };
  }

  /**
   * Issue a JWT token for a given target key
   */
  public async issueJwt(target_key: string, duration_hours: number): Promise<string> {
    return this.sendCommand<{ type: 'issue_jwt', token: string }>('IssueJwt', {
      target_key,
      duration_hours
    }).then(response => response.token);
  }

  /**
   * Verify a JWT token and return the claims
   */
  public async verifyJwt(public_key: string, token: string): Promise<{ target_key: string}> {
    return this.sendCommand<{ type: 'verify_jwt', target_key: string }>('VerifyJwt', {
      pubkey: public_key,
      token
    }).then(response => ({
      target_key: response.target_key,
    }));
  }

  /**
   * Request a Cashu token from a recipient
   */
  public async requestCashu(
    recipientKey: string,
    subkeys: string[],
    mint_url: string,
    unit: string,
    amount: number
  ): Promise<CashuResponseStatus> {
    const response = await this.sendCommand<ResponseData>('RequestCashu', { recipient_key: recipientKey, subkeys, mint_url, unit, amount });
    if (response.type === 'cashu_response') {
      return response.status;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Send a Cashu token directly to a recipient
   */
  public async sendCashuDirect(mainKey: string, subkeys: string[], token: string): Promise<string> {
    const response = await this.sendCommand<ResponseData>('SendCashuDirect', { main_key: mainKey, subkeys, token });
    if (response.type === 'send_cashu_direct_success') {
      return response.message;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Mint a Cashu token from a mint and return it
   */
  public async mintCashu(mint_url: string, static_auth_token: string | undefined, unit: string, amount: number, description?: string): Promise<string> {
    const response = await this.sendCommand<ResponseData>('MintCashu', { mint_url, static_auth_token, unit, amount, description });
    if (response.type === 'cashu_mint') {
      return response.token;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Burn a Cashu token at a mint
   */
  public async burnCashu(mint_url: string, unit: string, token: string, static_auth_token?: string): Promise<number> {
    const response = await this.sendCommand<ResponseData>('BurnCashu', { mint_url, unit, token, static_auth_token });
    if (response.type === 'cashu_burn') {
      return response.amount;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Add a relay to the relay pool
   */
  public async addRelay(relay: string): Promise<string> {
    const response = await this.sendCommand<ResponseData>('AddRelay', { relay });
    if (response.type === 'add_relay') {
      return response.relay;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Remove a relay from the relay pool
   */
  public async removeRelay(relay: string): Promise<string> {
    const response = await this.sendCommand<ResponseData>('RemoveRelay', { relay });
    if (response.type === 'remove_relay') {
      return response.relay;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Calculate the next occurrence for a calendar from a given timestamp
   */
  public async calculateNextOccurrence(calendar: string, from: Timestamp): Promise<Timestamp | null> {
    const response = await this.sendCommand<{ type: 'calculate_next_occurrence'; next_occurrence: string | number | null }>('CalculateNextOccurrence', {
      calendar,
      from
    });
    if (response.type === 'calculate_next_occurrence') {
      const next = response.next_occurrence;
      return next != null && next !== undefined ? new Timestamp(BigInt(next)) : null;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }

  /**
   * Fetch a NIP-05 profile (e.g. user@domain.com)
   */
  public async fetchNip05Profile(nip05: string): Promise<Nip05Profile> {
    const response = await this.sendCommand<{ type: 'fetch_nip05_profile'; profile: Nip05Profile }>('FetchNip05Profile', { nip05 });
    if (response.type === 'fetch_nip05_profile') {
      return response.profile;
    }
    throw new PortalSDKError('Unexpected response type', 'UNEXPECTED_RESPONSE');
  }
}
