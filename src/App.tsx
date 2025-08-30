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
}

interface StardewInfo {
  game_path: string | null;
  mods_path: string | null;
  found: boolean;
}

function App() {
  const [stardewInfo, setStardewInfo] = useState<StardewInfo | null>(null);
  const [mods, setMods] = useState<ModInfo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    initializeApp();
  }, []);

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
      <h1>Stardew Valley Mod Manager</h1>
      
      <div className="game-info">
        <h2>Game Information</h2>
        <p><strong>Game Path:</strong> {stardewInfo.game_path}</p>
        <p><strong>Mods Path:</strong> {stardewInfo.mods_path || "Mods folder not found"}</p>
      </div>

      {stardewInfo.mods_path ? (
        <div className="mods-section">
          <div className="mods-header">
            <h2>Installed Mods ({mods.length})</h2>
            <button onClick={refreshMods} disabled={loading}>
              Refresh
            </button>
          </div>
          
          {mods.length === 0 ? (
            <p>No mods found in the Mods folder.</p>
          ) : (
            <div className="mods-list">
              {mods.map((mod) => (
                <div key={mod.folder_name} className="mod-card">
                  <div className="mod-header">
                    <h3>{mod.name}</h3>
                    <span className="mod-version">v{mod.version}</span>
                  </div>
                  <p className="mod-author">by {mod.author}</p>
                  <p className="mod-description">{mod.description}</p>
                  <div className="mod-footer">
                    <span className="mod-folder">Folder: {mod.folder_name}</span>
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
              ))}
            </div>
          )}
        </div>
      ) : (
        <div className="no-mods-folder">
          <p>No Mods folder found. Create a "Mods" folder in your Stardew Valley directory to start using mods.</p>
        </div>
      )}
    </main>
  );
}

export default App;
