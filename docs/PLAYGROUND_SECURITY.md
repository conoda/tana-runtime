# Playground Security & Deployment Model

## Overview

The Tana Playground is a **read-only simulation environment** designed for:
- Testing contract validity
- Educational purposes to learn Tana APIs
- Safe experimentation without affecting production blockchain state

## Security Model

### Current Implementation (Development)

**Iframe Sandbox:** No sandbox attribute (removed for localStorage access)
- ✅ User code runs in isolated iframe context
- ✅ Fetch is whitelisted to specific domains
- ✅ No access to parent window from user code
- ✅ Only exposed APIs available via `tanaModules`

**Protections in Place:**
1. **Domain Whitelist:** Fetch requests restricted to approved domains only
2. **API Exposure Control:** Only `tana:*` modules exposed, no direct access to DOM/Window APIs from parent
3. **Read-Only Execution:** `tx.execute()` simulates but doesn't persist changes

### Production Security Recommendations

For production deployment, consider these additional security measures:

#### Option 1: Separate Origin (Recommended)
Deploy sandbox to a separate subdomain:
- **Main site:** `tana.dev` or `tana.network`
- **Sandbox:** `sandbox.tana.dev` or `playground.tana.dev`

This provides:
- Stronger isolation between parent and iframe
- Separate cookie/storage domains
- Better Content Security Policy (CSP) enforcement

#### Option 2: Re-enable Sandbox with Storage Bridge
Add back sandbox attribute and use `postMessage` for storage:
```html
<iframe sandbox="allow-scripts" src="/sandbox"></iframe>
```

Then implement storage via parent-child communication:
- User code calls `data.set()` in sandbox
- Sandbox sends `postMessage` to parent
- Parent stores in localStorage
- Sandbox receives confirmation

This provides:
- Maximum isolation
- Still allows localStorage inspection in DevTools (on parent)
- Prevents any direct DOM access from user code

#### Option 3: Content Security Policy
Add strict CSP headers to sandbox page:
```
Content-Security-Policy:
  default-src 'none';
  script-src 'self' https://unpkg.com;
  connect-src https://api.tana.dev https://pokeapi.co;
  style-src 'unsafe-inline';
```

## Read-Only Architecture

### What Reads from Blockchain

The playground makes **read-only API calls** to the ledger service:

```typescript
// ✅ READ-ONLY: Fetches current state
await block.getBalance(userId, 'USD')  // GET /balances
await block.getUser(userId)            // GET /users
await block.getTransaction(txId)       // GET /transactions
```

These calls are safe in production because they:
- Only perform GET requests
- Query current blockchain state
- Don't modify any data
- Subject to rate limiting on API side

### What Simulates (No Real Writes)

The playground **simulates** transaction execution:

```typescript
// ✅ SIMULATION ONLY: No actual POST/PUT requests
tx.transfer('alice', 'bob', 100, 'USD')  // Staged in memory
await tx.execute()                        // Returns mock result, no API call
```

The `tx.execute()` function:
- Validates proposed changes locally
- Calculates gas usage
- Returns success/failure result
- **Does NOT** make any POST/PUT/PATCH requests to the API
- **Does NOT** persist to blockchain

### Comparison: Playground vs Runtime

| Feature | Playground (Browser) | Runtime (Rust CLI) |
|---------|---------------------|-------------------|
| **Read State** | ✅ Real API calls | ✅ Real API calls |
| **Validate Contract** | ✅ Full validation | ✅ Full validation |
| **Calculate Gas** | ✅ Simulated | ✅ Real gas metering |
| **Write to Blockchain** | ❌ Simulation only | ✅ Real persistence |
| **Data Storage** | localStorage (session) | In-memory or Redis |

## Environment Configuration

The sandbox automatically detects the environment:

```javascript
// Development: localhost or *.pages.dev
const API_BASE_URL = 'http://localhost:8080'

// Production: any other domain
const API_BASE_URL = 'https://api.tana.dev'
```

Console output shows current configuration:
```
[Tana] Environment: PRODUCTION
[Tana] API: https://api.tana.dev
[Tana] Mode: READ-ONLY SIMULATION (no actual state changes)
```

## API Endpoints

### Development (Localhost)
- Ledger API: `http://localhost:8080`
- Endpoints: `/balances`, `/users`, `/transactions`

### Production
- Ledger API: `https://api.tana.dev` (to be deployed)
- Same endpoints as development
- Rate limiting applied
- CORS configured for `tana.dev`, `tana.network`

## Data Storage

### Development
- Uses real `localStorage` when available
- Falls back to in-memory polyfill if blocked
- Inspectable via DevTools Application tab or `window.__tanaDataStore`

### Production
- Same behavior as development
- Data persists per session in browser
- Not synchronized with blockchain (simulation only)
- Cleared when user refreshes page (if using polyfill)

## Known Limitations

1. **No Real Persistence:** Playground changes don't persist to blockchain
2. **Session-Only Storage:** Data storage is browser session only
3. **Rate Limits:** API calls subject to rate limiting on backend
4. **Batch Query Limits:** Max 10 items per query to prevent abuse

## Future Improvements

1. **Separate Sandbox Origin:** Deploy to `sandbox.tana.dev`
2. **CSP Headers:** Add Content-Security-Policy enforcement
3. **Execution Quotas:** Add per-user execution limits
4. **Analytics:** Track contract patterns and common errors
5. **Sandboxed Workers:** Consider using Web Workers for additional isolation

## Testing

### Development Testing
```bash
cd website
npm run dev
# Visit http://localhost:4322
```

### Production Testing
```bash
cd website
npm run build
npm run preview
# Visit http://localhost:4321
```

Check console for environment detection:
- Should show correct API URL
- Should indicate READ-ONLY mode

### Security Testing Checklist
- [ ] Verify fetch is whitelisted (try non-whitelisted domain)
- [ ] Verify tx.execute() doesn't POST to API (check Network tab)
- [ ] Verify user code can't access parent window
- [ ] Verify localStorage works or polyfill is used
- [ ] Verify rate limiting on API side
- [ ] Verify CORS is properly configured

## Deployment Checklist

Before deploying to production:

1. **API Endpoints**
   - [ ] Deploy ledger API to `https://api.tana.dev`
   - [ ] Configure CORS for playground domain
   - [ ] Set up rate limiting
   - [ ] Test all GET endpoints

2. **Security**
   - [ ] Review iframe security (consider separate origin)
   - [ ] Add CSP headers if using current implementation
   - [ ] Test fetch whitelist
   - [ ] Verify no actual writes occur

3. **Monitoring**
   - [ ] Set up API request monitoring
   - [ ] Track error rates
   - [ ] Monitor for abuse patterns

4. **Documentation**
   - [ ] Update user-facing docs with "simulation only" notice
   - [ ] Add examples showing difference between playground and runtime
   - [ ] Document API rate limits

## Questions?

See also:
- `/docs/FEATURE_PARITY.md` - Feature parity between playground and runtime
- `/docs/CONTRACT_EXECUTION.md` - Contract execution model
- `/runtime/BLOCK_TX_IMPLEMENTATION.md` - Rust runtime implementation
