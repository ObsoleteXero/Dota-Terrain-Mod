use winreg::RegKey;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::fs::File;
use regex::Regex;
use std::error::Error;

fn get_steam_path() -> Result<String, Box<dyn Error>> {
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let steam_key = hkcu.open_subkey("Software\\Valve\\Steam")?;
    let steam_path: String = steam_key.get_value("SteamPath")?;
    Ok(steam_path)
}

pub fn get_dota_path() -> Result<PathBuf, Box<dyn Error>> {
    let steam_path: String;
    match get_steam_path() {
        Ok(path) => steam_path = path,
        Err(_) => return Err("Steam Installation not found.")?,
    }

    let library_folders = Path::new(&steam_path).join("steamapps").join("libraryfolders.vdf");
    let mut lib_file = File::open(&library_folders)?;
    let mut lib_file_text = String::new();
    lib_file.read_to_string(&mut lib_file_text)?;

    let lib_regex = Regex::new(r#"(?m)"\d"\n\s\{\n[\s\S]+?\}\n\s}"#).unwrap(); // "\d"\n\s\{\n[\s\S]+?\}\n\s}
    let appid_regex = Regex::new(r#"(?m)\t{3}"570"\t{2}"\d+"\n"#).unwrap(); // \t{3}"570"\t{2}"\d+"\n
    let path_regex = Regex::new(r#"(?m)"(\w+:\\\\.+)"\n"#).unwrap(); // "(\w+:\\\\.+)"\n

    for lib in lib_regex.captures_iter(&lib_file_text) {
        if appid_regex.is_match(&lib[0]) {
            match path_regex.captures(&lib[0]).unwrap().get(1) {
                Some(path) => {
                    let lib_path_str = path.as_str();
                    let lib_path = Path::new(lib_path_str).canonicalize().unwrap();
                    return Ok(lib_path.join("steamapps").join("common").join("dota 2 beta").join("game"))
                },
                None => return Err("Dota Installation not found.")?
            };
        };
    }
    Err("Dota Installation not found.")?
}

// fn create_paths() -> Path {
//     let dota_path = match get_dota_path() {
//         Ok(path) => Path::new(&path),
//         Err(_) => panic!(),
//     };
// }