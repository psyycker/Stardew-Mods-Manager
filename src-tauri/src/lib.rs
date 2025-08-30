use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub folder_name: String,
    pub enabled: bool,
    pub update_keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StardewInfo {
    pub game_path: Option<PathBuf>,
    pub mods_path: Option<PathBuf>,
    pub found: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub download_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NexusModInfo {
    pub version: String,
    pub mod_id: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct NexusFileInfo {
    pub version: String,
    pub file_id: u32,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct GitHubRelease {
    pub tag_name: String,
    pub html_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub nexus_api_key: Option<String>,
}

#[tauri::command]
fn detect_stardew_valley() -> Result<StardewInfo, String> {
    let possible_paths = get_stardew_paths();
    
    if possible_paths.is_empty() {
        return Err("No potential Stardew Valley installation paths found for this operating system".to_string());
    }
    
    
    for path in possible_paths {
        if is_stardew_directory(&path) {
            // Check different possible Mods folder locations
            let mut mods_path = None;
            
            // Standard location
            let standard_mods = path.join("Mods");
            if standard_mods.exists() {
                mods_path = Some(standard_mods);
            } else {
                // macOS Steam version - check Contents/MacOS/Mods
                let contents_macos_mods = path.join("Contents").join("MacOS").join("Mods");
                if contents_macos_mods.exists() {
                    mods_path = Some(contents_macos_mods);
                } else {
                    // Try Contents/Resources/Mods
                    let contents_resources_mods = path.join("Contents").join("Resources").join("Mods");
                    if contents_resources_mods.exists() {
                        mods_path = Some(contents_resources_mods);
                    }
                }
            }
            
            return Ok(StardewInfo {
                game_path: Some(path),
                mods_path,
                found: true,
            });
        }
    }
    
    Ok(StardewInfo {
        game_path: None,
        mods_path: None,
        found: false,
    })
}

#[tauri::command]
fn scan_mods(mods_path: String) -> Result<Vec<ModInfo>, String> {
    let path = Path::new(&mods_path);
    let mut mods = Vec::new();
    
    if !path.exists() {
        return Err(format!("Mods directory does not exist: {}", mods_path));
    }
    
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", mods_path));
    }
    
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => {
                        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                            if let Some(mod_info) = parse_mod_folder(&entry.path()) {
                                mods.push(mod_info);
                            }
                        }
                    },
                    Err(e) => {
                        eprintln!("Error reading directory entry: {}", e);
                    }
                }
            }
        },
        Err(e) => {
            return Err(format!("Failed to read mods directory: {}", e));
        }
    }
    
    // Sort mods alphabetically by name, ignoring [CP] prefix
    mods.sort_by(|a, b| {
        let clean_name_a = a.name.strip_prefix("[CP] ").unwrap_or(&a.name).to_lowercase();
        let clean_name_b = b.name.strip_prefix("[CP] ").unwrap_or(&b.name).to_lowercase();
        clean_name_a.cmp(&clean_name_b)
    });
    
    Ok(mods)
}

#[tauri::command]
async fn check_mod_updates(mods: Vec<ModInfo>) -> Result<HashMap<String, UpdateInfo>, String> {
    let mut updates = HashMap::new();
    
    for mod_info in mods {
        if !mod_info.update_keys.is_empty() {
            match check_single_mod_update(&mod_info).await {
                Ok(update_info) => {
                    updates.insert(mod_info.folder_name, update_info);
                }
                Err(e) => {
                    eprintln!("Error checking updates for {}: {}", mod_info.name, e);
                    // Continue with other mods
                }
            }
        }
    }
    
    Ok(updates)
}

#[tauri::command]
fn get_settings() -> Result<AppSettings, String> {
    let settings_path = get_settings_path()?;
    
    if settings_path.exists() {
        match fs::read_to_string(&settings_path) {
            Ok(content) => {
                match serde_json::from_str::<AppSettings>(&content) {
                    Ok(settings) => Ok(settings),
                    Err(e) => {
                        eprintln!("Error parsing settings: {}", e);
                        Ok(AppSettings { nexus_api_key: None })
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading settings file: {}", e);
                Ok(AppSettings { nexus_api_key: None })
            }
        }
    } else {
        Ok(AppSettings { nexus_api_key: None })
    }
}

#[tauri::command]
fn save_settings(settings: AppSettings) -> Result<(), String> {
    let settings_path = get_settings_path()?;
    
    // Ensure the parent directory exists
    if let Some(parent) = settings_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create settings directory: {}", e))?;
    }
    
    let json = serde_json::to_string_pretty(&settings).map_err(|e| format!("Failed to serialize settings: {}", e))?;
    fs::write(&settings_path, json).map_err(|e| format!("Failed to write settings: {}", e))?;
    
    println!("Settings saved to: {}", settings_path.display());
    Ok(())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    use std::process::Command;
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| format!("Failed to open URL: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    use std::process::Command;
    
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "windows")]
    {
        Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| format!("Failed to open folder: {}", e))?;
    }
    
    Ok(())
}

