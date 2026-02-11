const { invoke } = window.__TAURI__.core;

// State
let relayStatus = null;
let isAuthenticated = false;

// Initialize app
async function init() {
    console.log('Initializing Skills Cookbook Relay UI...');

    // Set up event listeners
    document.getElementById('loginBtn').addEventListener('click', handleLogin);
    document.getElementById('logoutBtn').addEventListener('click', handleLogout);
    document.getElementById('refreshBtn').addEventListener('click', handleRefresh);
    document.getElementById('clearCacheBtn').addEventListener('click', handleClearCache);
    document.getElementById('wipeDataBtn').addEventListener('click', handleWipeData);
    document.getElementById('debugToggle').addEventListener('change', handleDebugToggle);

    // Initial status update
    await updateStatus();

    // Set up periodic status updates
    setInterval(updateStatus, 5000);
}

// Update status display
async function updateStatus() {
    try {
        relayStatus = await invoke('get_relay_status');

        // Update connection indicator
        const statusIndicator = document.getElementById('connectionStatus');
        if (relayStatus.authenticated) {
            statusIndicator.classList.add('connected');
            statusIndicator.querySelector('.text').textContent = 'Connected';
        } else {
            statusIndicator.classList.remove('connected');
            statusIndicator.querySelector('.text').textContent = 'Disconnected';
        }

        // Update authentication views
        isAuthenticated = relayStatus.authenticated;
        if (isAuthenticated) {
            document.getElementById('notAuthenticatedView').style.display = 'none';
            document.getElementById('authenticatedView').style.display = 'block';
            document.getElementById('userEmail').textContent = relayStatus.user_email || 'Unknown';
            document.getElementById('workspaceId').textContent = relayStatus.workspace_id || 'Default';
        } else {
            document.getElementById('notAuthenticatedView').style.display = 'block';
            document.getElementById('authenticatedView').style.display = 'none';
        }

        // Update status values
        document.getElementById('mcpPort').textContent = relayStatus.mcp_server_port || '-';
        document.getElementById('lastSync').textContent = relayStatus.last_sync
            ? new Date(relayStatus.last_sync).toLocaleString()
            : 'Never';
        document.getElementById('artifactCount').textContent = relayStatus.artifact_count;
        document.getElementById('cacheSize').textContent = formatBytes(relayStatus.cache_size_bytes);
        document.getElementById('libraryCount').textContent = relayStatus.library_count;

        // Update libraries list if authenticated
        if (isAuthenticated) {
            await updateLibraries();
        }
    } catch (error) {
        console.error('Failed to update status:', error);
    }
}

// Update libraries list
async function updateLibraries() {
    try {
        const libraries = await invoke('list_libraries');
        const librariesList = document.getElementById('librariesList');

        if (libraries.length === 0) {
            librariesList.innerHTML = '<p class="placeholder">No libraries available.</p>';
            return;
        }

        librariesList.innerHTML = libraries.map(lib => `
            <div class="library-item">
                <h3>${escapeHtml(lib.name)}</h3>
                <p>${escapeHtml(lib.description || 'No description')}</p>
                <p style="font-size: 0.75rem; margin-top: 0.5rem;">
                    Skills: ${lib.skills ? lib.skills.length : 0}
                </p>
            </div>
        `).join('');
    } catch (error) {
        console.error('Failed to update libraries:', error);
    }
}

// Handle login
async function handleLogin() {
    const btn = document.getElementById('loginBtn');
    btn.disabled = true;
    btn.textContent = 'Connecting...';

    try {
        const authStatus = await invoke('login_to_rss');
        console.log('Login successful:', authStatus);
        await updateStatus();
    } catch (error) {
        console.error('Login failed:', error);
        alert('Login failed: ' + error);
    } finally {
        btn.disabled = false;
        btn.textContent = 'Connect Account';
    }
}

// Handle logout
async function handleLogout() {
    if (!confirm('Are you sure you want to disconnect? This will clear your authentication tokens.')) {
        return;
    }

    try {
        await invoke('logout_from_rss');
        await updateStatus();
    } catch (error) {
        console.error('Logout failed:', error);
        alert('Logout failed: ' + error);
    }
}

// Handle refresh
async function handleRefresh() {
    const btn = document.getElementById('refreshBtn');
    btn.disabled = true;
    const originalText = btn.innerHTML;
    btn.innerHTML = '<span>Refreshing...</span>';

    try {
        const result = await invoke('refresh_skills');
        console.log('Refresh result:', result);
        alert(`Refresh complete!\n\nUpdated: ${result.updated_artifacts}\nRevoked: ${result.revoked_artifacts}`);
        await updateStatus();
    } catch (error) {
        console.error('Refresh failed:', error);
        alert('Refresh failed: ' + error);
    } finally {
        btn.disabled = false;
        btn.innerHTML = originalText;
    }
}

// Handle clear cache
async function handleClearCache() {
    if (!confirm('Are you sure you want to clear the cache? Skills will need to be re-downloaded.')) {
        return;
    }

    try {
        await invoke('clear_cache');
        await updateStatus();
        alert('Cache cleared successfully.');
    } catch (error) {
        console.error('Clear cache failed:', error);
        alert('Clear cache failed: ' + error);
    }
}

// Handle wipe data
async function handleWipeData() {
    if (!confirm('⚠️ WARNING: This will delete ALL cached data and tokens. Are you absolutely sure?')) {
        return;
    }

    if (!confirm('This action cannot be undone. Continue?')) {
        return;
    }

    try {
        await invoke('wipe_all_data');
        await updateStatus();
        alert('All data has been wiped.');
    } catch (error) {
        console.error('Wipe data failed:', error);
        alert('Wipe data failed: ' + error);
    }
}

// Handle debug toggle
function handleDebugToggle(event) {
    const debugOutput = document.getElementById('debugOutput');
    if (event.target.checked) {
        debugOutput.style.display = 'block';
        startDebugLogging();
    } else {
        debugOutput.style.display = 'none';
        stopDebugLogging();
    }
}

let debugInterval = null;

function startDebugLogging() {
    debugInterval = setInterval(() => {
        const debugLog = document.getElementById('debugLog');
        const timestamp = new Date().toISOString();
        const logEntry = `[${timestamp}] Status: ${isAuthenticated ? 'Connected' : 'Disconnected'}\n`;
        debugLog.textContent += logEntry;

        // Keep only last 50 lines
        const lines = debugLog.textContent.split('\n');
        if (lines.length > 50) {
            debugLog.textContent = lines.slice(-50).join('\n');
        }

        // Auto-scroll to bottom
        debugLog.parentElement.scrollTop = debugLog.parentElement.scrollHeight;
    }, 1000);
}

function stopDebugLogging() {
    if (debugInterval) {
        clearInterval(debugInterval);
        debugInterval = null;
    }
}

// Utility functions
function formatBytes(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return Math.round(bytes / Math.pow(k, i) * 100) / 100 + ' ' + sizes[i];
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
} else {
    init();
}
