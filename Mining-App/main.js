const { app, BrowserWindow, Menu, ipcMain, dialog, shell } = require('electron');
const path = require('path');
const fs = require('fs');

// Enable live reload for development
if (process.env.NODE_ENV === 'development') {
  require('electron-reload')(__dirname);
}

class QoraNetMiner {
  constructor() {
    this.mainWindow = null;
    this.isQuitting = false;
    
    // App configuration
    this.config = {
      rpcEndpoint: 'https://rpc.qoranet.org',
      miningEnabled: true,
      autoStart: true,
      minimizeToTray: true
    };
  }

  createMainWindow() {
    // Create the browser window
    this.mainWindow = new BrowserWindow({
      width: 1200,
      height: 800,
      minWidth: 1000,
      minHeight: 700,
      show: false,
      icon: path.join(__dirname, 'assets', 'icon.png'),
      titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true,
        enableRemoteModule: false,
        preload: path.join(__dirname, 'preload.js')
      }
    });

    // Load the HTML file
    this.mainWindow.loadFile('qoranet-miner.html');

    // Handle window events
    this.mainWindow.once('ready-to-show', () => {
      this.mainWindow.show();
      
      // Open DevTools in development
      if (process.env.NODE_ENV === 'development') {
        this.mainWindow.webContents.openDevTools();
      }
    });

    this.mainWindow.on('closed', () => {
      this.mainWindow = null;
    });

    this.mainWindow.on('close', (event) => {
      if (!this.isQuitting && process.platform === 'darwin') {
        event.preventDefault();
        this.mainWindow.hide();
      }
    });