#[tauri::command]
async fn check_single_mod_update_frontend(mod_info: ModInfo) -> Result<UpdateInfo, String> {
    println!("Frontend verification request for mod: {} ({})", mod_info.name, mod_info.version);
    println!("Update keys: {:?}", mod_info.update_keys);
    let result = check_single_mod_update(&mod_info).await;
    println!("Verification result: {:?}", result);
    result
}

#[tauri::command]
fn update_manifest_version(mods_path: String, mod_folder_name: String, new_version: String) -> Result<(), String> {
    println!("🔧 update_manifest_version called!");
    println!("mods_path: {}", mods_path);
    println!("mod_folder_name: {}", mod_folder_name);
    println!("new_version: {}", new_version);
    use regex::Regex;
    
    let mod_path = Path::new(&mods_path).join(&mod_folder_name);
    let manifest_path = mod_path.join("manifest.json");
    
    if !manifest_path.exists() {
        return Err("Manifest.json not found".to_string());
    }
    
    // Read the current manifest
    let manifest_content = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Failed to read manifest: {}", e))?;
    
    // Use regex to replace the version
    let version_re = Regex::new(r#""Version"\s*:\s*"([^"]+)""#).unwrap();
    let new_manifest = version_re.replace(&manifest_content, &format!(r#""Version": "{}""#, new_version));
    
    // Write the updated manifest back
    fs::write(&manifest_path, new_manifest.as_bytes())
        .map_err(|e| format!("Failed to write updated manifest: {}", e))?;
    
    println!("Updated manifest version for {} to {}", mod_folder_name, new_version);
    Ok(())
}

#[tauri::command]
async fn update_mod(mod_folder_name: String, download_url: String, mods_path: String) -> Result<String, String> {
    use std::io::Write;
    
    println!("Updating mod: {} from {}", mod_folder_name, download_url);
    
    // Get the temp directory for downloads
    let temp_dir = std::env::temp_dir();
    let download_path = temp_dir.join(format!("{}.zip", mod_folder_name));
    
    // Download the file
    let client = reqwest::Client::new();
    let response = client
        .get(&download_url)
        .send()
        .await
        .map_err(|e| format!("Failed to download mod: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()));
    }
    
    let content = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download content: {}", e))?;
    
    // Save to temp file
    let mut file = fs::File::create(&download_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    
    file.write_all(&content)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    
    // Extract the zip file
    let mod_path = Path::new(&mods_path).join(&mod_folder_name);
    
    // Create backup of existing mod
    let backup_path = Path::new(&mods_path).join(format!("{}.backup", mod_folder_name));
    if mod_path.exists() {
        // Remove old backup if it exists
        if backup_path.exists() {
            fs::remove_dir_all(&backup_path)
                .map_err(|e| format!("Failed to remove old backup: {}", e))?;
        }
        
        // Move current mod to backup
        fs::rename(&mod_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;
    }
    
    // Extract new mod
    extract_zip(&download_path, &mod_path)?;
    
    // Clean up temp file
    let _ = fs::remove_file(&download_path);
    
    // Remove backup if extraction was successful
    if backup_path.exists() {
        let _ = fs::remove_dir_all(&backup_path);
    }
    
    Ok(format!("Successfully updated mod: {}", mod_folder_name))
}

fn extract_zip(zip_path: &Path, extract_to: &Path) -> Result<(), String> {
    
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Failed to open zip file: {}", e))?;
    
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("Failed to read zip archive: {}", e))?;
    
    // Create extraction directory
    fs::create_dir_all(extract_to)
        .map_err(|e| format!("Failed to create extraction directory: {}", e))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)
            .map_err(|e| format!("Failed to read zip entry {}: {}", i, e))?;
        
        let outpath = match file.enclosed_name() {
            Some(path) => extract_to.join(path),
            None => continue,
        };
        
        if file.name().ends_with('/') {
            // Directory
            fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            // File
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p)
                        .map_err(|e| format!("Failed to create parent directory: {}", e))?;
                }
            }
            
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Failed to create output file: {}", e))?;
            
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
        }
    }
    
    Ok(())
}

