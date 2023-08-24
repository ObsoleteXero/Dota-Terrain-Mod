pub mod utils {
    use regex::Regex;
    use std::error::Error;
    use std::fmt::{Display, Formatter};
    use std::fs;
    use std::io::Read;
    use std::path::{Path, PathBuf};
    use winreg::RegKey;

    #[derive(Debug)]
    pub(crate) enum TMError {
        SteamNotFound,
        DotaNotFound,
        InternalError(std::io::Error),
    }

    impl Display for TMError {
        fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
            write!(f, "Dota-Terrain-Mod has encountered an error.")
        }
    }

    impl Error for TMError {}

    /// Object representing a dota installation. Exists to encapsulate the paths and identify
    /// if `dota_path` cannot be found
    pub struct Dota {
        pub(crate) dota_path: PathBuf,
        pub(crate) base_path: Option<PathBuf>,
        pub(crate) target_path: Option<PathBuf>,
        pub(crate) out_path: Option<PathBuf>,
    }

    impl Dota {
        /// On initialization of a `Dota` instance, tries to locate the dota installation and
        /// sets the attribute accordingly
        pub(crate) fn new() -> Result<Self, TMError> {
            let steam_path = get_steam_path()?;
            let libtext = load_libraries(steam_path)?;
            let dota_path = get_dota_path(libtext)?;

            Ok(Dota {
                dota_path,
                base_path: None,
                target_path: None,
                out_path: None,
            })
        }

        /// Using the `dota_path` which is assumed to exist if this function is called,
        /// populate the other attributes by creating the paths to the base `dota.vpk`,
        /// the given target terrain vpk, and the file path where the output vpk will be written
        pub(crate) fn build_paths(&mut self, target: &str) {
            let dota_path = &self.dota_path;
            let base_path = get_base_path(dota_path);
            let target_path = get_target_path(dota_path, target);
            let out_path = get_out_path(dota_path);

            self.base_path = Some(base_path);
            self.target_path = Some(target_path);
            self.out_path = Some(out_path);
        }
    }

    /// Reads the windows registry and returns the Steam installation directory
    fn get_steam_path() -> Result<PathBuf, TMError> {
        let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        match hkcu.open_subkey("Software\\Valve\\Steam") {
            Ok(steam_key) => match steam_key.get_value::<String, _>("SteamPath") {
                Ok(steam_path) => Ok(PathBuf::from(steam_path)),
                Err(_) => Err(TMError::SteamNotFound),
            },
            Err(_) => Err(TMError::SteamNotFound),
        }
    }

    /// Given the Steam installation path, return the contents of `libraryfolders.vdf`
    fn load_libraries(steam_path: PathBuf) -> Result<String, TMError> {
        let library_folders = Path::new(&steam_path)
            .join("steamapps")
            .join("libraryfolders.vdf");

        fs::read_to_string(library_folders).map_err(|_| TMError::SteamNotFound)
    }

    /// Given the contents of `libraryfolders.vdf`, returns the path to the dota installation
    /// directory.
    fn get_dota_path(lib_file: String) -> Result<PathBuf, TMError> {
        let lib_regex = Regex::new(r#"(?m)"\d"\n\s\{\n[\s\S]+?}\n\s}"#).unwrap(); // "\d"\n\s\{\n[\s\S]+?\}\n\s}
        let appid_regex = Regex::new(r#"(?m)\t{3}"570"\t{2}"\d+"\n"#).unwrap(); // \t{3}"570"\t{2}"\d+"\n
        let path_regex = Regex::new(r#"(?m)"(\w+:\\\\.+)"\n"#).unwrap(); // "(\w+:\\\\.+)"\n

        for lib in lib_regex.captures_iter(&lib_file) {
            if appid_regex.is_match(&lib[0]) {
                return match path_regex.captures(&lib[0]) {
                    Some(capture) => match capture.get(1) {
                        Some(path) => {
                            let lib_path_str = path.as_str();
                            let lib_path = Path::new(lib_path_str)
                                .canonicalize()
                                .map_err(TMError::InternalError)?;
                            Ok(lib_path
                                .join("steamapps")
                                .join("common")
                                .join("dota 2 beta")
                                .join("game"))
                        }
                        None => Err(TMError::DotaNotFound),
                    },
                    None => Err(TMError::DotaNotFound),
                };
            };
        }
        Err(TMError::DotaNotFound)
    }

    /// Create the path to the base terrain vpk using the dota installation directory
    fn get_base_path(dota_path: &PathBuf) -> PathBuf {
        dota_path.join("dota").join("maps").join("dota.vpk")
    }

    /// Create output path from the patched vpk using the dota installation directory
    fn get_out_path(dota_path: &PathBuf) -> PathBuf {
        dota_path
            .join("dota_tempcontent")
            .join("maps")
            .join("dota.vpk")
    }

    /// Create the path to the selected terrain vpk using the dota installation directory
    fn get_target_path(dota_path: &PathBuf, target: &str) -> PathBuf {
        dota_path.join("dota").join("maps").join(target)
    }

    pub(crate) fn pause() {
        std::io::stdin().read(&mut [0_u8]).unwrap();
    }
}

pub use self::utils::*;
