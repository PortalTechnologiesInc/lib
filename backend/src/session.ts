import Database from 'better-sqlite3';
import path from 'path';

export interface Session {
  id: string;
  publicKey: string;
  displayName: string;
  authToken: string;
}

export interface Payment {
  id: string;
  publicKey: string;
  amount: number;
  description: string;
  status: 'pending' | 'completed' | 'failed';
  createdAt: number;
  updatedAt: number;
}

export interface Subscription {
  id: string;
  publicKey: string;
  amount: number;
  frequency: string;
  status: 'pending' |'active' | 'cancelled' | 'failed';
  lastPaymentAt: number | null;
  nextPaymentAt: number;
  createdAt: number;
  authToken: string;
  portalSubscriptionId: string | null;
}

export class DatabaseManager {
  private db: Database.Database;

  constructor() {
    this.db = new Database(process.env.DATABASE_PATH || path.join(__dirname, '../sessions.db'));
    this.initDatabase();
  }

  private initDatabase() {
    // Create sessions table if it doesn't exist
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY,
        public_key TEXT NOT NULL,
        display_name TEXT NOT NULL,
        auth_token TEXT NOT NULL,
        created_at INTEGER NOT NULL DEFAULT (unixepoch())
      );

      CREATE TABLE IF NOT EXISTS payments (
        id TEXT PRIMARY KEY,
        public_key TEXT NOT NULL,
        amount INTEGER NOT NULL,
        description TEXT NOT NULL,
        status TEXT NOT NULL CHECK(status IN ('pending', 'completed', 'failed')),
        created_at INTEGER NOT NULL DEFAULT (unixepoch()),
        updated_at INTEGER NOT NULL DEFAULT (unixepoch())
      );

      CREATE TABLE IF NOT EXISTS subscriptions (
        id TEXT PRIMARY KEY,
        public_key TEXT NOT NULL,
        amount INTEGER NOT NULL,
        frequency TEXT NOT NULL,
        status TEXT NOT NULL CHECK(status IN ('active', 'cancelled', 'failed')),
        last_payment_at INTEGER,
        next_payment_at INTEGER NOT NULL,
        created_at INTEGER NOT NULL DEFAULT (unixepoch()),
        auth_token TEXT NOT NULL,
        portal_subscription_id TEXT
      );