fn get_settings_path() -> Result<PathBuf, String> {
    let config_dir = if cfg!(target_os = "macos") {
        env::var("HOME").map(|home| PathBuf::from(home).join("Library/Application Support"))
    } else if cfg!(target_os = "windows") {
        env::var("APPDATA").map(PathBuf::from)
    } else {
        env::var("HOME").map(|home| PathBuf::from(home).join(".config"))
    }.map_err(|_| "Failed to get config directory")?;
    
    Ok(config_dir.join("stardew-mod-manager").join("settings.json"))
}

async fn check_single_mod_update(mod_info: &ModInfo) -> Result<UpdateInfo, String> {
    println!("Checking updates for mod: {} ({})", mod_info.name, mod_info.version);
    println!("Update keys: {:?}", mod_info.update_keys);
    
    // Get settings for API key
    let settings = get_settings().unwrap_or_else(|_| AppSettings { nexus_api_key: None });
    
    for update_key in &mod_info.update_keys {
        println!("Checking update key: {}", update_key);
        match check_update_key(update_key, &mod_info.version, &settings).await {
            Ok(update_info) => {
                println!("Update check successful for {}: {} -> {}", mod_info.name, update_info.current_version, update_info.latest_version);
                return Ok(update_info);
            }
            Err(e) => {
                println!("Update check failed for {} with key {}: {}", mod_info.name, update_key, e);
                continue;
            }
        }
    }
    
    // No updates found or all checks failed
    println!("No update keys worked for mod: {}", mod_info.name);
    Ok(UpdateInfo {
        current_version: mod_info.version.clone(),
        latest_version: mod_info.version.clone(),
        update_available: false,
        download_url: None,
    })
}

async fn check_update_key(update_key: &str, current_version: &str, settings: &AppSettings) -> Result<UpdateInfo, String> {
    let key_lower = update_key.to_lowercase();
    if key_lower.starts_with("nexus:") {
        let mod_id = update_key[6..].trim(); // Skip "nexus:" and trim whitespace
        check_nexus_update(mod_id, current_version, settings).await
    } else if key_lower.starts_with("github:") {
        let repo = update_key[7..].trim(); // Skip "github:" and trim whitespace
        check_github_update(repo, current_version).await
    } else {
        Err(format!("Unsupported update key format: {}", update_key))
    }
}

async fn check_nexus_update(mod_id: &str, current_version: &str, settings: &AppSettings) -> Result<UpdateInfo, String> {
    let mod_page_url = format!("https://www.nexusmods.com/stardewvalley/mods/{}", mod_id);
    
    // Check if we have an API key
    if let Some(api_key) = &settings.nexus_api_key {
        if !api_key.trim().is_empty() {
            println!("Nexus mod {}: Checking with API", mod_id);
            return check_nexus_with_api(mod_id, current_version, api_key, &mod_page_url).await;
        }
    }
    
    // No API key available, provide manual check
    println!("Nexus mod {}: No API key configured, manual check required", mod_id);
    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: "Manual check".to_string(),
        update_available: false,
        download_url: Some(mod_page_url),
    })
}

async fn check_nexus_with_api(mod_id: &str, current_version: &str, api_key: &str, mod_page_url: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::new();
    let api_url = format!("https://api.nexusmods.com/v1/games/stardewvalley/mods/{}", mod_id);
    
    let response = client
        .get(&api_url)
        .header("apikey", api_key)
        .header("User-Agent", "stardew-mod-manager/1.0")
        .header("Application-Name", "Stardew Valley Mod Manager")
        .header("Application-Version", "1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch from Nexus API: {}", e))?;
    
    if !response.status().is_success() {
        if response.status() == 401 {
            return Err("Invalid Nexus API key".to_string());
        } else if response.status() == 404 {
            return Err(format!("Mod {} not found on Nexus", mod_id));
        } else {
            return Err(format!("Nexus API returned status: {}", response.status()));
        }
    }
    
    let mod_info: NexusModInfo = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Nexus API response: {}", e))?;
    
    let latest_version = &mod_info.version;
    let update_available = version_compare(current_version, latest_version);
    
    println!("Nexus mod {}: API returned version {} (current: {})", mod_id, latest_version, current_version);
    
    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: latest_version.to_string(),
        update_available,
        download_url: Some(mod_page_url.to_string()),
    })
}

