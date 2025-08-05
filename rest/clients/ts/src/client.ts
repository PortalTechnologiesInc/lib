import WebSocket from 'ws';
import {
  ClientConfig,
  Command,
  Response,
  NotificationData,
  EventCallbacks,
  RecurringPaymentRequestContent,
  SinglePaymentRequestContent,
  Profile,
  AuthResponseData,
  Event,
  InvoicePaymentRequestContent,
  RecurringPaymentResponseContent,
  CloseRecurringPaymentNotification,
  InvoiceStatus,
  InvoiceResponseContent,
  CashuRequestContentWithKey,
  CashuResponseContent,
  CashuRequestContent,
  CashuResponseStatus,
} from './types';

/**
 * Portal client for interacting with the Portal server
 */
export class PortalSDK {
  private config: ClientConfig;
  private socket: WebSocket | null = null;
  private connected = false;
  private commandCallbacks: Map<string, { resolve: Function; reject: Function }> = new Map();
  private eventListeners: Map<string, ((data: any) => void)[]> = new Map();
  private isAuthenticated = false;
  private reconnectAttempts = 0;
  private eventCallbacks: EventCallbacks = {};
  private activeStreams = new Map<string, (data: NotificationData) => void>();
  
  /**
   * Create a new Portal client
   */
  constructor(config: ClientConfig) {
    this.config = {
      connectTimeout: 10000,
      ...config
    };
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
            reject(new Error('Connection timeout'));
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
  public async sendCommand<T = any>(cmd: string, params: Record<string, any> = {}): Promise<T> {
    if (!this.connected || !this.socket) {
      throw new Error('Not connected to server');
    }

    const id = this.generateId();
    // Format command to match server's expected format
    const command = {
      id,
      cmd,
      ...(Object.keys(params).length > 0 ? { params } : {})
    };

    console.log('Sending command:', JSON.stringify(command, null, 2));
    console.log('Registered callback for id:', id);

    return new Promise<T>((resolve, reject) => {
      this.commandCallbacks.set(id, { resolve, reject });
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
      const data = JSON.parse(event.data.toString());
      console.log('Received message:', JSON.stringify(data, null, 2));
      
      // Handle command responses
      if ('id' in data) {
        const response = data as Response;
        console.log('Looking for callback with id:', response.id);
        const callback = this.commandCallbacks.get(response.id);
        
        this.commandCallbacks.delete(response.id);

        if (callback) {
          console.log('Found callback for id:', response.id);

          if (response.type === 'error') {
            callback.reject(new Error(response.message));
          } else if (response.type === 'success') {
            callback.resolve(response.data);
          }
        } else if (response.type === 'notification') {
          const streamId = response.id;
          const handler = this.activeStreams.get(streamId);
          if (handler) {
            handler(response.data);
          } else {
            console.log('No handler found for stream id:', streamId);
          }
        } else {
          console.log('No callback found for id:', response.id);
        }

        return;
      }
      
      // Handle events
      if ('type' in data) {
        const eventData = data as Event;
        const listeners = this.eventListeners.get(eventData.type);
        
        if (listeners) {
          listeners.forEach(listener => listener(eventData.data));
        }
        
        // Also trigger 'all' event listeners
        const allListeners = this.eventListeners.get('all');
        if (allListeners) {
          allListeners.forEach(listener => listener(eventData));
        }
      }
    } catch (error) {
      console.error('Error handling message:', error);
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
    const response = await this.sendCommand('Auth', { token });
    
    if (response.type === 'error') {
      throw new Error(`Authentication failed: ${response.message}`);
    }
    
    this.isAuthenticated = true;
    this.reconnectAttempts = 0; // Reset reconnect attempts on successful auth
  }
  
  /**
   * Generate a new key handshake URL
   */
  public async newKeyHandshakeUrl(onKeyHandshake: (mainKey: string, preferredRelays: string[]) => void, staticToken: string | null = null): Promise<string> {
    const _self = this;
    let streamId = '';

    const handler = (data: NotificationData) => {
      if (data.type === 'key_handshake') {
        onKeyHandshake(data.main_key, data.preferred_relays);
        _self.activeStreams.delete(streamId);
      }
    };
    
    const response = await this.sendCommand('NewKeyHandshakeUrl', { static_token: staticToken });
    
    if (response.type === 'key_handshake_url') {
      const { url, stream_id } = response;

      streamId = stream_id;
      this.activeStreams.set(stream_id, handler);

      return url;
    }
    
    throw new Error('Unexpected response type');
  }
  
  /**
   * Authenticate a key with the server
   */
  public async authenticateKey(mainKey: string, subkeys: string[] = []): Promise<AuthResponseData> {
    const response = await this.sendCommand('AuthenticateKey', { main_key: mainKey, subkeys });
    
    if (response.type === 'auth_response') {
      return response.event;
    }
    
    throw new Error('Unexpected response type');
  }
  
  /**
   * Request a recurring payment
   */
  public async requestRecurringPayment(
    mainKey: string,
    subkeys: string[] = [],
    paymentRequest: RecurringPaymentRequestContent
  ): Promise<RecurringPaymentResponseContent> {
    const response = await this.sendCommand('RequestRecurringPayment', { main_key: mainKey, subkeys, payment_request: paymentRequest });
    
    if (response.type === 'recurring_payment') {
      return response.status;
    }
    
    throw new Error('Unexpected response type');
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
          console.log('Deleting stream id:', streamId);
          _self.activeStreams.delete(streamId);
        }
      }
    };

    const response = await this.sendCommand('RequestSinglePayment', { main_key: mainKey, subkeys, payment_request: paymentRequest });
    
    if (response.type === 'single_payment') {
      streamId = response.stream_id;
      this.activeStreams.set(streamId, handler);

      return;
    }
    
    throw new Error('Unexpected response type');
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
          console.log('Deleting stream id:', streamId);
          _self.activeStreams.delete(streamId);
        }
      }
    };

    const response = await this.sendCommand('RequestPaymentRaw', { main_key: mainKey, subkeys, payment_request: paymentRequest });
    
    if (response.type === 'single_payment') {
      streamId = response.stream_id;
      this.activeStreams.set(streamId, handler);

      return;
    }
    
    throw new Error('Unexpected response type');
  }
 
  
  /**
   * Fetch a user profile
   */
  public async fetchProfile(mainKey: string): Promise<Profile | null> {
    const response = await this.sendCommand('FetchProfile', { main_key: mainKey });
    
    if (response.type === 'profile') {
      return response.profile;
    }
    
    throw new Error('Unexpected response type');
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
    const response = await this.sendCommand('CloseRecurringPayment', { main_key: mainKey, subkeys, subscription_id: subscriptionId });
    
    if (response.type === 'close_recurring_payment_success') {
      return response.message;
    }
    
    throw new Error('Unexpected response type');
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

    const response = await this.sendCommand('ListenClosedRecurringPayment');
    
    if (response.type === 'listen_closed_recurring_payment') {
      this.activeStreams.set(response.stream_id, handler);
      return;
    }
    
    throw new Error('Unexpected response type');
  }

  /**
   * Request an invoice
   */
  public async requestInvoice(
    recipientKey: string,
    content: InvoicePaymentRequestContent) : Promise<InvoiceResponseContent> {
    return this.sendCommand<InvoiceResponseContent>('RequestInvoice', {
      recipient_key: recipientKey,
      content
    });
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
    const response = await this.sendCommand('RequestCashu', { recipient_key: recipientKey, subkeys, mint_url, unit, amount });
    if (response.type === 'cashu_response') {
      return response.status;
    }
    throw new Error('Unexpected response type');
  }

  /**
   * Send a Cashu token directly to a recipient
   */
  public async sendCashuDirect(mainKey: string, subkeys: string[], token: string): Promise<string> {
    const response = await this.sendCommand('SendCashuDirect', { main_key: mainKey, subkeys, token });
    if (response.type === 'send_cashu_direct_success') {
      return response.message;
    }
    throw new Error('Unexpected response type');
  }

  /**
   * Mint a Cashu token from a mint and return it
   */
  public async mintCashu(mint_url: string, static_auth_token: string | undefined, unit: string, amount: number, description?: string): Promise<string> {
    const response = await this.sendCommand('MintCashu', { mint_url, static_auth_token, unit, amount, description });
    if (response.type === 'cashu_mint') {
      return response.token;
    }
    throw new Error('Unexpected response type');
  }

  /**
   * Burn a Cashu token at a mint
   */
  public async burnCashu(mint_url: string, unit: string, token: string, static_auth_token?: string): Promise<number> {
    const response = await this.sendCommand('BurnCashu', { mint_url, unit, token, static_auth_token });
    if (response.type === 'cashu_burn') {
      return response.amount;
    }
    throw new Error('Unexpected response type');
  }
}
