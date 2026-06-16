mod config;
mod doctor;
mod interface;
mod run;
mod state;
mod work;

use clap::Parser;

fn main() {
    let cli = interface::Cli::parse();
    if let Err(error) = interface::run(cli) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
