# Sync Engine Design

## Overview

The sync engine enables real-time data synchronization between the Onyx mobile app and OpenAgents.com backend using WebSocket connections authenticated with Nostr keys, following NIP-42 for authentication and NIP-01 for event handling.

## Authentication Flow (NIP-42)

1. Client connects via WebSocket to `wss://openagents.com/sync`
2. Server sends challenge: `["AUTH", "<random-challenge>"]`
3. Client creates and signs auth event:
```json
{
  "kind": 22242,
  "created_at": <timestamp>,
  "tags": [
    ["relay", "wss://openagents.com/sync"],
    ["challenge", "<challenge-string>"]
  ],
  "pubkey": "<user-npub>",
  // other fields per NIP-01
}
```
4. Client sends: `["AUTH", <signed-event-json>]`
5. Server verifies and responds: `["OK", <event-id>, true, ""]`

## Sync Events (NIP-01)

### Event Types
```typescript
interface SyncEvent {
  kind: number;  // Using Nostr kind ranges
  pubkey: string;
  created_at: number;
  content: string;
  tags: string[][];
  id: string;
  sig: string;
}

// Event kinds:
const KINDS = {
  CHAT_UPDATE: 30001,      // Addressable chat updates
  SETTINGS_UPDATE: 10001,  // Replaceable settings
  TRAINING_DATA: 1,        // Regular training data contributions
}
```

### Chat Updates
```json
{
  "kind": 30001,
  "content": "{\"messages\": [...], \"metadata\": {...}}",
  "tags": [
    ["d", "<chat-id>"],
    ["p", "<recipient-pubkey>", "<recommended-relay>"]
  ]
}
```

### Settings Updates
```json
{
  "kind": 10001,
  "content": "{\"preferences\": {...}, \"features\": [...]}",
  "tags": [
    ["d", "settings"]
  ]
}
```

### Training Data
```json
{
  "kind": 1,
  "content": "<message-content>",
  "tags": [
    ["t", "training"],
    ["quality", "<rating>"]
  ]
}
```

## Subscription Flow

1. Client subscribes to relevant events:
```json
["REQ", "sub1", {
  "authors": ["<user-pubkey>"],
  "kinds": [30001, 10001, 1],
  "#d": ["<chat-id>", "settings"]
}]
```

2. Server streams matching events:
```json
["EVENT", "sub1", {
  "kind": 30001,
  "content": "...",
  // ...other event fields
}]
```

3. Server indicates end of stored events:
```json
["EOSE", "sub1"]
```

## Implementation Notes

### Server (Rust)
```rust
pub struct SyncConnection {
    pub session_id: String,
    pub pubkey: String,
    pub subscriptions: HashMap<String, Filter>,
    pub last_seen: DateTime<Utc>,
}

impl SyncConnection {
    pub fn handle_auth(&mut self, event: Event) -> Result<()> {
        // Verify NIP-42 auth event
        if event.kind != 22242 {
            return Err(Error::InvalidAuth);
        }
        // Verify challenge, relay URL, timestamp
        // Set authenticated state
    }
    
    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        // Validate event signature (NIP-01)
        // Process based on kind
        match event.kind {
            30001 => self.handle_chat_update(event),
            10001 => self.handle_settings_update(event),
            1 => self.handle_training_data(event),
            _ => Err(Error::UnsupportedKind)
        }
    }
}
```

### Client (React Native)
```typescript
class SyncService {
  private ws: WebSocket;
  private subscriptions: Map<string, Filter>;
  
  constructor(private nostrKeys: NostrKeys) {}
  
  async connect() {
    this.ws = new WebSocket('wss://openagents.com/sync');
    this.ws.onmessage = this.handleMessage;
    await this.authenticate();
  }
  
  private async authenticate() {
    // Handle NIP-42 auth flow
    const challenge = await this.waitForChallenge();
    const authEvent = this.createAuthEvent(challenge);
    const signedEvent = await this.nostrKeys.signEvent(authEvent);
    this.ws.send(['AUTH', signedEvent]);
  }
  
  async syncChat(chatId: string, data: any) {
    const event = {
      kind: 30001,
      content: JSON.stringify(data),
      tags: [
        ['d', chatId],
        // other tags...
      ],
      created_at: Math.floor(Date.now() / 1000)
    };
    const signedEvent = await this.nostrKeys.signEvent(event);
    this.ws.send(['EVENT', signedEvent]);
  }
}
```

## Security Considerations

1. All messages signed with Nostr keys (NIP-01)
2. Auth challenges per NIP-42
3. TLS for transport security
4. Rate limiting per pubkey
5. Event size limits
6. Subscription limits per connection

## Error Handling

1. Connection drops:
   - Queue changes locally
   - Exponential backoff reconnection
   - Resubscribe on reconnect
   - Replay missed events

2. Auth failures:
   - Invalid challenge response
   - Expired timestamps
   - Invalid signatures
   - Rate limits

3. Event validation:
   - Invalid kinds
   - Schema validation
   - Permission checks
   - Version conflicts

## Future Enhancements

1. Batch operations (multiple events)
2. Compression for large payloads
3. Partial sync filters
4. Multi-device sync
5. Offline-first capabilities
6. End-to-end encryption
7. Relay federation