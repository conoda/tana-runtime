-- Tana Identity Service - Initial Schema
-- Mobile authentication sessions and device management

-- Create enums
CREATE TYPE auth_session_status AS ENUM ('waiting', 'scanned', 'approved', 'rejected', 'expired');
CREATE TYPE transaction_request_status AS ENUM ('pending', 'approved', 'rejected', 'expired');

-- Auth Sessions Table
CREATE TABLE auth_sessions (
  id TEXT PRIMARY KEY,
  challenge TEXT NOT NULL UNIQUE,
  status auth_session_status NOT NULL DEFAULT 'waiting',

  -- User info from blockchain (set after approval)
  user_id TEXT,
  username TEXT,
  public_key TEXT,

  -- Session token
  session_token TEXT UNIQUE,

  -- App info
  return_url TEXT NOT NULL,
  app_name TEXT,
  app_icon TEXT,

  -- Timestamps
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMP NOT NULL,
  approved_at TIMESTAMP,
  scanned_at TIMESTAMP
);

-- Indexes for auth_sessions
CREATE INDEX idx_auth_sessions_status ON auth_sessions(status);
CREATE INDEX idx_auth_sessions_expires ON auth_sessions(expires_at);
CREATE INDEX idx_auth_sessions_user ON auth_sessions(user_id);
CREATE INDEX idx_auth_sessions_token ON auth_sessions(session_token);
CREATE INDEX idx_auth_sessions_created ON auth_sessions(created_at DESC);

-- Transaction Requests Table
CREATE TABLE transaction_requests (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES auth_sessions(id),
  user_id TEXT NOT NULL,

  -- Transaction details
  transaction_type TEXT NOT NULL,
  transaction_data TEXT NOT NULL,

  -- Status
  status transaction_request_status NOT NULL DEFAULT 'pending',

  -- Result
  transaction_id TEXT,

  -- Timestamps
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMP NOT NULL,
  responded_at TIMESTAMP
);

-- Indexes for transaction_requests
CREATE INDEX idx_tx_requests_session ON transaction_requests(session_id);
CREATE INDEX idx_tx_requests_user ON transaction_requests(user_id);
CREATE INDEX idx_tx_requests_status ON transaction_requests(status);
CREATE INDEX idx_tx_requests_created ON transaction_requests(created_at DESC);

-- Device Tokens Table
CREATE TABLE device_tokens (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL,

  -- Push notification token
  push_token TEXT NOT NULL UNIQUE,

  -- Device info
  device_name TEXT,
  platform TEXT,
  app_version TEXT,

  -- Active status
  is_active TEXT NOT NULL DEFAULT 'true',

  -- Timestamps
  created_at TIMESTAMP NOT NULL DEFAULT NOW(),
  last_used_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indexes for device_tokens
CREATE INDEX idx_device_tokens_user ON device_tokens(user_id);
CREATE INDEX idx_device_tokens_active ON device_tokens(is_active);
CREATE INDEX idx_device_tokens_push ON device_tokens(push_token);

-- Comments
COMMENT ON TABLE auth_sessions IS 'Mobile QR code authentication sessions';
COMMENT ON TABLE transaction_requests IS 'Pending transaction approvals from mobile app';
COMMENT ON TABLE device_tokens IS 'Push notification tokens for mobile devices';
