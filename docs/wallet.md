# Wallet Integration Plan

## Overview
Integration of Bitcoin Lightning Network wallet functionality using the Breez SDK for handling payments and transactions.

## Core Components

### 1. Wallet Core (Rust)
```rust
// Key structures from Breez SDK
use breez_sdk_liquid::{
    BreezServices,
    SendPaymentRequest,
    SendPaymentResponse,
    Payment,
    PaymentDetails,
    ListPaymentsRequest,
};
```

### 2. Dependencies
```toml
[dependencies]
breez-sdk-liquid = { git = "https://github.com/breez/breez-sdk-liquid", branch = "main" }
sdk-common = { git = "https://github.com/breez/breez-sdk", rev = "f77208acd34d74b571388889e856444908c59a85", features = ["liquid"] }
```

### 3. Environment Configuration
```env
BREEZ_API_KEY=your_api_key
ADMIN_API_KEY=your_admin_key  # for securing admin endpoints
```

### 4. Database Schema
```sql
CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_hash TEXT NOT NULL,
    amount_sats BIGINT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status TEXT NOT NULL,  -- Using PaymentState from Breez: Created/Pending/Complete/Failed
    direction TEXT NOT NULL, -- Incoming/Outgoing
    bolt11 TEXT,  -- The lightning invoice
    description TEXT,
    receiver_pubkey TEXT,  -- pseudonymized identifier
    metadata JSONB  -- additional payment data
);
```

### 5. API Endpoints

#### Admin Payment Endpoint
```
POST /api/wallet/pay
Authorization: Bearer {ADMIN_API_KEY}

{
    "bolt11": "lnbc...",  // Lightning invoice
    "description": "Optional payment description"
}
```

#### Transaction List Endpoint
```
GET /api/wallet/transactions
Authorization: Bearer {ADMIN_API_KEY}

Response:
{
    "transactions": [
        {
            "id": "uuid",
            "amount": 1000,  // in sats
            "timestamp": "2024-01-20T12:00:00Z",
            "status": "Complete",
            "direction": "Outgoing",
            "description": "Payment for service",
            "receiver": "02abc..."  // first 6 chars of pubkey
        }
    ]
}
```

### 6. Transaction List UI
Simple HTML page following existing site style:

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <link rel="stylesheet" href="style.css" />
    <link href="https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400&display=swap" rel="stylesheet">
    <title>OpenAgents - Wallet Transactions</title>
</head>
<body>
    <canvas id="bg"></canvas>
    <div class="overlay"></div>
    <div class="container">
        <div class="card-wrapper">
            <article class="card">
                <header class="card-header">
                    <div class="card-left" aria-hidden="true"></div>
                    <h2 class="card-title">Lightning Transactions</h2>
                    <div class="card-right" aria-hidden="true"></div>
                </header>
                <section class="card-content">
                    <div id="transactions"></div>
                </section>
            </article>
        </div>
    </div>
    <script src="js/transactions.js"></script>
</body>
</html>
```

## Implementation Steps

1. Add Dependencies to Cargo.toml
2. Initialize Breez SDK with environment config
3. Create Database Migration
- Add payments table
- Add indexes for common queries
4. Implement Core Payment Logic
- Payment processing using Breez SDK
- Transaction logging
- Error handling for basic cases
5. Create API Endpoints
- Simple API key auth middleware
- Payment endpoint
- Transaction list endpoint
6. Build Transaction List Page
- Following existing site style
- Basic transaction display
- Simple filtering/sorting

## Data Privacy
- Store minimal required transaction data
- Pseudonymize receiver information by only showing first 6 chars of pubkey
- No personal information stored
- All amounts shown in sats only

## Security
- API key authentication for admin endpoints
- Environment variable configuration
- Rate limiting can be added later if needed
- No frontend payment functionality - admin API only

## Next Steps
1. Set up Breez SDK integration
2. Create database migration
3. Implement basic payment endpoint
4. Add transaction list page
5. Test with small amounts (~$1 worth of sats)

## Future Considerations
- Add monitoring/alerts
- Implement transaction limits
- Add more detailed error handling
- Enhance analytics/reporting
- Add backup procedures