# Tana Mobile Authentication Protocol

## Vision

Enable users to authenticate on websites using their blockchain identity via mobile app, similar to WhatsApp Web. The flow:

1. User clicks "Login with Tana" on website
2. Website displays QR code
3. User scans QR code with Tana mobile app
4. User is automatically logged into website
5. Any blockchain transactions trigger confirmation dialogs on mobile app

## Architecture Overview

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│   Website    │◄────────┤  Auth Server │────────►│  Mobile App  │
│  (Browser)   │  SSE    │  (Ledger)    │ WebSocket│  (React Native)│
└──────────────┘         └──────────────┘         └──────────────┘
       │                        │                         │
       │ 1. Request login       │                         │
       │───────────────────────►│                         │
       │                        │                         │
       │ 2. Session + QR data   │                         │
       │◄───────────────────────│                         │
       │                        │                         │
       │ 3. Display QR code     │                         │
       │                        │                         │
       │                        │ 4. Scan QR code         │
       │                        │◄────────────────────────│
       │                        │                         │
       │                        │ 5. Request user data    │
       │                        │────────────────────────►│
       │                        │                         │
       │                        │ 6. Sign challenge       │
       │                        │◄────────────────────────│
       │                        │                         │
       │                        │ 7. Verify signature     │
       │                        │                         │
       │ 8. Session activated   │                         │
       │◄───────────────────────│                         │
       │                        │                         │
       │ 9. User logged in      │                         │
```

---

## Part 1: Login Flow (QR Code Authentication)

### Step 1: Website Requests Login Session

**Endpoint:** `POST /auth/session/create`

**Request:**
```json
{
  "returnUrl": "https://app.tana.network/dashboard",
  "appName": "Tana Dashboard",
  "appIcon": "https://app.tana.network/icon.png"
}
```

**Response:**
```json
{
  "sessionId": "sess_a1b2c3d4e5f6",
  "secret": "sec_x9y8z7w6v5u4",
  "qrData": {
    "sessionId": "sess_a1b2c3d4e5f6",
    "challenge": "auth_chal_1234567890abcdef",
    "serverUrl": "wss://ledger.tana.network",
    "expiresAt": 1700000300000
  },
  "pollUrl": "/auth/session/sess_a1b2c3d4e5f6/status",
  "expiresIn": 300
}
```

**QR Code Format:**
```
tana://auth?session=sess_a1b2c3d4e5f6&challenge=auth_chal_1234567890abcdef&server=wss://ledger.tana.network&expires=1700000300000
```

### Step 2: Website Displays QR Code

```typescript
// Website component
import QRCode from 'qrcode'

function LoginWithTana() {
  const [session, setSession] = useState(null)
  const [status, setStatus] = useState('waiting') // waiting, scanning, approved, expired

  useEffect(() => {
    // Create session
    fetch('/auth/session/create', {
      method: 'POST',
      body: JSON.stringify({
        returnUrl: window.location.href,
        appName: 'Tana App',
        appIcon: '/icon.png'
      })
    })
    .then(res => res.json())
    .then(data => {
      setSession(data)

      // Generate QR code
      const qrData = `tana://auth?session=${data.sessionId}&challenge=${data.qrData.challenge}&server=${data.qrData.serverUrl}&expires=${data.qrData.expiresAt}`

      QRCode.toDataURL(qrData).then(url => {
        // Display QR code
      })

      // Poll for status
      pollSessionStatus(data.sessionId)
    })
  }, [])

  async function pollSessionStatus(sessionId: string) {
    const eventSource = new EventSource(`/auth/session/${sessionId}/events`)

    eventSource.onmessage = (event) => {
      const data = JSON.parse(event.data)

      if (data.status === 'scanned') {
        setStatus('scanning')
      } else if (data.status === 'approved') {
        setStatus('approved')
        // Store session token
        localStorage.setItem('tana_session', data.sessionToken)
        // Redirect to app
        window.location.href = data.returnUrl
      } else if (data.status === 'expired') {
        setStatus('expired')
      }
    }
  }

  return (
    <div>
      {status === 'waiting' && (
        <>
          <h2>Scan with Tana App</h2>
          <img src={qrCodeUrl} />
        </>
      )}
      {status === 'scanning' && <p>QR code scanned, waiting for approval...</p>}
      {status === 'approved' && <p>Approved! Redirecting...</p>}
      {status === 'expired' && <p>Session expired. Please refresh.</p>}
    </div>
  )
}
```

### Step 3: Mobile App Scans QR Code

```typescript
// Mobile app (React Native)
import { Camera } from 'expo-camera'
import { parseQRCode } from '@/utils/qr'