async fn check_github_update(repo: &str, current_version: &str) -> Result<UpdateInfo, String> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{}/releases/latest", repo);
    
    let response = client
        .get(&url)
        .header("User-Agent", "stardew-mod-manager")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch GitHub release: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("GitHub API returned status: {}", response.status()));
    }
    
    let release: GitHubRelease = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse GitHub response: {}", e))?;
    
    let latest_version = release.tag_name.trim_start_matches('v');
    let update_available = version_compare(current_version, latest_version);
    
    Ok(UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: latest_version.to_string(),
        update_available,
        download_url: Some(release.html_url),
    })
}

fn version_compare(current: &str, latest: &str) -> bool {
    println!("Version compare: '{}' vs '{}' (current vs latest)", current, latest);
    
    if current == latest {
        println!("  -> Same version, no update needed");
        return false;
    }
    
    let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let latest_parts: Vec<u32> = latest.split('.').filter_map(|s| s.parse().ok()).collect();
    
    println!("  -> Current parts: {:?}, Latest parts: {:?}", current_parts, latest_parts);
    
    let max_len = current_parts.len().max(latest_parts.len());
    
    for i in 0..max_len {
        let current_part = current_parts.get(i).unwrap_or(&0);
        let latest_part = latest_parts.get(i).unwrap_or(&0);
        
        if latest_part > current_part {
            println!("  -> Update available (latest {} > current {})", latest_part, current_part);
            return true;
        } else if latest_part < current_part {
            println!("  -> Local version is newer (latest {} < current {})", latest_part, current_part);
            return false;
        }
    }
    
    println!("  -> Versions are equivalent, no update needed");
    false
}

fn get_stardew_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    #[cfg(target_os = "windows")]
    {
        if let Some(steam_path) = get_steam_path_windows() {
            paths.push(steam_path.join("steamapps/common/Stardew Valley"));
        }
        if let Some(program_files) = std::env::var_os("PROGRAMFILES") {
            paths.push(PathBuf::from(program_files).join("Steam/steamapps/common/Stardew Valley"));
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            // User's Library folder (most common for Steam)
            paths.push(PathBuf::from(&home).join("Library/Application Support/Steam/steamapps/common/Stardew Valley"));
            // Applications folder for standalone installations
            paths.push(PathBuf::from(&home).join("Applications/Stardew Valley.app/Contents/MacOS"));
            // Direct Applications folder
            paths.push(PathBuf::from(&home).join("Applications/Stardew Valley.app"));
        }
        // System-wide Applications folder
        paths.push(PathBuf::from("/Applications/Stardew Valley.app/Contents/MacOS"));
        paths.push(PathBuf::from("/Applications/Stardew Valley.app"));
    }
    
    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(&home).join(".steam/steam/steamapps/common/Stardew Valley"));
            paths.push(PathBuf::from(&home).join(".local/share/Steam/steamapps/common/Stardew Valley"));
        }
    }
    
    paths
}

#[cfg(target_os = "windows")]
fn get_steam_path_windows() -> Option<PathBuf> {
    use std::process::Command;
    
    let output = Command::new("reg")
        .args(&["query", "HKEY_LOCAL_MACHINE\\SOFTWARE\\Valve\\Steam", "/v", "InstallPath"])
        .output()
        .ok()?;
    
    if output.status.success() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().find(|line| line.contains("InstallPath")) {
            if let Some(path_part) = line.split_whitespace().nth(2) {
                return Some(PathBuf::from(path_part));
            }
        }
    }
    
    None
}

