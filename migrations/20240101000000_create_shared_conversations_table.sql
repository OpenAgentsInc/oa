-- Create Shared Conversations Table
CREATE TABLE shared_conversations (
    id uuid NOT NULL PRIMARY KEY DEFAULT uuid_generate_v4(),
    chat_id TEXT NOT NULL,
    sender_npub TEXT NOT NULL,
    recipient_npub TEXT NOT NULL,
    message_count INTEGER NOT NULL,
    messages JSONB NOT NULL,
    share_type TEXT NOT NULL DEFAULT 'direct', -- direct, public, etc
    permissions TEXT NOT NULL DEFAULT 'read', -- read, write
    status TEXT NOT NULL DEFAULT 'active', -- active, revoked
    created_at timestamptz NOT NULL DEFAULT NOW(),
    revoked_at timestamptz,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    
    -- Add indexes for common queries
    CONSTRAINT valid_share_type CHECK (share_type IN ('direct', 'public')),
    CONSTRAINT valid_permissions CHECK (permissions IN ('read', 'write')),
    CONSTRAINT valid_status CHECK (status IN ('active', 'revoked'))
);

-- Add indexes for common queries
CREATE INDEX idx_shared_conversations_chat_id ON shared_conversations(chat_id);
CREATE INDEX idx_shared_conversations_sender ON shared_conversations(sender_npub);
CREATE INDEX idx_shared_conversations_recipient ON shared_conversations(recipient_npub);
CREATE INDEX idx_shared_conversations_status ON shared_conversations(status);