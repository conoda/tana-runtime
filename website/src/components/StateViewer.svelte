<script>
  import { onMount } from 'svelte';
  import { ledgerApi } from '../lib/ledgerApi';

  let activeTab = 'results';
  let sandboxIframe; // Internal iframe reference
  let blockchainState = null;
  let loading = true;
  let error = null;
  let lastRefresh = null;
  let transactionResult = null; // Last transaction execution result

  const tabs = [
    { id: 'results', label: 'Results', icon: '▶' },
    { id: 'transaction', label: 'Transaction', icon: '⚡' },
    { id: 'users', label: 'Users', icon: '👤' },
    { id: 'balances', label: 'Balances', icon: '💰' },
    { id: 'transactions', label: 'Transactions', icon: '💸' },
    { id: 'currencies', label: 'Currencies', icon: '💱' },
  ];

  async function refreshState() {
    loading = true;
    error = null;
    try {
      blockchainState = await ledgerApi.getBlockchainState();
      lastRefresh = new Date().toLocaleTimeString();
      loading = false;
    } catch (err) {
      error = err.message;
      loading = false;
    }
  }

  onMount(() => {
    refreshState();
    // Auto-refresh every 5 seconds
    const interval = setInterval(refreshState, 5000);

    // Listen for transaction execution results from sandbox
    const handleMessage = (event) => {
      if (event.data.type === 'transactionExecuted') {
        transactionResult = {
          success: event.data.success,
          changes: event.data.changes,
          gasUsed: event.data.gasUsed,
          timestamp: new Date().toLocaleTimeString()
        };
        // Auto-switch to transaction tab to show results
        activeTab = 'transaction';
      }
    };

    window.addEventListener('message', handleMessage);

    return () => {
      clearInterval(interval);
      window.removeEventListener('message', handleMessage);
    };
  });

  // Expose method to execute code in the sandbox
  export function executeCode(code) {
    if (sandboxIframe && sandboxIframe.contentWindow) {
      sandboxIframe.contentWindow.postMessage({
        type: 'execute',
        code: code
      }, '*');
    }
  }

  function formatAmount(amount) {
    return parseFloat(amount).toFixed(2);
  }

  function formatDate(dateString) {
    return new Date(dateString).toLocaleString();
  }
</script>

