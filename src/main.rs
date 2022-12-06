use std::collections::HashMap;
use std::io;

mod utils;
mod vpk;

fn get_selection() -> &'static str {
    let terrains: HashMap<i32, (&str, &str)> = HashMap::from([
        (1, ("Desert Terrain", "dota_desert.vpk")),
        (2, ("The King's New Journey", "dota_journey.vpk")),
        (3, ("Immortal Gardens", "dota_coloseum.vpk")),
        (4, ("Overgrown Empire", "dota_jungle.vpk")),
        (5, ("Reef's Edge", "dota_reef.vpk")),
        (6, ("Sanctums of the Divine", "dota_ti10.vpk")),
        (7, ("The Emerald Abyss", "dota_cavern.vpk")),
        (8, ("Seasonal Terrain, Autumn", "dota_autumn.vpk")),
        (9, ("Seasonal Terrain, Winter", "dota_winter.vpk")),
        (10, ("Seasonal Terrain, Spring", "dota_spring.vpk")),
        (11, ("Seasonal Terrain: Summer", "dota_summer.vpk")),
    ]);
    println!("Select a Terrain to apply \n");
    for i in 1..terrains.len() as i32 {
        println!("[{}] - {}", i, terrains.get(&i).unwrap().0);
    }
    println!();

    println!("Enter a number: ");
    let mut selection = String::new();
    io::stdin()
        .read_line(&mut selection)
        .expect("Failed to read input.");
    let selection: i32 = selection.trim().parse().expect("Invalid input.");
    if terrains.contains_key(&selection) {
        let terrain = terrains.get(&selection).unwrap();
        println!("Selected: {}. Applying terrain...", terrain.0);
        return terrain.1;
    } else {
        eprintln!("Invalid selection.");
        std::process::exit(1);
    };
}

fn main() {
    println!("-- Dota Terrain Mod (https://github.com/ObsoleteXero/Dota-Terrain-Mod) --\n");
    let terrain = get_selection();
    let (base_path, target_path, out_path) = utils::create_paths(terrain).unwrap();

    let out_file = vpk::create_terrain(base_path, target_path);

    std::fs::create_dir_all(out_path.parent().unwrap()).unwrap();
    std::fs::write(out_path, &out_file).unwrap();

    println!("Done. Launch Dota 2 with the \"-language tempcontent\" launch option.");
    println!("\nPress any key to exit.");
    utils::pause();
}