    // Handle external links
    this.mainWindow.webContents.setWindowOpenHandler(({ url }) => {
      shell.openExternal(url);
      return { action: 'deny' };
    });
  }

  createMenu() {
    const template = [
      {
        label: 'QoraNet Miner',
        submenu: [
          {
            label: 'About QoraNet Miner',
            click: () => {
              dialog.showMessageBox(this.mainWindow, {
                type: 'info',
                title: 'About QoraNet Miner',
                message: 'QoraNet Desktop Miner v1.0.0',
                detail: 'A next-generation blockchain powered by Proof of Liquidity and distributed application hosting.\n\nBuilt by the QoraNet community.',
                buttons: ['OK']
              });
            }
          },
          { type: 'separator' },
          {
            label: 'Preferences',
            accelerator: 'CmdOrCtrl+,',
            click: () => {
              this.showPreferences();
            }
          },
          { type: 'separator' },
          {
            label: 'Quit',
            accelerator: process.platform === 'darwin' ? 'Cmd+Q' : 'Ctrl+Q',
            click: () => {
              this.isQuitting = true;
              app.quit();
            }
          }
        ]
      },
      {
        label: 'Mining',
        submenu: [
          {
            label: 'Start Mining',
            accelerator: 'CmdOrCtrl+M',
            click: () => {
              this.mainWindow.webContents.send('mining-action', 'start');
            }
          },
          {
            label: 'Stop Mining',
            accelerator: 'CmdOrCtrl+Shift+M',
            click: () => {
              this.mainWindow.webContents.send('mining-action', 'stop');
            }
          },
          { type: 'separator' },
          {
            label: 'Claim Rewards',
            accelerator: 'CmdOrCtrl+R',
            click: () => {
              this.mainWindow.webContents.send('mining-action', 'claim');
            }
          }
        ]
      },
      {
        label: 'View',
        submenu: [
          { role: 'reload' },
          { role: 'forceReload' },
          { role: 'toggleDevTools' },
          { type: 'separator' },
          { role: 'resetZoom' },
          { role: 'zoomIn' },
          { role: 'zoomOut' },
          { type: 'separator' },
          { role: 'togglefullscreen' }
        ]
      },
      {
        label: 'Window',
        submenu: [
          { role: 'minimize' },
          { role: 'close' }
        ]
      },
      {
        label: 'Help',
        submenu: [
          {
            label: 'QoraNet Website',
            click: () => {
              shell.openExternal('https://qoranet.org');
            }
          },
          {
            label: 'Documentation',
            click: () => {
              shell.openExternal('https://docs.qoranet.org');
            }
          },
          {
            label: 'Discord Community',
            click: () => {
              shell.openExternal('https://discord.gg/qoranet');
            }
          },
          { type: 'separator' },
          {
            label: 'Report Issue',
            click: () => {
              shell.openExternal('https://github.com/qorafi/qoranet/issues');
            }
          }
        ]
      }
    ];

    // macOS specific menu adjustments
    if (process.platform === 'darwin') {
      template[0].submenu.unshift({
        label: 'QoraNet Miner',
        role: 'appMenu'
      });
      
      template[3].submenu = [
        { role: 'close' },
        { role: 'minimize' },
        { role: 'zoom' },
        { type: 'separator' },
        { role: 'front' }
      ];
    }

    const menu = Menu.buildFromTemplate(template);
    Menu.setApplicationMenu(menu);
  }

  showPreferences() {
    // Create preferences window (you can expand this)
    const prefsWindow = new BrowserWindow({
      width: 500,
      height: 400,
      parent: this.mainWindow,
      modal: true,
      show: false,
      resizable: false,
      title: 'Preferences',
      webPreferences: {
        nodeIntegration: false,
        contextIsolation: true
      }
    });

    // Load preferences HTML (you can create this separately)
    prefsWindow.loadURL('data:text/html,<h1 style="text-align:center;margin-top:100px;font-family:system-ui">Preferences coming soon!</h1>');
    
    prefsWindow.once('ready-to-show', () => {
      prefsWindow.show();
    });
  }

  setupIPC() {
    // Handle IPC messages from renderer process
    ipcMain.handle('get-config', () => {
      return this.config;
    });

    ipcMain.handle('set-config', (event, newConfig) => {
      this.config = { ...this.config, ...newConfig };
      // Save to file or electron-store
      return this.config;
    });

    ipcMain.handle('show-message', (event, options) => {
      return dialog.showMessageBox(this.mainWindow, options);
    });

    ipcMain.handle('show-open-dialog', (event, options) => {
      return dialog.showOpenDialog(this.mainWindow, options);
    });

    ipcMain.handle('show-save-dialog', (event, options) => {
      return dialog.showSaveDialog(this.mainWindow, options);
    });

    // Mining related IPC
    ipcMain.handle('start-mining', () => {
      console.log('Starting mining process...');
      // Implement your QoraNet mining logic here
      return { success: true, message: 'Mining started successfully!' };
    });

    ipcMain.handle('stop-mining', () => {
      console.log('Stopping mining process...');
      // Implement stop mining logic
      return { success: true, message: 'Mining stopped' };
    });

    ipcMain.handle('claim-rewards', () => {
      console.log('Claiming rewards...');
      // Implement reward claiming logic
      return { success: true, rewards: 2.3471 };
    });
  }

  init() {
    // Handle app events
    app.whenReady().then(() => {
      this.createMainWindow();
      this.createMenu();
      this.setupIPC();

      app.on('activate', () => {
        if (BrowserWindow.getAllWindows().length === 0) {
          this.createMainWindow();
        }
      });
    });

    app.on('window-all-closed', () => {
      if (process.platform !== 'darwin') {
        app.quit();
      }
    });

    app.on('before-quit', () => {
      this.isQuitting = true;
    });

    // Security: Prevent new window creation
    app.on('web-contents-created', (event, contents) => {
      contents.on('new-window', (event, navigationUrl) => {
        event.preventDefault();
        shell.openExternal(navigationUrl);
      });
    });
  }
}

// Initialize the application
const qoraNetMiner = new QoraNetMiner();
qoraNetMiner.init();
