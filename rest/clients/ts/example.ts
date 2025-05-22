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
      subscription_id: recurringStatus.status.subscription_id // Optional: link to subscription
    };

    const paymentResult = await client.requestSinglePayment(
      mainKey,
      subkeys,
      singlePayment,
      (status) => {
        console.log('Payment status update:', status);
      }
    );
    console.log('Initial payment status:', paymentResult.status);
    
    // Example 5: Fetch Profile
    console.log('\n=== Fetch Profile ===');
    const profile = await client.fetchProfile(mainKey);
    console.log('User profile:', profile);
}

async function main() {
  // Create a new client instance
  const client = new PortalSDK({
    serverUrl: 'ws://localhost:3000/ws',
    connectTimeout: 5000
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

    // Example 1: Authentication Flow
    console.log('\n=== Authentication Flow ===');
    const url = await client.newAuthInitUrl((mainKey) => {
      console.log('Auth Init received for key:', mainKey);
      testFullFlow(client, mainKey, []);
    });
    console.log('Auth Init URL:', url);

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