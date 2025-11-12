# Mobile Authentication Implementation Plan

## Overview

Build a secure, mobile-first authentication system where:
- **Mobile app = hardware wallet** (private keys never leave mobile)
- **Desktop/laptop = read-only interface** (no private keys stored)
- **QR code authentication** (like WhatsApp Web)
- **Mobile transaction approval** (all signing on mobile)

**Reference:** See `/docs/MOBILE_AUTH_PROTOCOL.md` for complete specification.

---

## Phase 1: Basic Login Flow (MVP)

### 1.1 Backend: Session Management API

**Location:** `cli/services/ledger/src/api/routes/auth.ts` (new file)

**Tasks:**
- [ ] Create `auth_sessions` table in PostgreSQL
  ```sql
  CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    challenge TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'waiting',
    user_id TEXT,
    session_token TEXT,
    return_url TEXT,
    app_name TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    approved_at TIMESTAMP
  );
  ```

- [ ] Add migration: `cli/services/ledger/migrations/0009_auth_sessions.sql`

- [ ] Implement `POST /auth/session/create`
  - Generate session ID
  - Generate challenge (random 32-byte hex)
  - Return QR data
  - Set 5-minute expiration

- [ ] Implement `GET /auth/session/:id/events` (Server-Sent Events)
  - Stream session status updates
  - Notify when scanned
  - Notify when approved/rejected

- [ ] Implement `POST /auth/session/:id/approve`
  - Verify Ed25519 signature of challenge
  - Validate user exists on blockchain
  - Generate session token
  - Update session status
  - Notify SSE listeners

- [ ] Implement `POST /auth/session/:id/reject`
  - Mark session as rejected
  - Notify SSE listeners

**Files to create:**
```
cli/services/ledger/
├── src/
│   ├── api/routes/auth.ts          # New auth endpoints
│   ├── auth/
│   │   ├── session.ts               # Session management logic
│   │   └── tokens.ts                # Token generation/verification
├── migrations/
│   └── 0009_auth_sessions.sql      # Database schema
```

**Testing:**
```bash
# Create session
curl -X POST http://localhost:8080/auth/session/create \
  -H "Content-Type: application/json" \
  -d '{"returnUrl":"http://localhost:3000/dashboard","appName":"Test App"}'

# Approve session (with signed challenge)
curl -X POST http://localhost:8080/auth/session/sess_123/approve \
  -H "Content-Type: application/json" \
  -d '{"userId":"usr_alice","signature":"ed25519_...","message":"..."}'
```

---

### 1.2 Frontend: QR Code Login Page

**Location:** `websites/landing/app/login/page.tsx` (new page)

**Tasks:**
- [ ] Install QR code library: `npm install qrcode`
- [ ] Create login page component
- [ ] Implement session creation on page load
- [ ] Generate and display QR code
- [ ] Set up SSE connection for real-time updates
- [ ] Handle session states: waiting → scanned → approved
- [ ] Redirect to app after approval
- [ ] Handle expiration (show "Refresh" button)