<div class="state-viewer">
  <!-- Tab Bar -->
  <div class="tab-bar">
    {#each tabs as tab}
      <button
        class="tab"
        class:active={activeTab === tab.id}
        on:click={() => activeTab = tab.id}
      >
        <span class="tab-icon">{tab.icon}</span>
        <span class="tab-label">{tab.label}</span>
      </button>
    {/each}

    <div class="tab-actions">
      <button class="refresh-btn" on:click={refreshState} title="Refresh state">
        <span class="refresh-icon">↻</span>
      </button>
      {#if lastRefresh}
        <span class="last-refresh">Updated: {lastRefresh}</span>
      {/if}
    </div>
  </div>

  <!-- Tab Content -->
  <div class="tab-content">
    <!-- Results panel - always present but hidden when not active -->
    <div class="results-panel" style="display: {activeTab === 'results' ? 'block' : 'none'};">
      <iframe
        bind:this={sandboxIframe}
        src="/sandbox"
        title="Tana Sandbox"
      ></iframe>
    </div>

    {#if activeTab === 'transaction'}
      <div class="data-panel">
        <div class="panel-header">
          <h3>Transaction Execution</h3>
        </div>
        {#if !transactionResult}
          <div class="empty-state">
            <p>No transaction executed yet</p>
            <p class="hint">Run a contract with tx.execute() to see results here</p>
          </div>
        {:else}
          <div class="transaction-result">
            <div class="result-header" class:success={transactionResult.success}>
              <span class="status-icon">{transactionResult.success ? '✓' : '✗'}</span>
              <span class="status-text">
                {transactionResult.success ? 'Transaction Successful' : 'Transaction Failed'}
              </span>
              <span class="timestamp">{transactionResult.timestamp}</span>
            </div>

            <div class="result-details">
              <div class="detail-row">
                <span class="label">Gas Used:</span>
                <span class="value mono">{transactionResult.gasUsed.toLocaleString()}</span>
              </div>

              {#if transactionResult.changes && transactionResult.changes.length > 0}
                <div class="changes-section">
                  <h4>State Changes ({transactionResult.changes.length})</h4>
                  {#each transactionResult.changes as change}
                    <div class="change-item">
                      {#if change.type === 'transfer'}
                        <div class="change-type">Transfer</div>
                        <div class="change-details">
                          <span class="mono">{change.from}</span>
                          <span class="arrow">→</span>
                          <span class="mono">{change.to}</span>
                          <span class="amount">{change.amount} {change.currency}</span>
                        </div>
                      {:else if change.type === 'balance_update'}
                        <div class="change-type">Balance Update</div>
                        <div class="change-details">
                          <span class="mono">{change.userId}</span>
                          <span class="amount">{change.amount} {change.currency}</span>
                        </div>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          </div>
        {/if}
      </div>
    {:else if activeTab !== 'results' && loading && !blockchainState}
      <div class="loading-state">
        <div class="spinner"></div>
        <p>Loading blockchain state...</p>
      </div>
    {:else if activeTab !== 'results' && error}
      <div class="error-state">
        <p class="error-icon">⚠️</p>
        <p class="error-message">{error}</p>
        <p class="error-hint">Make sure the ledger service is running on port 8080</p>
        <button on:click={refreshState}>Retry</button>
      </div>
    {:else if activeTab === 'users'}
      <div class="data-panel">
        <div class="panel-header">
          <h3>Users ({blockchainState?.users?.length || 0})</h3>
        </div>
        <div class="data-list">
          {#if blockchainState?.users?.length === 0}
            <div class="empty-state">
              <p>No users yet</p>
              <p class="hint">Create users via the ledger API or playground</p>
            </div>
          {:else}
            {#each blockchainState?.users || [] as user}
              <div class="data-item">
                <div class="item-header">
                  <span class="username">{user.username}</span>
                  <span class="display-name">{user.displayName}</span>
                </div>
                <div class="item-details">
                  <div class="detail-row">
                    <span class="label">ID:</span>
                    <span class="value mono">{user.id}</span>
                  </div>
                  <div class="detail-row">
                    <span class="label">Public Key:</span>
                    <span class="value mono">{user.publicKey}</span>
                  </div>
                  {#if user.bio}
                    <div class="detail-row">
                      <span class="label">Bio:</span>
                      <span class="value">{user.bio}</span>
                    </div>
                  {/if}
                  <div class="detail-row">
                    <span class="label">Created:</span>
                    <span class="value">{formatDate(user.createdAt)}</span>
                  </div>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {:else if activeTab === 'balances'}
      <div class="data-panel">
        <div class="panel-header">
          <h3>Balances ({blockchainState?.balances?.length || 0})</h3>
        </div>
        <div class="data-list">
          {#if blockchainState?.balances?.length === 0}
            <div class="empty-state">
              <p>No balances yet</p>
              <p class="hint">Set balances via the ledger API</p>
            </div>
          {:else}
            {#each blockchainState?.balances || [] as balance}
              <div class="data-item balance-item">
                <div class="balance-header">
                  <span class="amount">{formatAmount(balance.amount)} {balance.currencyCode}</span>
                  <span class="owner-type">{balance.ownerType}</span>
                </div>
                <div class="item-details">
                  <div class="detail-row">
                    <span class="label">Owner ID:</span>
                    <span class="value mono">{balance.ownerId}</span>
                  </div>
                  <div class="detail-row">
                    <span class="label">Updated:</span>
                    <span class="value">{formatDate(balance.updatedAt)}</span>
                  </div>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {:else if activeTab === 'transactions'}
      <div class="data-panel">
        <div class="panel-header">
          <h3>Transactions ({blockchainState?.transactions?.length || 0})</h3>
        </div>
        <div class="data-list">
          {#if blockchainState?.transactions?.length === 0}
            <div class="empty-state">
              <p>No transactions yet</p>
              <p class="hint">Submit transactions via the ledger API</p>
            </div>
          {:else}
            {#each blockchainState?.transactions || [] as tx}
              <div class="data-item">
                <div class="tx-header">
                  <span class="tx-amount">{formatAmount(tx.amount)} {tx.currencyCode}</span>
                  <span class="tx-status" class:pending={tx.status === 'pending'}>
                    {tx.status}
                  </span>
                </div>
                <div class="item-details">
                  <div class="detail-row">
                    <span class="label">From:</span>
                    <span class="value mono">{tx.fromId}</span>
                  </div>
                  <div class="detail-row">
                    <span class="label">To:</span>
                    <span class="value mono">{tx.toId}</span>
                  </div>
                  <div class="detail-row">
                    <span class="label">Type:</span>
                    <span class="value">{tx.type}</span>
                  </div>
                  <div class="detail-row">
                    <span class="label">Created:</span>
                    <span class="value">{formatDate(tx.createdAt)}</span>
                  </div>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {:else if activeTab === 'currencies'}
      <div class="data-panel">
        <div class="panel-header">
          <h3>Currencies ({blockchainState?.currencies?.length || 0})</h3>
        </div>
        <div class="data-list">
          {#if blockchainState?.currencies?.length === 0}
            <div class="empty-state">
              <p>No currencies configured</p>
              <p class="hint">Seed currencies via POST /balances/currencies/seed</p>
            </div>
          {:else}
            {#each blockchainState?.currencies || [] as currency}
              <div class="data-item currency-item">
                <div class="currency-header">
                  <span class="symbol">{currency.symbol}</span>
                  <span class="name">{currency.name}</span>
                  <span class="code">{currency.code}</span>
                </div>
                <div class="item-details">
                  <div class="detail-row">
                    <span class="label">Type:</span>
                    <span class="value">{currency.type}</span>
                  </div>
                  <div class="detail-row">
                    <span class="label">Decimals:</span>
                    <span class="value">{currency.decimals}</span>
                  </div>
                </div>
              </div>
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .state-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #fffbe8;
    position: relative;
  }

  .tab-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px;
    background: #fff9d9;
    border-bottom: 1px solid #e5e0c0;
    overflow-x: auto;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: transparent;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    color: #666;
    transition: all 0.2s;
    white-space: nowrap;
  }

  .tab:hover {
    background: #fff;
    color: #333;
  }

  .tab.active {
    background: #fff;
    color: #000;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
  }

  .tab-icon {
    font-size: 16px;
  }

  .tab-actions {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .refresh-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 18px;
    padding: 4px;
    color: #666;
    transition: transform 0.2s;
  }

  .refresh-btn:hover {
    transform: rotate(180deg);
    color: #000;
  }

  .last-refresh {
    font-size: 11px;
    color: #999;
  }

  .tab-content {
    flex: 1;
    overflow: auto;
    padding: 16px;
  }

  .results-panel {
    height: 100%;
  }

  .results-panel iframe {
    width: 100%;
    height: 100%;
    border: none;
  }

  .data-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
  }

  .panel-header {
    margin-bottom: 16px;
  }

  .panel-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    color: #333;
  }

  .data-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    overflow: auto;
  }

  .data-item {
    background: #fff;
    border-radius: 8px;
    padding: 16px;
    box-shadow: 0 1px 3px rgba(0,0,0,0.1);
  }

  .item-header, .balance-header, .tx-header, .currency-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 12px;
    padding-bottom: 12px;
    border-bottom: 1px solid #f0f0f0;
  }

  .username {
    font-size: 16px;
    font-weight: 600;
    color: #000;
  }

  .display-name {
    font-size: 14px;
    color: #666;
  }

  .amount {
    font-size: 18px;
    font-weight: 700;
    color: #2d5;
  }

  .symbol {
    font-size: 24px;
  }

  .name {
    font-weight: 600;
    color: #333;
  }

  .code {
    font-size: 12px;
    padding: 4px 8px;
    background: #f0f0f0;
    border-radius: 4px;
    color: #666;
  }

  .owner-type, .tx-status {
    font-size: 11px;
    padding: 4px 8px;
    background: #f0f0f0;
    border-radius: 4px;
    text-transform: uppercase;
    font-weight: 600;
    color: #666;
  }

  .tx-status.pending {
    background: #fff3cd;
    color: #856404;
  }

  .item-details {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .detail-row {
    display: flex;
    gap: 8px;
    font-size: 13px;
  }

  .detail-row .label {
    color: #999;
    font-weight: 500;
    min-width: 80px;
  }

  .detail-row .value {
    color: #333;
    word-break: break-all;
  }

  .mono {
    font-family: 'Courier New', monospace;
    font-size: 12px;
  }

  .loading-state, .error-state, .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 300px;
    text-align: center;
    color: #666;
  }

  .spinner {
    width: 40px;
    height: 40px;
    border: 4px solid #f0f0f0;
    border-top-color: #666;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  .error-message {
    font-size: 16px;
    color: #d32f2f;
    margin-bottom: 8px;
  }

  .error-hint {
    font-size: 13px;
    color: #999;
    margin-bottom: 16px;
  }

  .error-state button, .empty-state button {
    padding: 8px 16px;
    background: #333;
    color: #fff;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    font-size: 14px;
  }

  .error-state button:hover {
    background: #000;
  }

  .empty-state p {
    margin: 8px 0;
  }

  .empty-state .hint {
    font-size: 13px;
    color: #999;
  }

  /* Transaction Result Styles */
  .transaction-result {
    background: #fff;
    border-radius: 8px;
    overflow: hidden;
  }

  .result-header {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px;
    background: #f5f5f5;
    border-bottom: 2px solid #e0e0e0;
  }

  .result-header.success {
    background: #e8f5e9;
    border-bottom-color: #4caf50;
  }

  .status-icon {
    font-size: 24px;
    font-weight: bold;
  }

  .result-header.success .status-icon {
    color: #4caf50;
  }

  .status-text {
    font-size: 16px;
    font-weight: 600;
    flex: 1;
  }

  .result-header.success .status-text {
    color: #2e7d32;
  }

  .timestamp {
    font-size: 12px;
    color: #999;
  }

  .result-details {
    padding: 16px;
  }

  .changes-section {
    margin-top: 16px;
  }

  .changes-section h4 {
    margin: 0 0 12px 0;
    font-size: 14px;
    font-weight: 600;
    color: #666;
  }

  .change-item {
    background: #f9f9f9;
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 8px;
  }

  .change-type {
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    color: #999;
    margin-bottom: 8px;
  }

  .change-details {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
  }

  .change-details .arrow {
    color: #999;
  }

  .change-details .amount {
    margin-left: auto;
    font-weight: 600;
    color: #2e7d32;
  }
</style>
