import { PortalSDK, Currency, RecurringPaymentRequestContent, SinglePaymentRequestContent, Timestamp } from './src/index';
import * as readline from 'readline';

// Store unsubscribe functions at the module level
async function testFullFlow(client: PortalSDK, mainKey: string, subkeys: string[]) {
    // Example 2: Key Authentication
    console.log('\n=== Key Authentication ===');
    const authResponse = await client.authenticateKey(mainKey, subkeys);
    console.log('Authentication successful:', authResponse);

    // Example 3: Recurring Payment
    console.log('\n=== Recurring Payment ===');
    const recurringPayment: RecurringPaymentRequestContent = {
      amount: 10 * 1000,
      currency: Currency.Millisats,
      recurrence: {
        calendar: "monthly",
        first_payment_due: Timestamp.fromNow(86400), // 24 hours from now
        max_payments: 12
      },
      expires_at: Timestamp.fromNow(3600) // 1 hour from now
    };

    const recurringStatus = await client.requestRecurringPayment(
      mainKey,
      subkeys,
      recurringPayment
    );
    console.log('Recurring payment status:', recurringStatus);

    // Example 4: Single Payment
    console.log('\n=== Single Payment ===');
    const singlePayment: SinglePaymentRequestContent = {
      amount: 11 * 1000,
      currency: Currency.Millisats,
      description: "Test payment",
      subscription_id: recurringStatus.status.status === 'confirmed' ? recurringStatus.status.subscription_id : undefined
    };

    await client.requestSinglePayment(
      mainKey,
      subkeys,
      singlePayment,
      (status) => {
        console.log('Payment status update:', status);
      }
    );
    
    // Example 5: Fetch Profile
    console.log('\n=== Fetch Profile ===');
    const profile = await client.fetchProfile(mainKey);
    console.log('User profile:', profile);
}

async function main() {
  // Create a new client instance
  const client = new PortalSDK({
    serverUrl: 'ws://localhost:7000/ws',
    connectTimeout: 5000,
    debug: false
  });

  // Create readline interface for user input
  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
  });

  try {
    // Connect to the server
    console.log('Connecting to server...');
    await client.connect();
    console.log('Connected successfully');

    // Set up event listeners
    client.on({
      onConnected: () => console.log('Connection established'),
      onDisconnected: () => console.log('Disconnected from server'),
      onError: (error) => console.error('Error:', error)
    });

    // First, authenticate with the server
    console.log('\n=== Authentication ===');
    const authToken = process.env.AUTH_TOKEN || 'your-auth-token'; // Replace with your actual token
    await client.authenticate(authToken);
    console.log('Authentication successful');

    // Example: JWT Operations
    console.log('\n=== JWT Operations ===');
    const target_key = '02eec5685e141a8fc6ee91e3aad0556bdb4f7b8f3c8c8c8c8c8c8c8c8c8c8c8c8';
    const durationHours = 1; // 1 hour
    
    try {
      const token = await client.issueJwt(target_key, durationHours);
      console.log('Issued JWT token:', token);
      
      // Example: Verify the JWT token
      const claims = await client.verifyJwt(target_key, token);
      console.log('JWT claims:', claims);
      console.log('Target key:', claims.target_key);
    } catch (error) {
      console.error('JWT operation failed:', error);
    }

    // Example: Relay Management
    console.log('\n=== Relay Management ===');
    try {
      const relayUrl = 'wss://relay.damus.io';
      const addedRelay = await client.addRelay(relayUrl);
      console.log('Added relay:', addedRelay);
      
      const removedRelay = await client.removeRelay(relayUrl);
      console.log('Removed relay:', removedRelay);
    } catch (error) {
      console.error('Relay management failed:', error);
    }

    // Example 1: Authentication Flow
    console.log('\n=== Authentication Flow ===');
    const url = await client.newKeyHandshakeUrl((mainKey) => {
      console.log('Auth Init received for key:', mainKey);
      testFullFlow(client, mainKey, []);
    });
    console.log('Auth Init URL:', url);

    // Example 2: Calculate Next Occurrence
    console.log('\n=== Calculate Next Occurrence ===');
    const calendar = 'daily';
    const from = Timestamp.fromNow(0); // now
    const nextOccurrence = await client.calculateNextOccurrence(calendar, from);
    if (nextOccurrence) {
      // Print the next occurrence in a human readable format
      const nextOccurrenceDate = nextOccurrence.toDate();
      console.log('Next occurrence:', nextOccurrenceDate.toISOString());
    } else {
      console.log('No next occurrence found');
    }

    // Example 3: Fetch Nip05 Profile
    console.log('\n=== Fetch Nip05 Profile ===');
    const nip05 = 'unldenis@getportal.cc';
    const nip05Profile = await client.fetchNip05Profile(nip05);
    const pubkey = nip05Profile.public_key;
    console.log('Nip05 profile pubkey:', pubkey);

    // Example 4: Request Single Payment
    console.log('\n=== Request Single Payment ===');
    await client.requestSinglePayment(pubkey, [], {
      amount: 1000 * 3,
      currency: Currency.Millisats,
      description: "Test payment",
      request_id: "test-payment-2",
    }, (status) => {
      console.log('Payment status:', status);
    });
    console.log('Payment request sent');

    // Example 5: Get Wallet Info
    console.log('\n=== Get Wallet Info ===');
    const walletInfo = await client.getWalletInfo();
    console.log('Wallet info:', walletInfo);

    // Keep the connection alive and wait for user input
    console.log('\nConnection is active. Press Enter to disconnect...');
    
    await new Promise<void>((resolve) => {
      rl.question('', () => {
        resolve();
      });
    });

    // Clean up
    client.disconnect();
    rl.close();
    console.log('\nDisconnected and cleaned up');

  } catch (error) {
    console.error('Error in example:', error);
    client.disconnect();
    rl.close();
  }
}



// Run the example
main().catch(console.error); 