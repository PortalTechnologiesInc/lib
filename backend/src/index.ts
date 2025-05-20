import express from 'express';
import { WebSocketServer, WebSocket } from 'ws';
import path from 'path';
import { v4 as uuidv4 } from 'uuid';
import { AuthResponseData, Currency, PaymentStatusContent, PortalSDK, Profile, RecurringPaymentStatusContent, Timestamp } from 'portal-sdk';
import { DatabaseManager, Payment } from './session';
import { bech32 } from 'bech32';

interface LoginStatus {
  type: 'waiting' | 'sending_challenge' | 'approved' | 'timeout';
  displayName?: string;
  publicKey?: string;
  authToken?: string;
}

interface PaymentRequest {
  amount: number;
  description: string;
  payment_type: 'single' | 'recurring';
  frequency: 'minutely' | 'hourly' | 'daily' | 'weekly' | 'monthly' | 'quarterly' | 'semiannually' | 'yearly';
}

const app = express();
const port = process.env.PORT || 8000;
const db = new DatabaseManager();
const loginTokens = new Map<string, LoginStatus>();
const connectionMap = new Map<string, [WebSocket]>();

function formatNpub(mainKey: string) {
  const bytes = new Buffer(mainKey, "hex");
  const npub = bech32.encode('npub', bech32.toWords(bytes));
  return npub.slice(0, 10) + '...' + npub.slice(-10);
}

const portalClient = new PortalSDK({
  serverUrl: 'ws://127.0.0.1:3000/ws',
  connectTimeout: 5000
});
portalClient.connect()
    .then(() => {
        console.log('Connection established')
        mainFunction();
    })
    .catch(error => {
        console.error('Error connecting to server:', error);
        process.exit(1);
    });

