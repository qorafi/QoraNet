const { contextBridge, ipcRenderer } = require('electron');

// Expose protected methods that allow the renderer process to use
// the ipcRenderer without exposing the entire object
contextBridge.exposeInMainWorld('electronAPI', {
  // Config management
  getConfig: () => ipcRenderer.invoke('get-config'),
  setConfig: (config) => ipcRenderer.invoke('set-config', config),
  
  // Dialog methods
  showMessage: (options) => ipcRenderer.invoke('show-message', options),
  showOpenDialog: (options) => ipcRenderer.invoke('show-open-dialog', options),
  showSaveDialog: (options) => ipcRenderer.invoke('show-save-dialog', options),
  
  // Mining operations
  startMining: () => ipcRenderer.invoke('start-mining'),
  stopMining: () => ipcRenderer.invoke('stop-mining'),
  claimRewards: () => ipcRenderer.invoke('claim-rewards'),
  
  // Listen for mining actions from menu
  onMiningAction: (callback) => ipcRenderer.on('mining-action', callback),
  
  // Platform info
  platform: process.platform,
  
  // Version info
  versions: {
    node: process.versions.node,
    chrome: process.versions.chrome,
    electron: process.versions.electron
  }
});

// Expose QoraNet specific APIs
contextBridge.exposeInMainWorld('qoranetAPI', {
  // RPC connection methods
  connectRPC: async (endpoint) => {
    // This would connect to your QoraNet RPC
    console.log('Connecting to QoraNet RPC:', endpoint);
    return { success: true, connected: true };
  },
  
  // Blockchain data
  getBalance: async (address) => {
    // Mock implementation - replace with actual RPC calls
    return {
      qor: 24.67221,
      usd: 61.68
    };
  },
  
  getLiquidityPools: async () => {
    // Mock LP data
    return [
      { pair: 'QOR/USDC', contribution: 3421, apy: 24.5 },
      { pair: 'QOR/ETH', contribution: 4824, apy: 18.2 }
    ];
  },
  
  getRunningApps: async () => {
    // Mock app data
    return [
      { name: 'IPFS Storage Node', status: 'running', uptime: 97, rewards: 0.0012 },
      { name: 'Cross-chain Bridge', status: 'running', uptime: 89, rewards: 0.0018 },
      { name: 'Oracle Service', status: 'stopped', uptime: 0, rewards: 0 }
    ];
  },
  
  getMiningStats: async () => {
    // Mock mining stats
    return {
      rate: 0.0043,
      rank: 147,
      totalMiners: 2834,
      uptime: 87
    };
  }
});

// Add some utility functions
contextBridge.exposeInMainWorld('utils', {
  formatNumber: (num, decimals = 2) => {
    return Number(num).toLocaleString(undefined, {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals
    });
  },
  
  formatCurrency: (amount) => {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD'
    }).format(amount);
  },
  
  formatTime: (seconds) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    return `${hours}h ${minutes}m`;
  }
});
