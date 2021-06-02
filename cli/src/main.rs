use std::env::args;
use std::error::Error;
use std::process::exit;

use colored::*;

fn print_version() {
    println!("ergol {}", env!("CARGO_PKG_VERSION"));
}

fn print_help() {
    println!(
        r#"{name} {version}
{description}

{USAGE}
    {command} [SUBCOMMAND]

{FLAGS}
    {help_short}, {help_long}       Prints help information
    {version_short}, {version_long}    Prints version information

{SUBCOMMANDS}
    {hint}       Gives a hint of the current migration
    {save}       Saves the current migration
    {delete}     Deletes everything in the database
    {migrate}    Runs all the migrations in the database
    {reset}      Deletes everything in the database and recreates an empty database"#,
        name = "ergol".green(),
        version = env!("CARGO_PKG_VERSION"),
        description = env!("CARGO_PKG_DESCRIPTION"),
        USAGE = "USAGE:".yellow(),
        command = "ergol",
        FLAGS = "FLAGS:".yellow(),
        help_short = "-h".green(),
        help_long = "--help".green(),
        version_short = "-v".green(),
        version_long = "--version".green(),
        SUBCOMMANDS = "SUBCOMMANDS:".yellow(),
        save = "save".green(),
        hint = "hint".green(),
        delete = "delete".green(),
        migrate = "migrate".green(),
        reset = "reset".green(),
    );
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}", e);
        exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let args = args().collect::<Vec<_>>();

    // The first argument is the name of the binary, the second one is the command
    if args.len() < 2 {
        print_help();
        exit(1);
    }

    if args.contains(&String::from("-h")) || args.contains(&String::from("--help")) {
        print_help();
        exit(0);
    }

    if args.contains(&String::from("-v")) || args.contains(&String::from("--version")) {
        print_version();
        exit(0);
    }

    let cargo_toml = ergol_cli::find_cargo_toml().expect("couldn't find Cargo.toml");

    match args[1].as_ref() {
        "hint" => println!("{}", ergol_cli::current_diff(cargo_toml)?.hint()),
        "save" => ergol_cli::save(cargo_toml.join("migrations"))?,
        "migrate" => ergol_cli::migrate(cargo_toml).await?,
        "delete" => ergol_cli::delete(cargo_toml).await?,
        "reset" => ergol_cli::reset(cargo_toml).await?,

        command => {
            // Unknwon command
            eprintln!(
                "{}: {}{}{}",
                "error".bold().red(),
                "command \"",
                command,
                "\" does not exist."
            );
            print_help();
            exit(1);
        }
    }

    Ok(())
}