function mainFunction() {
  const authToken = process.env.AUTH_TOKEN || 'your-auth-token'; // Replace with your actual token
  portalClient.authenticate(authToken)
    .then(() => {
        console.log('Authentication successful');
        portalClient.setProfile({
            id: '',
            pubkey: '',
            name: 'Portal Demo',
            display_name: 'Portal Demo',
            picture: 'https://getportal.cc/logo-nip05.png',
            nip05: 'demo@getportal.cc',
        });
    })
    .catch(error => {
        console.error('Error authenticating:', error);
        process.exit(1);
    });
  
  // Serve static files from the public directory
  app.use(express.static(path.join(__dirname, '../public')));
  
  app.get('/logout', (req, res) => {
    const sessionId = req.cookies?.session_id;
    if (sessionId) {
      db.deleteSession(sessionId);
      res.clearCookie('session_id');
    }
    res.redirect('/');
  });
  
  // Create HTTP server
  const server = app.listen(port, () => {
    console.log(`Server running at http://localhost:${port}`);
  });
  
  // Initialize WebSocket server
  const wss = new WebSocketServer({ server });

  wss.on('connection', async (ws: WebSocket, req) => {
    console.log('New WebSocket connection');
  
    const mainKey = req.headers.cookie?.split(';')
      .find(c => c.trim().startsWith('main_key='))
      ?.split('=')[1];
    const sessionId = req.headers.cookie?.split(';')
        .find(c => c.trim().startsWith('session_id='))
        ?.split('=')[1];
  
    const isDashboard = req.url?.includes('dashboard');
    if (isDashboard) {
      // Dashboard connection
      if (!sessionId || !db.hasSession(sessionId)) {
          console.log('No session found');
          ws.send(`<span id="redirect" data-url="/"></span>`);
      } else {
        const session = db.getSession(sessionId)!;
        const map = connectionMap.get(session.publicKey);
        if (!map) {
            connectionMap.set(session.publicKey, [ws]);
        } else {
            map.push(ws);
        }

        // Send user info
        ws.send(`<span id="user-name">${session.displayName}</span>`);
  
        // Handle payment requests
        ws.on('message', async (message: Buffer) => {
          try {
            const data = JSON.parse(message.toString());
            console.log('Received payment request:', data);

            const payment = data as PaymentRequest;
            payment.amount = parseInt(data.amount);

            if (payment.payment_type === 'single') {
                claimSinglePayment(connectionMap.get(session.publicKey) || [ws], session.publicKey, undefined, session.authToken, payment);
            } else {
                const subscriptionId = uuidv4();
                const nextPaymentAt = Math.floor(Date.now() / 1000); // Now

                // Create subscription record
                const subscription = db.createSubscription({
                  id: subscriptionId,
                  publicKey: session.publicKey,
                  amount: payment.amount,
                  frequency: payment.frequency,
                  status: 'active',
                  lastPaymentAt: null,
                  nextPaymentAt,
                  authToken: session.authToken,
                  portalSubscriptionId: null
                });

                const payReq = {
                    amount: payment.amount * 1000,
                    currency: Currency.Millisats,
                    auth_token: session.authToken,
                    recurrence: {
                        calendar: payment.frequency,
                        first_payment_due: new Timestamp(nextPaymentAt),
                    },
                    expires_at: Timestamp.fromNow(3600),
                    current_exchange_rate: undefined,
                };

                const paymentResult = await portalClient.requestRecurringPayment(
                    session.publicKey,
                    [],
                    payReq,
                );
                
                console.log('Recurring payment result:', paymentResult);
                db.updateSubscriptionStatus(subscriptionId, 'active', nextPaymentAt, paymentResult.subscription_id);
                
                sendHistory(ws, session.publicKey);
            }
          } catch (err) {
            console.error('Error processing message:', err);
          }
        });

        sendHistory(ws, session.publicKey);
     }
    } else if (mainKey) {
      console.log(`User has main key: ${mainKey}`);

      if (sessionId && db.hasSession(sessionId)) {
          console.log(`Resuming session ${sessionId}`);
          ws.send(`
              <div id="status" class="status approved" x-session-id="${sessionId}" x-main-key="${mainKey}">
                Login approved! Redirecting...
              </div>
          `);
      } else {
          console.log('Resuming session for user with main key');

          ws.send(`
                <div id="qr-overlay" class="show">Loading...</div>
                <div id="login-button-section">
                  <a href="#" class="login-button disabled" id="portal-login">Login with Portal</a>
                </div>
            `);

          ws.on('message', (message: Buffer) => {
            const data = JSON.parse(message.toString());
            if (data.HEADERS['HX-Trigger'] === 'portal-login') {
                ws.send(`
                    <div id="login-button-section">
                      <a href="#" class="login-button disabled" id="portal-login">Login with Portal</a>
                    </div>
                    <div class="local-login" id="forget-user-section">
                    </div>
                    `);
                authenticateKey(portalClient, ws, mainKey, mainKey);
            }
          });
  
          ws.send(`
              <div id="qr-section">
                <h3>Hello ${formatNpub(mainKey)}!</h3>
              </div>
              <div class="local-login" id="login-button-section">
                <a href="#" class="login-button" id="portal-login" ws-send='{"cmd": "login"}'>Login with Portal</a>
              </div>
              <div class="local-login" id="forget-user-section">
                <a href="#" class="login-button forget-user-button" id="forget-user" onclick="window.resetMainKey()">Not you?</a>
              </div>
              <div id="status" class="status sending">
                Welcome back, ${formatNpub(mainKey)}!
              </div>
          `);

          portalClient.fetchProfile(mainKey)
            .then(profile => {
              console.log('Profile:', profile);

              if (!profile || !profile.name){
                return;
              }

              loginTokens.set(mainKey, {
                type: 'sending_challenge',
                displayName: profile.name,
              });

              ws.send(`
                <div id="qr-section">
                  <h3>Hello ${profile.name}!</h3>
                </div>
                <div id="status" class="status sending">
                  Welcome back, ${profile.name}!
                </div>
              `);
            });
      }
    } else {
      const loginUrl = await portalClient.newAuthInitUrl((mainKey) => {
          console.log('Auth Init received for key:', mainKey);

          const status = loginTokens.get(loginUrl);
          if (status && status.type === 'waiting') {
            loginTokens.set(loginUrl, {
              type: 'sending_challenge',
              displayName: formatNpub(mainKey),
            });
          } else {
            return;
          }
          
          ws.send(`
            <div id="status" class="status sending">
              Welcome back, ${formatNpub(mainKey)}!
            </div>
            <div id="qr-overlay" class="show">Loading...</div>
            <div id="login-button-section">
              <a href="#" class="login-button disabled" id="portal-login">Login with Portal</a>
            </div>
          `);

          // Fetch the profile in background
          portalClient.fetchProfile(mainKey)
            .then(profile => {
              console.log('Profile:', profile);

              if (profile) {
                ws.send(`
                  <div id="status" class="status sending">
                    Welcome back, ${profile.name}!
                  </div>
                `);

                const current = loginTokens.get(loginUrl);
                if (current) {
                  current.displayName = profile.name || formatNpub(mainKey);
                  loginTokens.set(loginUrl, current);
                }
              }
            })
            .catch(error => {
              console.log('Error fetching profile:', error);
            });
  
          return authenticateKey(portalClient, ws, loginUrl, mainKey);
      });

      loginTokens.set(loginUrl, { type: 'waiting' });
      
      // Send HTML updates
      ws.send(`
        <div id="login-button-section">
          <a href="${loginUrl}" class="login-button" id="portal-login">Login with Portal</a>
        </div>
        <canvas id="qrcode" data-url="${loginUrl}"></canvas>
      `);
    }
  
    ws.on('close', () => {
      console.log('Client disconnected');

      if (mainKey) {
        const connections = connectionMap.get(mainKey);
        if (connections) {
          connections.splice(connections.indexOf(ws), 1);
        }
      }
    });
  }); 
}