**Example Component:**
```typescript
'use client'
import { useState, useEffect } from 'react'
import QRCode from 'qrcode'

export default function LoginPage() {
  const [qrCode, setQrCode] = useState<string>('')
  const [status, setStatus] = useState<'loading' | 'waiting' | 'scanned' | 'approved'>('loading')

  useEffect(() => {
    async function initSession() {
      // Create session
      const res = await fetch('/api/auth/session/create', {
        method: 'POST',
        body: JSON.stringify({
          returnUrl: '/dashboard',
          appName: 'Tana App'
        })
      })
      const data = await res.json()

      // Generate QR code
      const qrData = `tana://auth?session=${data.sessionId}&challenge=${data.qrData.challenge}&server=${window.location.origin}`
      const qrImage = await QRCode.toDataURL(qrData)
      setQrCode(qrImage)
      setStatus('waiting')

      // Listen for updates
      const eventSource = new EventSource(`/api/auth/session/${data.sessionId}/events`)
      eventSource.onmessage = (event) => {
        const update = JSON.parse(event.data)
        setStatus(update.status)

        if (update.status === 'approved') {
          localStorage.setItem('tana_session', update.sessionToken)
          window.location.href = '/dashboard'
        }
      }
    }

    initSession()
  }, [])

  return (
    <div className="flex flex-col items-center justify-center min-h-screen">
      <h1 className="text-3xl font-bold mb-8">Login with Tana</h1>

      {status === 'waiting' && (
        <>
          <p className="mb-4">Scan this QR code with your Tana mobile app</p>
          <img src={qrCode} alt="QR Code" className="w-64 h-64" />
        </>
      )}

      {status === 'scanned' && (
        <p className="text-lg">QR code scanned! Please approve on your mobile device...</p>
      )}

      {status === 'approved' && (
        <p className="text-lg text-green-600">Approved! Redirecting...</p>
      )}
    </div>
  )
}
```

**Files to create:**
```
websites/landing/
├── app/
│   ├── login/
│   │   └── page.tsx                 # QR code login page
│   ├── api/
│   │   └── auth/
│   │       └── [...path]/route.ts   # Proxy to ledger API
```

---

### 1.3 Mobile App: QR Scanner

**Location:** `mobile/` (new React Native app)

**Initial Setup:**
```bash
# Create React Native app with Expo
npx create-expo-app@latest mobile --template blank-typescript

cd mobile

# Install dependencies
npx expo install expo-camera expo-barcode-scanner
npm install @noble/ed25519
npm install @react-native-async-storage/async-storage
```

**Project Structure:**
```
mobile/
├── app/
│   ├── index.tsx                    # Main app screen
│   ├── qr-scanner.tsx               # QR code scanner
│   ├── auth-confirm.tsx             # Login confirmation
│   └── transaction-approval.tsx     # Transaction approval (Phase 2)
├── components/
│   └── QRScanner.tsx                # Reusable QR scanner component
├── utils/
│   ├── crypto.ts                    # Ed25519 signing utilities
│   ├── storage.ts                   # Secure key storage
│   └── api.ts                       # API client
├── types/
│   └── index.ts                     # TypeScript types
```

**Tasks:**
- [ ] Set up Expo project with TypeScript
- [ ] Request camera permissions
- [ ] Build QR scanner component
- [ ] Parse `tana://auth?...` QR code format
- [ ] Create authentication confirmation screen
- [ ] Implement Ed25519 signing
- [ ] Send signed approval to server
- [ ] Show success/error feedback

**Example QR Scanner:**
```typescript
import { Camera } from 'expo-camera'
import { useState } from 'react'

export default function QRScannerScreen() {
  const [hasPermission, setHasPermission] = useState<boolean | null>(null)

  useEffect(() => {
    (async () => {
      const { status } = await Camera.requestCameraPermissionsAsync()
      setHasPermission(status === 'granted')
    })()
  }, [])

  async function handleBarCodeScanned({ data }: { data: string }) {
    // Parse QR code: tana://auth?session=...&challenge=...
    const url = new URL(data)

    if (url.protocol === 'tana:' && url.pathname === '//auth') {
      const params = url.searchParams
      const sessionId = params.get('session')
      const challenge = params.get('challenge')
      const serverUrl = params.get('server')

      // Navigate to confirmation screen
      router.push({
        pathname: '/auth-confirm',
        params: { sessionId, challenge, serverUrl }
      })
    }
  }

  if (hasPermission === null) {
    return <Text>Requesting camera permission...</Text>
  }

  if (hasPermission === false) {
    return <Text>No access to camera</Text>
  }

  return (
    <Camera
      style={{ flex: 1 }}
      onBarCodeScanned={handleBarCodeScanned}
    >
      <View style={styles.overlay}>
        <Text style={styles.instructions}>Scan QR code to login</Text>
      </View>
    </Camera>
  )
}
```

