# Chat Sharing Functionality

This document describes the chat sharing implementation between Onyx and OA.

## Overview

The chat sharing feature allows users to share their chat conversations from the Onyx mobile app to other users via the OA backend server.

## Related Files

### Onyx (Mobile App)
- `app/screens/ShareScreen/ShareScreen.tsx`
  - React Native screen component for sharing chats
  - Handles user input for recipient
  - Makes POST request to OA server with chat data
  - Shows success/error feedback to user

### OA (Backend Server)
- `src/routes/chats.rs`
  - Main handler for chat sharing endpoint
  - Defines data structures for request payload
  - Implements custom header handling for Nostr pubkey
  - Currently logs received data (database implementation pending)

- `src/routes/mod.rs`
  - Exports chat routes
  - Makes routes available to the application

- `src/startup.rs`
  - Configures server routes
  - Mounts chat sharing endpoint at `/api/v1/chats/{chat_id}/share`

## API Endpoint

```
POST /api/v1/chats/{chat_id}/share
```

### Headers
- `Content-Type: application/json`
- `x-nostr-pubkey: <npub...>` (Nostr public key of sender)

### Request Body
```json
{
  "recipient": "npub...",
  "messages": [
    {
      "id": "string",
      "role": "user|assistant",
      "content": "string",
      "createdAt": 1234567890,
      "metadata": {
        // Optional message metadata
      }
    }
  ],
  "metadata": {
    "messageCount": 123,
    "timestamp": 1234567890
  }
}
```

### Response
```json
{
  "status": "success",
  "message": "Chat shared successfully"
}
```

## Current Status

- ✅ Basic endpoint structure implemented
- ✅ Request/response handling
- ✅ Nostr pubkey header parsing
- ✅ Input validation
- ✅ Basic logging
- ❌ Database storage (pending)
- ❌ Recipient notification (pending)
- ❌ Error handling improvements (pending)

## Next Steps

1. Implement database schema for shared conversations
2. Add proper validation of Nostr pubkeys
3. Implement database storage logic
4. Add error handling for various edge cases
5. Add recipient notification system
6. Add tests for the chat sharing endpoint

## Testing

You can test the endpoint using curl:

```bash
curl -X POST \
  -H "accept:application/json" \
  -H "content-type:application/json" \
  -H "x-nostr-pubkey:npub1cy7sv7ykvpp5n6pt4p05xw5gvdmuu6uesyum699jlz9uygh5akzscesy7c" \
  http://localhost:8000/api/v1/chats/chat_1735527232128/share \
  -d '{
    "recipient":"npub12345",
    "messages":[
      {
        "id":"qcl6nz0",
        "role":"user",
        "content":"Example message",
        "createdAt":1735527281194,
        "metadata":{"conversationId":"chat_1735527232128"}
      }
    ],
    "metadata":{
      "messageCount":1,
      "timestamp":1735527285172
    }
  }'
```