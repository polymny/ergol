fn main() {
    let cargo_toml = ergol_cli::find_cargo_toml().expect("couldn't find Cargo.toml");

    let last = ergol_cli::last_saved_state(cargo_toml.join("migrations"))
        .expect("failed to read db state");

    let current = ergol_cli::state_from_dir(cargo_toml.join("migrations/current"))
        .expect("failed to read db state");

    println!("{}", ergol_cli::diff(last, current).hint());
}