**Example Auth Confirmation:**
```typescript
import { signMessage } from '@/utils/crypto'
import { getCurrentUser } from '@/utils/storage'

export default function AuthConfirmScreen({ route }) {
  const { sessionId, challenge, serverUrl } = route.params
  const [user, setUser] = useState(null)

  useEffect(() => {
    getCurrentUser().then(setUser)
  }, [])

  async function handleApprove() {
    // Create message to sign
    const message = JSON.stringify({
      sessionId,
      challenge,
      userId: user.id,
      username: user.username,
      timestamp: Date.now()
    })

    // Sign with user's private key
    const signature = await signMessage(message, user.privateKey)

    // Send to server
    const response = await fetch(`${serverUrl}/auth/session/${sessionId}/approve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        userId: user.id,
        username: user.username,
        publicKey: user.publicKey,
        signature,
        message
      })
    })

    if (response.ok) {
      Alert.alert('Success', 'You are now logged in!')
      router.back()
    } else {
      Alert.alert('Error', 'Failed to authenticate')
    }
  }

  return (
    <View style={styles.container}>
      <Text style={styles.title}>Login Request</Text>
      <Text>Session: {sessionId}</Text>
      <Text>User: {user?.username}</Text>
      <Button title="Approve" onPress={handleApprove} />
      <Button title="Reject" onPress={handleReject} />
    </View>
  )
}
```

---

## Phase 2: Transaction Signing

### 2.1 Backend: Transaction Request API

**Location:** `cli/services/ledger/src/api/routes/auth.ts` (extend)

**Tasks:**
- [ ] Create `transaction_requests` table
  ```sql
  CREATE TABLE transaction_requests (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES auth_sessions(id),
    user_id TEXT NOT NULL,
    transaction_type TEXT NOT NULL,
    transaction_data JSONB NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',
    transaction_id TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL
  );
  ```

- [ ] Add migration: `cli/services/ledger/migrations/0010_transaction_requests.sql`

- [ ] Implement `POST /auth/transaction/request`
  - Validate session token
  - Create transaction request
  - Send push notification to mobile
  - Return request ID

- [ ] Implement `GET /auth/transaction/:id/events` (SSE)
  - Stream approval status
  - Notify when approved/rejected

- [ ] Implement `POST /auth/transaction/:id/approve`
  - Submit signed transaction to blockchain
  - Update request status
  - Notify SSE listeners

---

### 2.2 Mobile App: Push Notifications

**Tasks:**
- [ ] Set up Expo push notifications
- [ ] Request notification permissions
- [ ] Store device push token on server
- [ ] Handle incoming transaction requests
- [ ] Show transaction approval screen
- [ ] Sign and submit transactions

**Example:**
```typescript
import * as Notifications from 'expo-notifications'

// Register for push notifications
export async function registerForPushNotifications() {
  const { status } = await Notifications.requestPermissionsAsync()

  if (status === 'granted') {
    const token = (await Notifications.getExpoPushTokenAsync()).data

    // Send to server
    await fetch(`${API_URL}/auth/device/register`, {
      method: 'POST',
      body: JSON.stringify({ pushToken: token })
    })
  }
}

// Handle notifications
Notifications.addNotificationReceivedListener(notification => {
  if (notification.request.content.data.type === 'transaction_approval') {
    // Navigate to approval screen
    router.push('/transaction-approval')
  }
})
```

---

## Phase 3: Production Hardening

### 3.1 Security Enhancements

- [ ] Implement rate limiting on all auth endpoints
- [ ] Add IP-based blocking for abuse
- [ ] Implement session token rotation
- [ ] Add audit logging for all auth events
- [ ] Implement device fingerprinting
- [ ] Add CAPTCHA for session creation (optional)

### 3.2 User Experience

- [ ] Add session management dashboard
- [ ] Show list of active devices/sessions
- [ ] Allow revoking specific sessions
- [ ] Add device nicknames and icons
- [ ] Implement biometric confirmation on mobile
- [ ] Add transaction history per session

### 3.3 Monitoring & Analytics

- [ ] Track authentication success/failure rates
- [ ] Monitor session creation volume
- [ ] Alert on suspicious patterns
- [ ] Log transaction approval latencies
- [ ] Track mobile app versions in use

---

## Development Workflow

### Step 1: Set Up Development Environment

```bash
# Terminal 1: Start ledger service
cd cli/services/ledger
DATABASE_URL='postgres://tana:tana_dev_password@localhost:5432/tana' bun run dev