async function authenticateKey(portalClient: PortalSDK, ws: WebSocket, loginUrl: string, mainKey: string) {
    let authResponse: AuthResponseData | null = null;
    let timeout: NodeJS.Timeout | null = null;
    try {
      authResponse = await Promise.race([
        portalClient.authenticateKey(mainKey),
        new Promise((resolve, reject) => {
          timeout = setTimeout(() => {
            reject(new Error('Authentication timed out'));
          }, 10000);
        })
      ]) as AuthResponseData | null;

      if (timeout) {
        clearTimeout(timeout);
      }
    } catch (error) {
      console.log(error);
      ws.send(`<div id="status" class="status timeout">Authentication timed out</div>`);
      return;
    }

    const sessionId = uuidv4();
    
    const current = loginTokens.get(loginUrl);
    console.log(current);
    let name = null;
    if (current) {
      name = current.displayName;
    }

    loginTokens.set(loginUrl, {
      type: 'approved',
      displayName: name || mainKey,
      publicKey: mainKey,
      authToken: authResponse!.session_token,
    });
    
    // Create session
    db.setSession({
      id: sessionId,
      publicKey: mainKey,
      displayName: name || formatNpub(mainKey),
      authToken: authResponse!.session_token,
    });
    
    ws.send(`
        <div id="status" class="status approved" x-session-id="${sessionId}" x-main-key="${mainKey}">
          Login approved! Redirecting...
        </div>
      `);
}

function sendHistory(ws: WebSocket, publicKey: string) {
    // Send existing payments and subscriptions
    const payments = db.getPublicKeyPayments(publicKey);
    const subscriptions = db.getPublicKeySubscriptions(publicKey);
    
    ws.send(`
        <div id="history-section">
        <h3>Payment History</h3>
        <div class="payment-list">
            ${payments.map(p => `
            <div class="payment-item ${p.status}">
                <span class="amount">${p.amount} sats</span>
                <span class="description">${p.description}</span>
                <span class="status">${p.status}</span>
                <span class="date">${new Date(p.createdAt * 1000).toLocaleString()}</span>
            </div>
            `).join('')}
        </div>
        
        <h3>Active Subscriptions</h3>
        <div class="subscription-list">
            ${subscriptions.filter(s => s.status === 'active').map(s => `
            <div class="subscription-item">
                <span class="amount">${s.amount} sats</span>
                <span class="frequency">${s.frequency}</span>
                <span class="next-payment">Next: ${new Date(s.nextPaymentAt * 1000).toLocaleString()}</span>
            </div>
            `).join('')}
        </div>
        </div>
    `);
}

