import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

interface ModInfo {
  name: string;
  version: string;
  author: string;
  description: string;
  folder_name: string;
  enabled: boolean;
  update_keys: string[];
}

interface StardewInfo {
  game_path: string | null;
  mods_path: string | null;
  found: boolean;
}

interface UpdateInfo {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  download_url: string | null;
}

interface AppSettings {
  nexus_api_key: string | null;
}

function App() {
  const [stardewInfo, setStardewInfo] = useState<StardewInfo | null>(null);
  const [mods, setMods] = useState<ModInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [updates, setUpdates] = useState<Record<string, UpdateInfo>>({});
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [currentModBeingChecked, setCurrentModBeingChecked] = useState<string>("");
  const [updateProgress, setUpdateProgress] = useState({ current: 0, total: 0 });
  const [showSettings, setShowSettings] = useState(false);
  const [settings, setSettings] = useState<AppSettings>({ nexus_api_key: null });
  const [tempApiKey, setTempApiKey] = useState("");
  const [lastUpdateCheck, setLastUpdateCheck] = useState<number | null>(null);
  const [showApiWarning, setShowApiWarning] = useState(false);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const [updateModalMod, setUpdateModalMod] = useState<ModInfo | null>(null);
  const [updateModalInfo, setUpdateModalInfo] = useState<UpdateInfo | null>(null);
  const [updateStep, setUpdateStep] = useState(1);
  const [isVerifying, setIsVerifying] = useState(false);
  const [verificationStatus, setVerificationStatus] = useState<string | null>(null);

  useEffect(() => {
    initializeApp();
    loadSettings();
    loadPersistedUpdates();
  }, []);

  function loadPersistedUpdates() {
    try {
      const savedUpdates = localStorage.getItem('mod-updates');
      const savedLastCheck = localStorage.getItem('last-update-check');
      
      if (savedUpdates) {
        setUpdates(JSON.parse(savedUpdates));
      }
      
      if (savedLastCheck) {
        setLastUpdateCheck(parseInt(savedLastCheck, 10));
      }
    } catch (err) {
      console.error("Error loading persisted updates:", err);
    }
  }

  function savePersistedUpdates(updateInfo: Record<string, UpdateInfo>, timestamp: number) {
    try {
      localStorage.setItem('mod-updates', JSON.stringify(updateInfo));
      localStorage.setItem('last-update-check', timestamp.toString());
    } catch (err) {
      console.error("Error saving persisted updates:", err);
    }
  }

  async function loadSettings() {
    try {
      const loadedSettings = await invoke<AppSettings>("get_settings");
      setSettings(loadedSettings);
      setTempApiKey(loadedSettings.nexus_api_key || "");
    } catch (err) {
      console.error("Error loading settings:", err);
    }
  }

  async function initializeApp() {
    try {
      setLoading(true);
      setError(null);
      
      const info = await invoke<StardewInfo>("detect_stardew_valley");
      setStardewInfo(info);
      
      if (info.found && info.mods_path) {
        const modList = await invoke<ModInfo[]>("scan_mods", { modsPath: info.mods_path });
        setMods(modList);
      }
    } catch (err) {
      console.error("Error in initializeApp:", err);
      setError(err instanceof Error ? err.message : `Unknown error: ${JSON.stringify(err)}`);
    } finally {
      setLoading(false);
    }
  }

  async function refreshMods() {
    if (!stardewInfo?.mods_path) return;
    
    try {
      setLoading(true);
      const modList = await invoke<ModInfo[]>("scan_mods", { modsPath: stardewInfo.mods_path });
      setMods(modList);
    } catch (err) {
      console.error("Error in refreshMods:", err);
      setError(err instanceof Error ? err.message : `Failed to refresh mods: ${JSON.stringify(err)}`);
    } finally {
      setLoading(false);
    }
  }

  function handleCheckUpdatesClick() {
    const now = Date.now();
    const oneHourAgo = now - (60 * 60 * 1000); // 1 hour in milliseconds
    
    // Check if we've done an update check in the past hour
    if (lastUpdateCheck && lastUpdateCheck > oneHourAgo && settings.nexus_api_key) {
      setShowApiWarning(true);
      return;
    }
    
    checkForUpdates();
  }

  async function checkForUpdates() {
    if (mods.length === 0) return;
    
    try {
      setCheckingUpdates(true);
      setUpdateProgress({ current: 0, total: mods.length });
      
      // Filter mods that have update keys
      const modsWithUpdateKeys = mods.filter(mod => mod.update_keys.length > 0);
      const totalMods = modsWithUpdateKeys.length;
      
      if (totalMods === 0) {
        setCheckingUpdates(false);
        return;
      }
      
      setUpdateProgress({ current: 0, total: totalMods });
      
      // Since the backend processes all mods at once, we'll simulate progress
      // by showing each mod name as we "process" it
      const simulateProgress = async () => {
        for (let i = 0; i < totalMods; i++) {
          setCurrentModBeingChecked(modsWithUpdateKeys[i].name);
          setUpdateProgress({ current: i + 1, total: totalMods });
          
          // Small delay to show the progress
          await new Promise(resolve => setTimeout(resolve, 100));
        }
      };
      
      // Start the progress simulation and actual API call concurrently
      const [updateInfo] = await Promise.all([
        invoke<Record<string, UpdateInfo>>("check_mod_updates", { mods }),
        simulateProgress()
      ]);
      
      const timestamp = Date.now();
      
      setUpdates(updateInfo);
      setLastUpdateCheck(timestamp);
      savePersistedUpdates(updateInfo, timestamp);
    } catch (err) {
      console.error("Error checking for updates:", err);
      setError(err instanceof Error ? err.message : `Failed to check for updates: ${JSON.stringify(err)}`);
    } finally {
      setCheckingUpdates(false);
      setCurrentModBeingChecked("");
      setUpdateProgress({ current: 0, total: 0 });
    }
  }

  function proceedWithUpdateCheck() {
    setShowApiWarning(false);
    checkForUpdates();
  }

  function startUpdateProcess(mod: ModInfo, updateInfo: UpdateInfo) {
    setUpdateModalMod(mod);
    setUpdateModalInfo(updateInfo);
    setUpdateStep(1);
    setShowUpdateModal(true);
  }

  function closeUpdateModal() {
    setShowUpdateModal(false);
    setUpdateModalMod(null);
    setUpdateModalInfo(null);
    setUpdateStep(1);
    setIsVerifying(false);
    setVerificationStatus(null);
  }

  async function openModsFolder() {
    if (!stardewInfo?.mods_path) return;
    try {
      await invoke('open_folder', { path: stardewInfo.mods_path });
    } catch (err) {
      console.error('Failed to open mods folder:', err);
      alert(`Failed to open mods folder: ${err}`);
    }
  }

  async function verifyModUpdate() {
    if (!updateModalMod) {
      console.log('No updateModalMod available');
      return;
    }
    
    console.log('Verifying update for mod:', updateModalMod.name);
    console.log('Mod info being sent:', updateModalMod);
    
    setIsVerifying(true);
    setVerificationStatus('Checking mod version...');
    
    try {
      const newUpdateInfo = await invoke<UpdateInfo>('check_single_mod_update_frontend', { modInfo: updateModalMod });
      console.log('Verification result:', newUpdateInfo);
      
      setVerificationStatus('Processing verification result...');
      
      if (!newUpdateInfo.update_available) {
        console.log('Update verification successful - removing from updates list');
        setVerificationStatus('Update verified successfully!');
        
        // Update successful - remove from updates list and refresh
        setUpdates(prevUpdates => {
          const newUpdates = { ...prevUpdates };
          delete newUpdates[updateModalMod.folder_name];
          localStorage.setItem('mod-updates', JSON.stringify(newUpdates));
          return newUpdates;
        });
        
        // Refresh mod list to show new version
        setVerificationStatus('Refreshing mod list...');
        await refreshMods();
        
        setTimeout(() => {
          alert('✅ Mod updated successfully! The mod is now up to date.');
          closeUpdateModal();
        }, 500);
      } else {
        setVerificationStatus('Update not detected');
        setTimeout(() => {
          alert(`❌ Update not detected. Current version: ${newUpdateInfo.current_version}, Latest version: ${newUpdateInfo.latest_version}. Please make sure you installed the latest version correctly, or use "Force Update Version" if the developer forgot to update the manifest.`);
          setVerificationStatus(null);
        }, 1000);
      }
    } catch (err) {
      console.error('Failed to verify update:', err);
      setVerificationStatus('Verification failed');
      setTimeout(() => {
        alert(`Failed to verify update: ${err}`);
        setVerificationStatus(null);
      }, 1000);
    } finally {
      setTimeout(() => setIsVerifying(false), 500);
    }
  }

  async function forceUpdateVersion() {
    console.log('forceUpdateVersion called');
    console.log('updateModalMod:', updateModalMod);
    console.log('updateModalInfo:', updateModalInfo);
    console.log('stardewInfo?.mods_path:', stardewInfo?.mods_path);
    
    if (!updateModalMod || !updateModalInfo || !stardewInfo?.mods_path) {
      console.log('Missing required data for force update');
      alert('Error: Missing required information for force update');
      return;
    }
    
    console.log('About to show confirmation dialog...');
    
    // Temporarily skip confirmation for testing
    const confirmed = true;
    console.log('Skipping confirmation dialog, proceeding with update...');
    
    console.log('User confirmed:', confirmed);
    if (!confirmed) return;
    
    setIsVerifying(true);
    setVerificationStatus('Updating manifest version...');
    
    try {
      console.log('Calling update_manifest_version with:', {
        modsPath: stardewInfo.mods_path,
        modFolderName: updateModalMod.folder_name,
        newVersion: updateModalInfo.latest_version
      });
      
      console.log('About to call invoke...');
      try {
        const result = await invoke('update_manifest_version', {
          modsPath: stardewInfo.mods_path,
          modFolderName: updateModalMod.folder_name,
          newVersion: updateModalInfo.latest_version
        });
        console.log('Invoke returned successfully:', result);
      } catch (invokeError) {
        console.error('Invoke failed with error:', invokeError);
        throw invokeError; // Re-throw to be caught by outer try-catch
      }
      
      console.log('Manifest version update successful, updating UI...');
      setVerificationStatus('Manifest updated, refreshing mod list...');
      
      // Remove from updates list and refresh
      setUpdates(prevUpdates => {
        const newUpdates = { ...prevUpdates };
        delete newUpdates[updateModalMod.folder_name];
        localStorage.setItem('mod-updates', JSON.stringify(newUpdates));
        return newUpdates;
      });
      
      // Refresh mod list to show new version
      await refreshMods();
      
      setVerificationStatus('Success! Version updated.');
      setTimeout(() => {
        alert('✅ Manifest version updated successfully! The mod is now marked as up to date.');
        closeUpdateModal();
      }, 500);
    } catch (err) {
      console.error('Failed to update manifest version:', err);
      console.error('Error details:', JSON.stringify(err, null, 2));
      setVerificationStatus(`Failed to update manifest version: ${err}`);
      setTimeout(() => {
        alert(`Failed to update manifest version: ${err}`);
        setVerificationStatus(null);
      }, 1000);
    } finally {
      setTimeout(() => setIsVerifying(false), 500);
    }
  }


  async function saveSettings() {
    const newSettings: AppSettings = {
      nexus_api_key: tempApiKey.trim() || null
    };

    try {
      await invoke("save_settings", { settings: newSettings });
      setSettings(newSettings);
      setShowSettings(false);
    } catch (err) {
      console.error("Error saving settings:", err);
      setError(err instanceof Error ? err.message : `Failed to save settings: ${JSON.stringify(err)}`);
    }
  }

  if (loading) {
    return (
      <main className="container">
        <h1>Stardew Valley Mod Manager</h1>
        <p>Loading...</p>
      </main>
    );
  }

  if (error) {
    return (
      <main className="container">
        <h1>Stardew Valley Mod Manager</h1>
        <div className="error">
          <p>Error: {error}</p>
          <button onClick={initializeApp}>Retry</button>
        </div>
      </main>
    );
  }

  if (!stardewInfo?.found) {
    return (
      <main className="container">
        <h1>Stardew Valley Mod Manager</h1>
        <div className="not-found">
          <p>Stardew Valley installation not found!</p>
          <p>Please make sure Stardew Valley is installed and try again.</p>
          <button onClick={initializeApp}>Retry Detection</button>
        </div>
      </main>
    );
  }

  return (
    <main className="container">
      <div className="header">
        <h1>Stardew Valley Mod Manager</h1>
        <button className="settings-button" onClick={() => setShowSettings(true)}>
          Settings
        </button>
      </div>
      
      <div className="game-info">
        <h2>Game Information</h2>
        <p><strong>Game Path:</strong> {stardewInfo.game_path}</p>
        <p><strong>Mods Path:</strong> {stardewInfo.mods_path || "Mods folder not found"}</p>
      </div>

      {stardewInfo.mods_path ? (
        <div className="mods-section">
          <div className="mods-header">
            <h2>Installed Mods ({mods.length})</h2>
            <div className="mods-actions">
              <button onClick={refreshMods} disabled={loading}>
                Refresh
              </button>
              <button onClick={handleCheckUpdatesClick} disabled={checkingUpdates || mods.length === 0}>
                {checkingUpdates ? "Checking..." : "Check Updates"}
              </button>
            </div>
          </div>
          
          {mods.length === 0 ? (
            <p>No mods found in the Mods folder.</p>
          ) : (
            <div className="mods-list">
              {mods
                .sort((a, b) => {
                  const aHasUpdate = updates[a.folder_name]?.update_available || false;
                  const bHasUpdate = updates[b.folder_name]?.update_available || false;
                  
                  if (aHasUpdate && !bHasUpdate) return -1;
                  if (!aHasUpdate && bHasUpdate) return 1;
                  return a.name.localeCompare(b.name);
                })
                .map((mod) => {
                const updateInfo = updates[mod.folder_name];
                return (
                  <div key={mod.folder_name} className="mod-card">
                    <div className="mod-header">
                      <h3>{mod.name}</h3>
                      <div className="mod-version-info">
                        <span className="mod-version">v{mod.version}</span>
                        {updateInfo?.update_available && (
                          <span className="update-available">
                            → v{updateInfo.latest_version}
                          </span>
                        )}
                      </div>
                    </div>
                    <p className="mod-author">by {mod.author}</p>
                    <p className="mod-description">{mod.description}</p>
                    {mod.update_keys.length > 0 && (
                      <div className="mod-update-keys">
                        <small>Update sources: {mod.update_keys.join(", ")}</small>
                      </div>
                    )}
                    <div className="mod-footer">
                      <span className="mod-folder">Folder: {mod.folder_name}</span>
                      {updateInfo && updateInfo.download_url && (
                        updateInfo.update_available ? (
                          <button 
                            className="update-button"
                            onClick={() => startUpdateProcess(mod, updateInfo)}
                            title={`Update from ${updateInfo.current_version} to ${updateInfo.latest_version}`}
                          >
                            Update Available
                          </button>
                        ) : updateInfo.latest_version === "Manual check" ? (
                          <button 
                            className="manual-check-button"
                            onClick={async () => {
                              try {
                                await invoke('open_url', { url: updateInfo.download_url! });
                              } catch (err) {
                                console.error('Failed to open URL:', err);
                                alert(`Failed to open URL: ${err}`);
                              }
                            }}
                          >
                            Check for Updates
                          </button>
                        ) : null
                      )}
                      <label className="mod-toggle">
                        <input 
                          type="checkbox" 
                          checked={mod.enabled} 
                          onChange={() => {}}
                          disabled
                        />
                        Enabled
                      </label>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      ) : (
        <div className="no-mods-folder">
          <p>No Mods folder found. Create a "Mods" folder in your Stardew Valley directory to start using mods.</p>
        </div>
      )}

      {showSettings && (
        <div className="settings-modal">
          <div className="settings-content">
            <div className="settings-header">
              <h2>Settings</h2>
              <button className="close-button" onClick={() => setShowSettings(false)}>×</button>
            </div>
            
            <div className="settings-body">
              <div className="setting-group">
                <label htmlFor="nexus-api-key">Nexus Mods API Key</label>
                <input
                  id="nexus-api-key"
                  type="password"
                  value={tempApiKey}
                  onChange={(e) => setTempApiKey(e.target.value)}
                  placeholder="Enter your Nexus Mods API key"
                />
                <div className="setting-help">
                  <p>Get your API key from: <button 
                    onClick={async () => {
                      try {
                        await invoke('open_url', { url: "https://www.nexusmods.com/users/myaccount?tab=api" });
                      } catch (err) {
                        console.error('Failed to open URL:', err);
                      }
                    }}
                    style={{
                      background: 'none',
                      border: 'none',
                      color: '#396cd8',
                      textDecoration: 'underline',
                      cursor: 'pointer',
                      padding: 0,
                      font: 'inherit'
                    }}
                  >
                    Nexus Mods Account Settings
                  </button></p>
                  <p>This enables automatic update checking for Nexus mods.</p>
                  {settings.nexus_api_key && (
                    <p className="api-status">✅ API key configured</p>
                  )}
                </div>
              </div>
            </div>
            
            <div className="settings-footer">
              <button onClick={() => setShowSettings(false)}>Cancel</button>
              <button onClick={saveSettings} className="primary">Save Settings</button>
            </div>
          </div>
        </div>
      )}

      {showApiWarning && (
        <div className="warning-modal">
          <div className="warning-content">
            <div className="warning-header">
              <h3>⚠️ API Usage Warning</h3>
            </div>
            
            <div className="warning-body">
              <p>You've already checked for updates within the past hour.</p>
              <p>The Nexus Mods API has usage limits. Frequent requests may exceed your daily quota and temporarily block further API calls.</p>
              <p>Are you sure you want to check for updates again?</p>
            </div>
            
            <div className="warning-footer">
              <button onClick={() => setShowApiWarning(false)}>Cancel</button>
              <button onClick={proceedWithUpdateCheck} className="warning-proceed">
                Check Anyway
              </button>
            </div>
          </div>
        </div>
      )}

      {checkingUpdates && (
        <div className="update-progress-modal">
          <div className="update-progress-content">
            <div className="update-progress-header">
              <h3>Checking for Updates</h3>
            </div>
            
            <div className="update-progress-body">
              {currentModBeingChecked && (
                <p className="current-mod">Checking: {currentModBeingChecked}</p>
              )}
              
              <div className="progress-bar-container">
                <div className="progress-bar">
                  <div 
                    className="progress-bar-fill"
                    style={{
                      width: updateProgress.total > 0 
                        ? `${(updateProgress.current / updateProgress.total) * 100}%` 
                        : '0%'
                    }}
                  ></div>
                </div>
                <div className="progress-text">
                  {updateProgress.current} of {updateProgress.total} mods
                </div>
              </div>
            </div>
          </div>
        </div>
      )}

      {showUpdateModal && updateModalMod && updateModalInfo && (
        <div className="update-modal">
          <div className="update-modal-content">
            <div className="update-modal-header">
              <h3>Update {updateModalMod.name}</h3>
              <button className="close-button" onClick={closeUpdateModal}>×</button>
            </div>
            
            <div className="update-modal-body">
              <div className="update-info">
                <p><strong>Current version:</strong> {updateModalInfo.current_version}</p>
                <p><strong>Latest version:</strong> {updateModalInfo.latest_version}</p>
              </div>
              
              <div className="update-steps">
                <div className={`update-step ${updateStep >= 1 ? 'active' : ''}`}>
                  <div className="step-number">1</div>
                  <div className="step-content">
                    <h4>Download the updated mod</h4>
                    <p>Download the latest version of the mod from Nexus Mods.</p>
                    <button 
                      className="step-button"
                      onClick={async () => {
                        try {
                          await invoke('open_url', { url: updateModalInfo.download_url! });
                          setUpdateStep(2);
                        } catch (err) {
                          console.error('Failed to open URL:', err);
                          alert(`Failed to open URL: ${err}`);
                        }
                      }}
                    >
                      📥 Open Nexus Mods Page
                    </button>
                  </div>
                </div>
                
                <div className={`update-step ${updateStep >= 2 ? 'active' : ''}`}>
                  <div className="step-number">2</div>
                  <div className="step-content">
                    <h4>Install the mod</h4>
                    <p>Extract the downloaded zip file and replace the old mod folder with the new version.</p>
                    <div className="step-buttons">
                      <button 
                        className="step-button"
                        onClick={() => openModsFolder()}
                      >
                        📁 Open Mods Folder
                      </button>
                      <button 
                        className="step-button secondary"
                        onClick={() => setUpdateStep(3)}
                        disabled={updateStep < 2}
                      >
                        ✅ I've installed it
                      </button>
                    </div>
                  </div>
                </div>
                
                <div className={`update-step ${updateStep >= 3 ? 'active' : ''}`}>
                  <div className="step-number">3</div>
                  <div className="step-content">
                    <h4>Verify the update</h4>
                    <p>Choose how to verify the update:</p>
                    <div className="step-buttons">
                      <button 
                        className="step-button"
                        onClick={() => {
                          console.log('Verify button clicked!', 'Step:', updateStep, 'Verifying:', isVerifying);
                          verifyModUpdate();
                        }}
                        disabled={updateStep < 3 || isVerifying}
                      >
                        {isVerifying ? '🔍 Verifying...' : '🔍 Auto-Verify'}
                      </button>
                      <button 
                        className="step-button secondary"
                        onClick={(e) => {
                          console.log('Force update button clicked!', e);
                          console.log('Button disabled?', updateStep < 3 || isVerifying);
                          console.log('Current step:', updateStep);
                          console.log('Is verifying:', isVerifying);
                          e.preventDefault();
                          e.stopPropagation();
                          forceUpdateVersion();
                        }}
                        disabled={updateStep < 3 || isVerifying}
                        title="Use this if the mod files are updated but the developer forgot to update the version in manifest.json"
                      >
                        {isVerifying ? '⚠️ Updating...' : '⚠️ Force Update Version'}
                      </button>
                      <button 
                        className="step-button"
                        onClick={() => {
                          console.log('TEST: Force update function called directly');
                          forceUpdateVersion();
                        }}
                        style={{ background: '#ff6b6b', marginTop: '10px' }}
                      >
                        🔧 TEST Force Update (Always Enabled)
                      </button>
                    </div>
                    {verificationStatus && (
                      <div className="verification-status">
                        <p>🔄 {verificationStatus}</p>
                      </div>
                    )}
                    <div className="verification-help">
                      <p><strong>Auto-Verify:</strong> Checks if the manifest version was updated</p>
                      <p><strong>Force Update:</strong> Use if the mod files are new but manifest version wasn't updated by the developer</p>
                    </div>
                  </div>
                </div>
              </div>
            </div>
            
            <div className="update-modal-footer">
              <button onClick={closeUpdateModal}>Cancel</button>
            </div>
          </div>
        </div>
      )}
    </main>
  );
}

export default App;