function QRScanner() {
  async function handleBarCodeScanned({ data }: { data: string }) {
    // Parse QR code
    const qrData = parseQRCode(data) // tana://auth?session=...

    if (qrData.type === 'auth') {
      // Show authentication dialog
      navigation.navigate('AuthConfirm', {
        sessionId: qrData.session,
        challenge: qrData.challenge,
        serverUrl: qrData.server
      })
    }
  }

  return (
    <Camera
      onBarCodeScanned={handleBarCodeScanned}
      barCodeTypes={[BarCodeScanner.Constants.BarCodeType.qr]}
    />
  )
}
```

### Step 4: Mobile App Authenticates User

```typescript
// Mobile app: AuthConfirm screen
function AuthConfirm({ route }) {
  const { sessionId, challenge, serverUrl } = route.params
  const currentUser = useCurrentUser() // Get logged-in blockchain user

  async function handleApprove() {
    // Sign the challenge with user's private key
    const message = JSON.stringify({
      sessionId,
      challenge,
      userId: currentUser.id,
      username: currentUser.username,
      timestamp: Date.now()
    })

    const signature = await signMessage(message, currentUser.privateKey)

    // Send authentication to server
    await fetch(`${serverUrl}/auth/session/${sessionId}/approve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        userId: currentUser.id,
        username: currentUser.username,
        publicKey: currentUser.publicKey,
        signature,
        message,
        timestamp: Date.now()
      })
    })

    // Show success
    Alert.alert('Success', 'You are now logged in on the web app')
  }

  return (
    <View>
      <Text>Login Request</Text>
      <Text>App: Tana Dashboard</Text>
      <Text>Session: {sessionId}</Text>
      <Button title="Approve" onPress={handleApprove} />
      <Button title="Reject" onPress={handleReject} />
    </View>
  )
}
```

### Step 5: Server Verifies and Establishes Session

**Endpoint:** `POST /auth/session/:sessionId/approve`

```typescript
// Ledger service: auth/session route
app.post('/auth/session/:sessionId/approve', async (c) => {
  const { sessionId } = c.req.param()
  const body = c.req.valid('json')

  // 1. Get session from database
  const session = await db
    .select()
    .from(authSessions)
    .where(eq(authSessions.id, sessionId))
    .limit(1)

  if (!session || session.status !== 'waiting') {
    return c.json({ error: 'Invalid session' }, 400)
  }

  // 2. Check expiration
  if (Date.now() > session.expiresAt) {
    return c.json({ error: 'Session expired' }, 400)
  }

  // 3. Verify signature
  const messageToVerify = body.message
  const isValid = await verifySignature(
    messageToVerify,
    body.signature,
    body.publicKey
  )

  if (!isValid) {
    return c.json({ error: 'Invalid signature' }, 403)
  }

  // 4. Verify user exists on blockchain
  const user = await db
    .select()
    .from(users)
    .where(eq(users.id, body.userId))
    .limit(1)

  if (!user || user.publicKey !== body.publicKey) {
    return c.json({ error: 'User not found' }, 404)
  }

  // 5. Generate session token
  const sessionToken = generateSecureToken()

  // 6. Update session
  await db
    .update(authSessions)
    .set({
      status: 'approved',
      userId: body.userId,
      sessionToken,
      approvedAt: new Date()
    })
    .where(eq(authSessions.id, sessionId))

  // 7. Notify web app via SSE
  notifySessionApproved(sessionId, {
    status: 'approved',
    sessionToken,
    userId: body.userId,
    username: body.username,
    returnUrl: session.returnUrl
  })

  return c.json({ success: true })
})
```

---

## Part 2: Transaction Signing Flow

Once authenticated, the user is logged into the website. When they perform a blockchain action (transfer, contract call, etc.), the website needs mobile app approval.

### Step 1: Website Initiates Transaction

```typescript
// Website: Transfer page
async function handleTransfer(to: string, amount: string) {
  // Create transaction request
  const response = await fetch('/auth/transaction/request', {
    method: 'POST',
    headers: {
      'Authorization': `Bearer ${sessionToken}`,
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({
      type: 'transfer',
      to,
      amount,
      currencyCode: 'USD'
    })
  })

  const { requestId } = await response.json()

  // Poll for mobile approval
  const eventSource = new EventSource(`/auth/transaction/${requestId}/events`)

  eventSource.onmessage = (event) => {
    const data = JSON.parse(event.data)

    if (data.status === 'approved') {
      // Transaction signed and submitted
      alert('Transaction submitted!')
      eventSource.close()
    } else if (data.status === 'rejected') {
      alert('Transaction rejected')
      eventSource.close()
    }
  }
}
```

### Step 2: Server Notifies Mobile App

**Push Notification:**
```json
{
  "type": "transaction_approval",
  "requestId": "txreq_abc123",
  "transaction": {
    "type": "transfer",
    "from": "usr_alice",
    "to": "usr_bob",
    "amount": "100",
    "currencyCode": "USD"
  }
}
```

### Step 3: Mobile App Shows Approval Dialog

```typescript
// Mobile app: Push notification handler
usePushNotifications({
  onNotification: (notification) => {
    if (notification.data.type === 'transaction_approval') {
      // Show transaction approval screen
      navigation.navigate('TransactionApproval', {
        requestId: notification.data.requestId,
        transaction: notification.data.transaction
      })
    }
  }
})

// TransactionApproval screen
function TransactionApproval({ route }) {
  const { requestId, transaction } = route.params
  const currentUser = useCurrentUser()

  async function handleApprove() {
    // Get user's current nonce
    const nonce = await getUserNonce(currentUser.id)

    // Create signed transaction
    const txData = {
      type: transaction.type,
      from: currentUser.id,
      to: transaction.to,
      amount: transaction.amount,
      currencyCode: transaction.currencyCode,
      timestamp: Date.now(),
      nonce: nonce + 1
    }

    const message = createTransactionMessage(txData)
    const signature = await signMessage(message, currentUser.privateKey)

    // Submit transaction
    await fetch('/transactions', {
      method: 'POST',
      body: JSON.stringify({
        ...txData,
        signature
      })
    })

    // Notify approval
    await fetch(`/auth/transaction/${requestId}/approve`, {
      method: 'POST',
      body: JSON.stringify({
        approved: true,
        transactionId: txData.txId
      })
    })

    Alert.alert('Success', 'Transaction submitted to blockchain')
  }

  return (
    <View>
      <Text>Transaction Approval</Text>
      <Text>Type: {transaction.type}</Text>
      <Text>To: {transaction.to}</Text>
      <Text>Amount: {transaction.amount} {transaction.currencyCode}</Text>
      <Button title="Approve & Sign" onPress={handleApprove} />
      <Button title="Reject" onPress={handleReject} />
    </View>
  )
}
```

---

## Database Schema

### Auth Sessions Table

```sql
CREATE TABLE auth_sessions (
  id TEXT PRIMARY KEY,              -- sess_a1b2c3d4e5f6
  challenge TEXT NOT NULL,          -- Random challenge for signature
  status TEXT NOT NULL,             -- waiting, scanned, approved, rejected, expired
  user_id TEXT,                     -- User who approved (from blockchain)
  session_token TEXT,               -- JWT or secure token for website
  return_url TEXT,                  -- URL to redirect after auth
  app_name TEXT,                    -- Name of requesting app
  app_icon TEXT,                    -- Icon URL
  created_at TIMESTAMP DEFAULT NOW(),
  expires_at TIMESTAMP NOT NULL,
  approved_at TIMESTAMP,
  scanned_at TIMESTAMP
);

CREATE INDEX idx_auth_sessions_status ON auth_sessions(status);
CREATE INDEX idx_auth_sessions_expires ON auth_sessions(expires_at);
```

### Transaction Requests Table

```sql
CREATE TABLE transaction_requests (
  id TEXT PRIMARY KEY,              -- txreq_abc123
  session_id TEXT NOT NULL REFERENCES auth_sessions(id),
  user_id TEXT NOT NULL,
  transaction_type TEXT NOT NULL,   -- transfer, contract_call, etc.
  transaction_data JSONB NOT NULL,  -- Full transaction details
  status TEXT NOT NULL,             -- pending, approved, rejected, expired
  transaction_id TEXT,              -- Actual blockchain tx ID after approval
  created_at TIMESTAMP DEFAULT NOW(),
  expires_at TIMESTAMP NOT NULL,
  responded_at TIMESTAMP
);

CREATE INDEX idx_tx_requests_session ON transaction_requests(session_id);
CREATE INDEX idx_tx_requests_user ON transaction_requests(user_id);
CREATE INDEX idx_tx_requests_status ON transaction_requests(status);
```

---

## API Endpoints

### Authentication Endpoints

```
POST   /auth/session/create           Create new login session
GET    /auth/session/:id/status       Get session status (polling)
GET    /auth/session/:id/events       SSE stream for real-time updates
POST   /auth/session/:id/approve      Mobile app approves login
POST   /auth/session/:id/reject       Mobile app rejects login
DELETE /auth/session/:id              Logout/invalidate session
```

### Transaction Request Endpoints

```
POST   /auth/transaction/request      Create transaction approval request
GET    /auth/transaction/:id/status   Get request status
GET    /auth/transaction/:id/events   SSE stream for approval status
POST   /auth/transaction/:id/approve  Mobile app approves transaction
POST   /auth/transaction/:id/reject   Mobile app rejects transaction
```

### Session Management Endpoints

```
GET    /auth/sessions                 List active sessions for user
DELETE /auth/sessions/:id             Revoke specific session
DELETE /auth/sessions                 Revoke all sessions
```

---

## Security Considerations

### 0. Mobile-Only Private Keys (Critical Design Decision)

**Why NO browser extension or desktop storage:**

Desktop and laptop computers are significantly more vulnerable to:
- **Malware and viruses**: Keyloggers, clipboard hijackers, memory scrapers
- **Browser exploits**: XSS attacks, malicious extensions
- **Physical access**: Easier to steal/clone hard drives
- **Supply chain attacks**: Compromised software packages

**Mobile devices are inherently more secure:**
- **Hardware security**: Secure Enclave (iOS) / StrongBox (Android)
- **OS sandboxing**: Apps cannot access other apps' data
- **Biometric protection**: Face ID, Touch ID, fingerprint
- **Limited attack surface**: Fewer processes, tighter permissions
- **Physical security**: Users carry phones, harder to physically compromise

**This is why banks use mobile apps for 2FA, not browser extensions.**

**Design Principles:**
1. ✅ **Private keys ONLY exist on mobile device**
2. ✅ **Mobile device is the hardware wallet**
3. ✅ **Desktop/laptop is read-only interface**
4. ✅ **All signing operations require mobile approval**
5. ❌ **NO MetaMask-style browser extensions**
6. ❌ **NO private key export to desktop**
7. ❌ **NO "convenience" options that compromise security**

**User Experience:**
- Slightly more friction (mobile approval required)
- Significantly better security (keys never leave secure device)
- Peace of mind (desktop compromise ≠ identity theft)

**This is not just "more secure" - it's the ONLY secure way to handle blockchain identities on the web.**

---

### 1. Challenge-Response Authentication

**Problem:** Prevent replay attacks

**Solution:** Each session includes a random challenge that must be signed

```typescript
// Server generates challenge
const challenge = crypto.randomBytes(32).toString('hex')

// Mobile app signs challenge + session data
const message = JSON.stringify({
  sessionId,
  challenge,  // Must match server's challenge
  userId,
  timestamp
})
const signature = sign(message, privateKey)
```

### 2. Session Expiration

- QR codes expire after 5 minutes
- Unused sessions are garbage collected
- Active sessions expire after 30 days (configurable)

### 3. Rate Limiting

```typescript
// Limit session creation per IP
rateLimiter.limit('session-create', {
  ip: req.ip,
  max: 10,
  window: '1m'
})

// Limit approval attempts per session
rateLimiter.limit('session-approve', {
  sessionId,
  max: 3,
  window: '5m'
})
```

### 4. HTTPS/WSS Only

All communication must use encrypted channels:
- Website: HTTPS
- WebSocket: WSS
- API: HTTPS

### 5. Session Token Security

```typescript
// Generate cryptographically secure tokens
function generateSessionToken() {
  return `tana_session_${crypto.randomBytes(32).toString('base64url')}`
}

// Store securely
// Option 1: JWT with RS256 signing
const token = jwt.sign({ userId, sessionId }, privateKey, {
  algorithm: 'RS256',
  expiresIn: '30d'
})

// Option 2: Secure random token stored in database
await db.insert(sessions).values({
  token,
  userId,
  expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000)
})
```

---

## Implementation Phases

### Phase 1: Basic Login Flow (MVP)
- [x] QR code generation on website
- [ ] QR code scanning in mobile app
- [ ] Session creation API
- [ ] Session approval API
- [ ] SSE polling for web app
- [ ] Basic session management

### Phase 2: Transaction Signing
- [ ] Transaction request API
- [ ] Push notifications to mobile app
- [ ] Transaction approval UI on mobile
- [ ] Transaction signing on mobile
- [ ] Automatic transaction submission

### Phase 3: Advanced Features
- [ ] Multi-device support
- [ ] Session management dashboard
- [ ] Device nicknames and icons
- [ ] Biometric confirmation on mobile
- [ ] Transaction history per session

### Phase 4: Production Hardening
- [ ] Rate limiting
- [ ] Comprehensive logging
- [ ] Session analytics
- [ ] Security auditing
- [ ] Automated session cleanup

---

## Technology Stack

### Website
- **QR Code Generation:** `qrcode` npm package
- **Real-time Updates:** Server-Sent Events (SSE)
- **HTTP Client:** `fetch` API
- **State Management:** React hooks

### Mobile App (React Native)
- **QR Scanning:** `expo-camera` or `react-native-qrcode-scanner`
- **Push Notifications:** `expo-notifications` or Firebase Cloud Messaging
- **Crypto:** `@noble/ed25519` (same as CLI)
- **Storage:** `@react-native-async-storage/async-storage`

### Server (Ledger Service)
- **Framework:** Hono (existing)
- **Database:** PostgreSQL (existing)
- **Real-time:** Server-Sent Events
- **WebSocket:** Optional for bi-directional communication

---

## Example User Flows

### Flow 1: First-time Website Login

1. User visits https://app.tana.network
2. Clicks "Login with Tana"
3. QR code appears
4. Opens Tana mobile app
5. Taps "Scan QR Code"
6. Camera opens, scans code
7. App shows: "Login to Tana Dashboard?"
8. User taps "Approve"
9. Website automatically logs in
10. User is redirected to dashboard

### Flow 2: Making a Transfer

1. User (already logged in) navigates to "Send"
2. Enters recipient and amount
3. Clicks "Send"
4. Mobile app notification: "Transaction approval needed"
5. User opens mobile app
6. Sees transaction details
7. Taps "Approve & Sign"
8. Uses biometric (Face ID/Touch ID)
9. Transaction signed and submitted
10. Website shows "Transaction pending"
11. After block confirmation: "Transfer complete"

### Flow 3: Deploying a Smart Contract

1. User on website creates contract
2. Clicks "Deploy"
3. Mobile notification appears
4. User reviews contract code hash
5. Approves deployment
6. Contract deployed to blockchain
7. Website shows contract ID

---

## Migration from Old Device Flow

The old `/cli` page and `add_device.ts` can be deprecated:

**Old Flow:**
```
CLI → Generate 6-char code → User enters on website → Approve
```

**New Flow:**
```
Website → Generate QR code → User scans with mobile → Approve
```

**Benefits:**
- No manual code entry (better UX)
- Works for websites (not just CLI)
- Mobile app controls all private keys (more secure)
- Can extend to transaction signing (not just login)

**Migration Path:**
1. Keep old `/cli` route for backward compatibility
2. Add new `/login` route with QR code
3. Encourage users to upgrade to mobile app
4. Eventually deprecate manual code entry

---

## Next Steps

1. **Create API routes** in ledger service (`/auth/session/*`)
2. **Add database tables** (auth_sessions, transaction_requests)
3. **Implement QR code generation** on website
4. **Build mobile app QR scanner**
5. **Implement signature verification** on server
6. **Add SSE polling** for real-time updates
7. **Build transaction approval** flow

Would you like me to start implementing any specific part?
