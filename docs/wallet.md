# Wallet Integration Plan

## Overview
Integration of Bitcoin Lightning Network wallet functionality using the Breez SDK for handling payments and transactions.

## Components

### 1. Wallet Core (Rust)
- Integration with Breez SDK
- Secure key management
- Transaction handling
- Database integration for transaction history

### 2. Admin API Endpoints
- `POST /api/wallet/pay` - Process Lightning invoice payments
- `GET /api/wallet/transactions` - Retrieve transaction history
- `GET /api/wallet/balance` - Get current wallet balance

### 3. Transaction History Storage
- PostgreSQL table for storing transaction records
- Fields:
  - transaction_id (UUID)
  - amount_sats (i64)
  - timestamp (DateTime)
  - payment_hash (String)
  - status (enum: Success/Failed)
  - pseudonymized_recipient (String)
  - memo (Optional<String>)

### 4. Transaction List Web Interface
- Display paginated transaction history
- Show pseudonymized transaction data
- Filtering and sorting capabilities
- Export functionality (optional)

## Security Considerations
- Admin-only access to payment endpoints
- Secure storage of wallet credentials
- Rate limiting on API endpoints
- Audit logging for all wallet operations

## Implementation Phases

### Phase 1: Core Integration
1. Add Breez SDK dependency
2. Implement wallet initialization
3. Create database schema for transactions
4. Basic payment handling

### Phase 2: API Layer
1. Implement admin authentication
2. Create REST endpoints
3. Add transaction logging
4. Error handling

### Phase 3: Web Interface
1. Transaction list page
2. Admin payment interface
3. Basic analytics/reporting

## Open Questions
- Authentication mechanism for admin access
- Specific fields to pseudonymize in transaction history
- Integration points with existing admin interface
- Monitoring and alerting requirements
- Backup and recovery procedures

## Dependencies
- Breez SDK
- PostgreSQL
- Actix-web (existing)
- Additional security libraries (TBD)

## Next Steps
1. Finalize security requirements
2. Set up Breez SDK integration
3. Create database migrations
4. Implement core payment logic
5. Build admin API endpoints
6. Develop web interface