# Sync Engine Design

## Overview

The sync engine enables real-time data synchronization between the Onyx mobile app and OpenAgents.com backend using WebSocket connections authenticated with Nostr keys.

## Authentication Flow

1. User connects via WebSocket from Onyx app
2. Initial handshake includes signed message with user's Nostr pubkey
3. Server verifies signature and establishes authenticated session
4. All subsequent messages are signed using the same key

## Message Protocol

### Connection Establishment
```typescript
// Client -> Server
interface ConnectMessage {
  type: "connect";
  pubkey: string;  // npub...
  timestamp: number;
  signature: string;  // Signed {pubkey, timestamp} 
}

// Server -> Client
interface ConnectResponse {
  type: "connect_ack";
  session_id: string;
  timestamp: number;
  signature: string;  // Server signs {session_id, timestamp}
}
```

### Data Sync Messages
```typescript
interface SyncMessage {
  type: "sync";
  session_id: string;
  timestamp: number;
  signature: string;
  payload: {
    entity: "chat" | "settings" | "training_data";
    action: "create" | "update" | "delete";
    data: any;
    version: number;
  }
}
```

## Sync Entities

### Chats
- Full chat history
- Message metadata
- Sharing permissions
- Training data opt-in status

### User Settings 
- Preferences
- Pro subscription status
- Feature flags
- UI customizations

### Training Data
- Contributed messages
- Quality ratings
- Reward status
- Usage permissions

## Versioning & Conflict Resolution

1. Each entity maintains a version number
2. Server is source of truth for version conflicts
3. Client sends current version with updates
4. Server rejects updates with outdated versions
5. Client must fetch latest before retrying update

## Error Handling

1. Connection drops
   - Client queues changes locally
   - Automatic reconnection with exponential backoff
   - Replay missed changes on reconnect

2. Version conflicts
   - Server returns current version
   - Client fetches latest
   - Merges changes if possible
   - Prompts user for resolution if needed

3. Invalid signatures
   - Connection terminated
   - Client must re-authenticate

## Implementation Notes

### Server (Rust)
```rust
// In src/routes/websocket.rs
pub struct WebSocketConnection {
    pub session_id: String,
    pub pubkey: String,
    pub last_seen: DateTime<Utc>,
    pub pending_messages: Vec<SyncMessage>,
}

impl WebSocketConnection {
    pub fn verify_signature(&self, msg: &SyncMessage) -> bool {
        // Verify Nostr signature
    }
    
    pub fn handle_message(&mut self, msg: SyncMessage) -> Result<()> {
        // Process incoming sync message
    }
}
```

### Client (React Native)
```typescript
// In services/sync.ts
class SyncService {
  private ws: WebSocket;
  private queue: SyncMessage[] = [];
  
  constructor(private nostrKeys: NostrKeys) {}
  
  async connect() {
    this.ws = new WebSocket('wss://openagents.com/sync');
    // Setup handlers & auth
  }
  
  async syncEntity(entity: string, action: string, data: any) {
    // Queue & send sync message
  }
}
```

## Security Considerations

1. All messages signed with Nostr keys
2. TLS for transport security
3. Session IDs rotated periodically
4. Rate limiting per connection
5. Message size limits
6. Timeout for inactive connections

## Future Enhancements

1. Batch sync operations
2. Compression for large payloads
3. Partial sync for large datasets
4. Multiple device sync
5. Offline-first capabilities
6. End-to-end encryption option

## Testing Strategy

1. Unit tests for message handling
2. Integration tests for sync flows
3. Stress tests for concurrent connections
4. Chaos testing for network issues
5. Security audit of auth flow