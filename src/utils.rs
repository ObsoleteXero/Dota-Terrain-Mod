use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use winreg::RegKey;

enum TMError {
    SteamNotFound,
    DotaNotFound,
    GenericError,
}

fn get_steam_path() -> Result<String, TMError> {
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    match hkcu.open_subkey("Software\\Valve\\Steam") {
        Ok(steam_key) => match steam_key.get_value("SteamPath") {
            Ok(steam_path) => Ok(steam_path),
            Err(_) => Err(TMError::SteamNotFound),
        },
        Err(_) => Err(TMError::SteamNotFound),
    }
}

fn get_dota_path() -> Result<PathBuf, TMError> {
    let steam_path = get_steam_path()?;

    let library_folders = Path::new(&steam_path)
        .join("steamapps")
        .join("libraryfolders.vdf");

    let mut lib_file = match File::open(&library_folders) {
        Ok(file) => file,
        Err(_) => return Err(TMError::GenericError),
    };
    let mut lib_file_text = String::new();
    lib_file.read_to_string(&mut lib_file_text).unwrap();

    let lib_regex = Regex::new(r#"(?m)"\d"\n\s\{\n[\s\S]+?\}\n\s}"#).unwrap(); // "\d"\n\s\{\n[\s\S]+?\}\n\s}
    let appid_regex = Regex::new(r#"(?m)\t{3}"570"\t{2}"\d+"\n"#).unwrap(); // \t{3}"570"\t{2}"\d+"\n
    let path_regex = Regex::new(r#"(?m)"(\w+:\\\\.+)"\n"#).unwrap(); // "(\w+:\\\\.+)"\n

    for lib in lib_regex.captures_iter(&lib_file_text) {
        if appid_regex.is_match(&lib[0]) {
            match path_regex.captures(&lib[0]).unwrap().get(1) {
                Some(path) => {
                    let lib_path_str = path.as_str();
                    let lib_path = Path::new(lib_path_str).canonicalize().unwrap();
                    return Ok(lib_path
                        .join("steamapps")
                        .join("common")
                        .join("dota 2 beta")
                        .join("game"));
                }
                None => return Err(TMError::DotaNotFound),
            };
        };
    }
    Err(TMError::DotaNotFound)
}

pub fn create_paths(target: &str) -> Option<(PathBuf, PathBuf, PathBuf)> {
    let dota_path: PathBuf = match get_dota_path() {
        Ok(path) => Path::new(&path).to_path_buf(),
        Err(_) => return None,
    };

    let base_path = dota_path.join("dota").join("maps").join("dota.vpk");
    let target_path = dota_path.join("dota").join("maps").join(target);
    let out_path = dota_path
        .join("dota_tempcontent")
        .join("maps")
        .join("dota.vpk");

    Some((base_path, target_path, out_path))
}

pub fn pause() {
    std::io::stdin().read(&mut [0_u8]).unwrap();
}