      -- Add indexes for common queries
      CREATE INDEX IF NOT EXISTS idx_payments_public_key ON payments(public_key);
      CREATE INDEX IF NOT EXISTS idx_payments_status ON payments(status);
      CREATE INDEX IF NOT EXISTS idx_subscriptions_public_key ON subscriptions(public_key);
      CREATE INDEX IF NOT EXISTS idx_subscriptions_status ON subscriptions(status);
      CREATE INDEX IF NOT EXISTS idx_subscriptions_next_payment ON subscriptions(next_payment_at);
      CREATE INDEX IF NOT EXISTS idx_sessions_public_key ON sessions(public_key);
    `);
  }

  // Session methods
  public setSession(session: Session): void {
    const stmt = this.db.prepare(`
      INSERT OR REPLACE INTO sessions (id, public_key, display_name, auth_token)
      VALUES (@id, @publicKey, @displayName, @authToken)
    `);
    stmt.run(session);
  }

  public getSession(id: string): Session | undefined {
    const stmt = this.db.prepare('SELECT * FROM sessions WHERE id = ?');
    const row = stmt.get(id) as any;
    
    if (!row) return undefined;
    
    return {
      id: row.id,
      publicKey: row.public_key,
      displayName: row.display_name,
      authToken: row.auth_token
    };
  }

  public hasSession(id: string): boolean {
    const stmt = this.db.prepare('SELECT 1 FROM sessions WHERE id = ?');
    return stmt.get(id) !== undefined;
  }

  public deleteSession(id: string): void {
    const stmt = this.db.prepare('DELETE FROM sessions WHERE id = ?');
    stmt.run(id);
  }

  // Payment methods
  public createPayment(payment: Omit<Payment, 'createdAt' | 'updatedAt'>): Payment {
    const stmt = this.db.prepare(`
      INSERT INTO payments (id, public_key, amount, description, status)
      VALUES (@id, @publicKey, @amount, @description, @status)
    `);
    stmt.run(payment);
    return this.getPayment(payment.id)!;
  }

  public getPayment(id: string): Payment | undefined {
    const stmt = this.db.prepare('SELECT * FROM payments WHERE id = ?');
    const row = stmt.get(id) as any;
    
    if (!row) return undefined;
    
    return {
      id: row.id,
      publicKey: row.public_key,
      amount: row.amount,
      description: row.description,
      status: row.status,
      createdAt: row.created_at,
      updatedAt: row.updated_at
    };
  }

  public updatePaymentStatus(id: string, status: Payment['status']): void {
    const stmt = this.db.prepare(`
      UPDATE payments 
      SET status = ?, updated_at = unixepoch()
      WHERE id = ?
    `);
    stmt.run(status, id);
  }

  public getPublicKeyPayments(publicKey: string): Payment[] {
    const stmt = this.db.prepare('SELECT * FROM payments WHERE public_key = ? ORDER BY created_at DESC');
    const rows = stmt.all(publicKey) as any[];
    
    return rows.map(row => ({
      id: row.id,
      publicKey: row.public_key,
      amount: row.amount,
      description: row.description,
      status: row.status,
      createdAt: row.created_at,
      updatedAt: row.updated_at
    }));
  }

  // Subscription methods
  public createSubscription(subscription: Omit<Subscription, 'createdAt'>): Subscription {
    const stmt = this.db.prepare(`
      INSERT INTO subscriptions (
        id, public_key, amount, frequency, status, 
        last_payment_at, next_payment_at, auth_token,
        portal_subscription_id
      )
      VALUES (
        @id, @publicKey, @amount, @frequency, @status,
        @lastPaymentAt, @nextPaymentAt, @authToken,
        @portalSubscriptionId
      )
    `);
    stmt.run(subscription);
    return this.getSubscription(subscription.id)!;
  }

  public getSubscription(id: string): Subscription | undefined {
    const stmt = this.db.prepare('SELECT * FROM subscriptions WHERE id = ?');
    const row = stmt.get(id) as any;
    
    if (!row) return undefined;
    
    return {
      id: row.id,
      publicKey: row.public_key,
      amount: row.amount,
      frequency: row.frequency,
      status: row.status,
      lastPaymentAt: row.last_payment_at,
      nextPaymentAt: row.next_payment_at,
      createdAt: row.created_at,
      authToken: row.auth_token,
      portalSubscriptionId: row.portal_subscription_id
    };
  }

  public getSubscriptionPortalId(portalSubscriptionId: string): Subscription | undefined {
    const stmt = this.db.prepare('SELECT * FROM subscriptions WHERE status = \'active\' AND portal_subscription_id = ?');
    const row = stmt.get(portalSubscriptionId) as any;
    
    if (!row) return undefined;
    
    return {
      id: row.id,
      publicKey: row.public_key,
      amount: row.amount,
      frequency: row.frequency,
      status: row.status,
      lastPaymentAt: row.last_payment_at,
      nextPaymentAt: row.next_payment_at,
      createdAt: row.created_at,
      authToken: row.auth_token,
      portalSubscriptionId: row.portal_subscription_id
    };
  }

  public updateSubscriptionStatus(id: string, status: Subscription['status'], nextPaymentAt?: number, portalSubscriptionId?: string): void {
    const stmt = this.db.prepare(`
      UPDATE subscriptions 
      SET status = ?, 
          last_payment_at = CASE WHEN ? = 1 THEN unixepoch() ELSE last_payment_at END,
          next_payment_at = COALESCE(?, next_payment_at),
          portal_subscription_id = COALESCE(?, portal_subscription_id)
      WHERE id = ?
    `);
    stmt.run(status, status === 'active' ? 1 : 0, nextPaymentAt, portalSubscriptionId, id);
  }

  public getPublicKeySubscriptions(publicKey: string): Subscription[] {
    const stmt = this.db.prepare('SELECT * FROM subscriptions WHERE public_key = ? ORDER BY created_at DESC');
    const rows = stmt.all(publicKey) as any[];
    
    return rows.map(row => ({
      id: row.id,
      publicKey: row.public_key,
      amount: row.amount,
      frequency: row.frequency,
      status: row.status,
      lastPaymentAt: row.last_payment_at,
      nextPaymentAt: row.next_payment_at,
      createdAt: row.created_at,
      authToken: row.auth_token,
      portalSubscriptionId: row.portal_subscription_id
    }));
  }

  public getDueSubscriptions(now: number): Subscription[] {
    const stmt = this.db.prepare(`
      SELECT * FROM subscriptions 
      WHERE status = 'active' 
      AND next_payment_at < ?
      ORDER BY next_payment_at ASC
    `);
    const rows = stmt.all(now) as any[];
    
    return rows.map(row => ({
      id: row.id,
      publicKey: row.public_key,
      amount: row.amount,
      frequency: row.frequency,
      status: row.status,
      lastPaymentAt: row.last_payment_at,
      nextPaymentAt: row.next_payment_at,
      createdAt: row.created_at,
      authToken: row.auth_token,
      portalSubscriptionId: row.portal_subscription_id
    }));
  }

  public cleanup(): void {
    const db = this.db;
    db.transaction(() => {
      // Delete sessions older than 24 hours
      const deleteOldSessions = db.prepare(`
        DELETE FROM sessions 
        WHERE created_at < unixepoch() - 86400
      `);
      
      // Mark old pending payments as failed
      const cleanupPendingPayments = db.prepare(`
        UPDATE payments 
        SET status = 'failed', updated_at = unixepoch()
        WHERE status = 'pending' 
        AND created_at < unixepoch() - 3600
      `);
      
      // Mark failed subscriptions
      const cleanupSubscriptions = db.prepare(`
        UPDATE subscriptions 
        SET status = 'failed'
        WHERE status = 'active' 
        AND next_payment_at < unixepoch() - 86400
      `);
      
      deleteOldSessions.run();
      cleanupPendingPayments.run();
      cleanupSubscriptions.run();
    })();
  }
} 
