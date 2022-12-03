mod utils;
mod vpk;
// vpk -> patch_vpk(base, target), write_vpk(buffer, outfile)

// Print all terrains, get selection
// Get path of base, target and outfile
// Write outfile
// Print message and exit

fn test_utils() {
    let (base_path, target_path, out_path) = utils::create_paths(utils::Terrains::Img).unwrap();

    let a = base_path.display();
    let b = target_path.display();
    let c = out_path.display();

    println!("{a}, \n {b} \n {c}");
}

fn main() {
    vpk::testvpk();
}