async function claimSinglePayment(
    ws: WebSocket[],
    publicKey: string,
    subscriptionId: string | undefined,
    authToken: string | undefined,
    payment: PaymentRequest,
    successCallback?: (status: PaymentStatusContent) => void
) {
    const paymentId = uuidv4();
    
    // Create payment record
    const paymentRecord = db.createPayment({
        id: paymentId,
        publicKey: publicKey,
        amount: payment.amount,
        description: payment.description,
        status: 'pending'
    });
    for (const w of ws) {
        sendHistory(w, publicKey);
    }

    const payReq = {
        amount: payment.amount * 1000,
        description: payment.description,
        currency: Currency.Millisats,
        auth_token: authToken,
        subscription_id: subscriptionId,
    };
    
    const paymentResult = await portalClient.requestSinglePayment(
        publicKey,
        [],
        payReq,
        (status) => {
            console.log('Payment status update:', status);
            // Update payment status in database
            let dbStatus: 'pending' | 'completed' | 'failed';
            if (status.status === 'paid') {
                if (successCallback) {
                    successCallback(status);
                }
                dbStatus = 'completed';
            } else if (status.status === 'failed' || status.status === 'rejected' || status.status === 'timeout') {
                dbStatus = 'failed';
            } else {
                dbStatus = 'pending';
            }
            db.updatePaymentStatus(paymentId, dbStatus);
            for (const w of ws) {
                sendHistory(w, publicKey);
            }
        }
    );

    console.log('Single payment result:', paymentResult);
    let finalStatus: 'pending' | 'completed' | 'failed';
    if (paymentResult.status === 'paid') {
        finalStatus = 'completed';
    } else if (paymentResult.status === 'failed' || paymentResult.status === 'rejected' || paymentResult.status === 'timeout') {
        finalStatus = 'failed';
    } else {
        finalStatus = 'pending';
    }
    db.updatePaymentStatus(paymentId, finalStatus);
    for (const w of ws) {
        sendHistory(w, publicKey);
    }
}

// Helper function to calculate next payment timestamp
function calculateNextPayment(fromTimestamp: number, frequency: string): number {
  const date = new Date(fromTimestamp * 1000);
  
  switch (frequency) {
    case 'minutely':
      return Math.floor(date.setMinutes(date.getMinutes() + 1) / 1000);
    case 'hourly':
      return Math.floor(date.setHours(date.getHours() + 1) / 1000);
    case 'daily':
      return Math.floor(date.setDate(date.getDate() + 1) / 1000);
    case 'weekly':
      return Math.floor(date.setDate(date.getDate() + 7) / 1000);
    case 'monthly':
      return Math.floor(date.setMonth(date.getMonth() + 1) / 1000);
    case 'quarterly':
      return Math.floor(date.setMonth(date.getMonth() + 3) / 1000);
    case 'semiannually':
      return Math.floor(date.setMonth(date.getMonth() + 6) / 1000);
    case 'yearly':
      return Math.floor(date.setFullYear(date.getFullYear() + 1) / 1000);
    default:
      throw new Error(`Unknown frequency: ${frequency}`);
  }
}

// Process subscriptions every minute
setInterval(() => {
  const now = Math.floor(Date.now() / 1000);
  const subscriptions = db.getDueSubscriptions(now);
  for (const subscription of subscriptions) {
    if (!subscription.portalSubscriptionId) {
      continue;
    }

    const ws = connectionMap.get(subscription.publicKey);
    const payReq = {
        amount: subscription.amount,
        description: `Payment  for subscription ${subscription.portalSubscriptionId}`,
    };
    claimSinglePayment(ws || [], subscription.publicKey, subscription.portalSubscriptionId, subscription.authToken, payReq as PaymentRequest, () => {
        db.updateSubscriptionStatus(subscription.id, 'active', calculateNextPayment(now, subscription.frequency), subscription.portalSubscriptionId!);
    });
  }
}, 60000);

// Optional: Add session cleanup on a schedule
setInterval(() => {
  db.cleanup();
}, 3600000); // Run cleanup every hour