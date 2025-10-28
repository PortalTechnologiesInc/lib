# Frequently Asked Questions

## General Questions

### What is Portal?

Portal is a toolkit for businesses to process payments, authenticate users, issue tickets and much more. Portal is based on freedom tech (Nostr, Lightning Network and Cashu) all without intermediaries.

### Do users need a Nostr account?

Yes, users need a nostr key to interact with businesses using Portal. A key is generated automatically by the Portal app, or it can be imported.

### Is Portal free?

Portal is free and open-source (MIT license). 

## Technical Questions

### Can I use Portal without Docker?

Yes! You can build and run from source using Cargo. See [Building from Source](../getting-started/building-from-source.md).

### Do I need to run a Lightning node?

Not necessarily. You can use Nostr Wallet Connect (NWC) with a hosted wallet service like Alby, or use the built-in wallet powered by the Breez SDK.

### How do I handle user sessions?

Use JWT tokens issued by Portal for session management. See [JWT Tokens Guide](../guides/jwt-tokens.md).

## Payment Questions

### What happens if a payment fails?

The user receives a status update, and you can handle it in your callback. No funds are lost.

### Can I issue refunds?

Yes, but you'll need to initiate a reverse payment to the user's Lightning wallet.

### How long do payments take?

Lightning fast.

## Security Questions

### Is Portal secure?

Portal uses cryptographic signatures for authentication and doesn't handle private keys.

### Where are private keys stored?

Your Portal instance has its own private key. User private keys are stored in the secure storage and never leave their devices

### Can users be tracked?

Portal is designed with privacy in mind. Nostr relays don't require registration, and Lightning payments don't expose personal information.

## Troubleshooting

### "Connection refused" error

- Check Portal daemon is running: `docker ps`
- Verify correct port (default: 3000)
- Check firewall settings

### Users can't authenticate

- Verify users have a compatible Nostr wallet
- Check relay connectivity
- Ensure NOSTR_KEY is set correctly

### Payments not working

- Verify NWC_URL is configured
- Check wallet has sufficient balance
- Test wallet connectivity separately

---

**Need more help?** Check [Troubleshooting Guide](../advanced/troubleshooting.md)

