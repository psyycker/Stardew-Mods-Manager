use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub folder_name: String,
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StardewInfo {
    pub game_path: Option<PathBuf>,
    pub mods_path: Option<PathBuf>,
    pub found: bool,
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
    
    Ok(mods)
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
                
                return Some(ModInfo {
                    name,
                    version,
                    author,
                    description,
                    folder_name: folder_name.clone(),
                    enabled: true,
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
            });
        }
    }
    
    None
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![detect_stardew_valley, scan_mods])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
