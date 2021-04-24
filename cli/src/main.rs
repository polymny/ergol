use std::env::{current_dir, set_current_dir};
use std::fs::{read_dir, read_to_string, File};
use std::process::exit;

use ergol_cli::Table;

fn main() {
    loop {
        let current_dir = match current_dir() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("Cannot read current dir: {}", e);
                exit(1);
            }
        };

        if File::open(current_dir.join("Cargo.toml")).is_ok() {
            break;
        }

        match current_dir.parent().map(|x| set_current_dir(x)) {
            Some(Ok(_)) => (),
            _ => {
                eprintln!("Cannot find a Cargo.toml");
                exit(1);
            }
        }
    }

    let dir = match read_dir("migrations/current") {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Cannot read migration directory: {}", e);
            exit(1);
        }
    };

    for file in dir {
        let file = match file.map(|x| read_to_string(x.path())) {
            Ok(Ok(file)) => file,
            _ => {
                eprintln!("Cannot open migration file");
                exit(1);
            }
        };

        let tables: Vec<Table> = match serde_json::from_str(&file) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Couldn't parse json: {}", e);
                exit(1);
            }
        };

        for table in tables {
            println!("{}", table.create_table());
        }
    }
}