# Terminal 2: Start website
cd websites/landing
npm run dev

# Terminal 3: Start mobile app
cd mobile
npx expo start
```

### Step 2: Test Locally

1. Open website at http://localhost:3000/login
2. QR code appears
3. Open mobile app on phone (via Expo Go)
4. Scan QR code
5. Approve authentication
6. Website logs in

### Step 3: Deploy

```bash
# Deploy ledger service (backend)
# Already deployed at ledger.tana.network:8080

# Deploy website (frontend)
# Already deployed at tana.network

# Build and publish mobile app
cd mobile
eas build --platform ios
eas submit --platform ios
```

---

## Testing Strategy

### Unit Tests

```typescript
// Test session creation
describe('POST /auth/session/create', () => {
  it('creates session with valid QR data', async () => {
    const res = await fetch('/auth/session/create', {
      method: 'POST',
      body: JSON.stringify({ returnUrl: '/dashboard' })
    })
    const data = await res.json()

    expect(data.sessionId).toMatch(/^sess_/)
    expect(data.qrData.challenge).toHaveLength(64)
    expect(data.expiresIn).toBe(300)
  })
})

// Test signature verification
describe('POST /auth/session/:id/approve', () => {
  it('approves with valid signature', async () => {
    const message = JSON.stringify({ sessionId, challenge, userId })
    const signature = await signMessage(message, privateKey)

    const res = await fetch(`/auth/session/${sessionId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ signature, message, publicKey })
    })

    expect(res.status).toBe(200)
  })

  it('rejects with invalid signature', async () => {
    const res = await fetch(`/auth/session/${sessionId}/approve`, {
      method: 'POST',
      body: JSON.stringify({ signature: 'invalid', message, publicKey })
    })

    expect(res.status).toBe(403)
  })
})
```

### Integration Tests

```typescript
// End-to-end authentication flow
describe('QR Code Authentication', () => {
  it('completes full login flow', async () => {
    // 1. Create session
    const session = await createSession()

    // 2. Simulate mobile app scanning
    const qrData = parseQRCode(session.qrData)

    // 3. Sign challenge
    const signature = await signChallenge(qrData.challenge, userPrivateKey)

    // 4. Approve session
    await approveSession(session.sessionId, signature)

    // 5. Verify session is active
    const sessionStatus = await getSessionStatus(session.sessionId)
    expect(sessionStatus.status).toBe('approved')
  })
})
```

---

## Rollout Plan

### Week 1: Backend Foundation
- Database schema
- Auth API endpoints
- SSE implementation
- Basic testing

### Week 2: Web Frontend
- QR code generation
- Login page
- SSE integration
- Error handling

### Week 3: Mobile App MVP
- Project setup
- QR scanner
- Authentication flow
- Basic UI

### Week 4: Integration & Testing
- End-to-end testing
- Bug fixes
- Performance optimization
- Security audit

### Week 5: Transaction Signing (Phase 2)
- Transaction request API
- Push notifications
- Mobile approval UI
- Integration testing

### Week 6: Polish & Deploy
- UI/UX improvements
- Documentation
- Beta testing
- Production deployment

---

## Success Criteria

### MVP (Phase 1) Complete When:
- [x] User can scan QR code and login to website
- [x] Private keys remain only on mobile device
- [x] Session persists across page refreshes
- [x] Session expires after timeout
- [x] Multiple concurrent sessions work

### Phase 2 Complete When:
- [ ] User can approve transactions from mobile
- [ ] Transactions are signed on mobile only
- [ ] Push notifications work reliably
- [ ] Transaction history is tracked

### Production Ready When:
- [ ] Rate limiting implemented
- [ ] Security audit passed
- [ ] Load testing completed
- [ ] Documentation complete
- [ ] Mobile app published to stores

---

## Next Immediate Steps

1. **Create auth_sessions migration**
2. **Implement POST /auth/session/create endpoint**
3. **Build basic QR code login page**
4. **Set up mobile app project**
5. **Implement QR scanner in mobile app**

Would you like me to start implementing any of these? I can begin with the backend API endpoints since those are foundational.
