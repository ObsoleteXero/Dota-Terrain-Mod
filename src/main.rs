mod utils;

fn main() {
    let steam_path: String;
    match utils::get_dota_path() {
        Ok(path) => steam_path = path.into_os_string().into_string().unwrap(),
        Err(err) => {
            eprintln!("{err} Program will exit.");
            std::process::exit(1)
        },
    }


    println!("{steam_path}");
}