fn is_stardew_directory(path: &Path) -> bool {
    if cfg!(target_os = "windows") {
        let executable_names = vec!["Stardew Valley.exe", "StardewValley.exe"];
        return executable_names.iter().any(|name| path.join(name).exists());
    } else if cfg!(target_os = "macos") {
        // Check for .app bundle
        if path.extension().and_then(|s| s.to_str()) == Some("app") {
            let contents_macos = path.join("Contents/MacOS");
            if contents_macos.exists() {
                // Check for known executables
                let executable_names = vec!["StardewValley", "Stardew Valley"];
                if executable_names.iter().any(|name| contents_macos.join(name).exists()) {
                    return true;
                }
                
                // Check for any executable files in Contents/MacOS
                if let Ok(entries) = fs::read_dir(&contents_macos) {
                    for entry in entries.flatten() {
                        if entry.file_type().map_or(false, |ft| ft.is_file()) {
                            return true;
                        }
                    }
                }
            }
        } else {
            // For regular directories, check for executables and .app bundles
            let executable_names = vec!["StardewValley", "Stardew Valley", "StardewModdingAPI", "Stardew Valley.app"];
            if executable_names.iter().any(|name| path.join(name).exists()) {
                return true;
            }
            
            // Check if this directory has a Contents folder (Steam macOS structure)
            let contents_dir = path.join("Contents");
            if contents_dir.exists() {
                let contents_macos = contents_dir.join("MacOS");
                if contents_macos.exists() {
                    // Check for any files in Contents/MacOS
                    if let Ok(entries) = fs::read_dir(&contents_macos) {
                        for entry in entries.flatten() {
                            if entry.file_type().map_or(false, |ft| ft.is_file()) {
                                return true;
                            }
                        }
                    }
                }
                // Also check if there's a Resources folder
                if contents_dir.join("Resources").exists() {
                    return true;
                }
            }
        }
    } else {
        // Linux
        let executable_names = vec!["StardewValley", "Stardew Valley"];
        return executable_names.iter().any(|name| path.join(name).exists());
    }
    
    false
}

fn parse_mod_folder(mod_path: &Path) -> Option<ModInfo> {
    let folder_name = mod_path.file_name()?.to_string_lossy().to_string();
    
    // Skip hidden folders and system folders
    if folder_name.starts_with('.') || folder_name.starts_with("__") {
        return None;
    }
    
    let manifest_path = mod_path.join("manifest.json");
    if manifest_path.exists() {
        match fs::read_to_string(&manifest_path) {
            Ok(manifest_content) => {
                // Use regex to extract values directly from the text
                use regex::Regex;
                
                // Extract Name
                let name_re = Regex::new(r#""Name"\s*:\s*"([^"]+)""#).unwrap();
                let name = name_re.captures(&manifest_content)
                    .and_then(|caps| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| folder_name.clone());
                
                // Extract Version
                let version_re = Regex::new(r#""Version"\s*:\s*"([^"]+)""#).unwrap();
                let version = version_re.captures(&manifest_content)
                    .and_then(|caps| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                // Extract Author
                let author_re = Regex::new(r#""Author"\s*:\s*"([^"]+)""#).unwrap();
                let author = author_re.captures(&manifest_content)
                    .and_then(|caps| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());
                
                // Extract Description
                let description_re = Regex::new(r#""Description"\s*:\s*"([^"]+)""#).unwrap();
                let description = description_re.captures(&manifest_content)
                    .and_then(|caps| caps.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "No description".to_string());
                
                // Extract UpdateKeys
                let mut update_keys = Vec::new();
                let update_keys_re = Regex::new(r#""UpdateKeys"\s*:\s*\[\s*([^\]]*)\s*\]"#).unwrap();
                if let Some(caps) = update_keys_re.captures(&manifest_content) {
                    if let Some(keys_str) = caps.get(1) {
                        let keys_content = keys_str.as_str();
                        let individual_key_re = Regex::new(r#""([^"]+)""#).unwrap();
                        for cap in individual_key_re.captures_iter(keys_content) {
                            if let Some(key) = cap.get(1) {
                                update_keys.push(key.as_str().to_string());
                            }
                        }
                    }
                }
                
                return Some(ModInfo {
                    name,
                    version,
                    author,
                    description,
                    folder_name: folder_name.clone(),
                    enabled: true,
                    update_keys,
                });
            },
            Err(e) => {
                eprintln!("Error reading manifest.json for {}: {}", folder_name, e);
            }
        }
    }
    
    // Check if this looks like a mod directory (has .dll or .cs files)
    if let Ok(entries) = fs::read_dir(mod_path) {
        let has_mod_files = entries
            .flatten()
            .any(|entry| {
                if let Some(ext) = entry.path().extension() {
                    ext == "dll" || ext == "cs"
                } else {
                    false
                }
            });
        
        if has_mod_files {
            return Some(ModInfo {
                name: folder_name.clone(),
                version: "Unknown".to_string(),
                author: "Unknown".to_string(),
                description: "No manifest found - detected mod files".to_string(),
                folder_name,
                enabled: true,
                update_keys: Vec::new(),
            });
        }
    }
    
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            detect_stardew_valley, 
            scan_mods, 
            check_mod_updates,
            get_settings,
            save_settings,
            update_mod,
            open_url,
            open_folder,
            check_single_mod_update_frontend,
            update_manifest_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
