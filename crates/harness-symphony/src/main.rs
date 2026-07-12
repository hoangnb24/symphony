mod agent;
mod auto;
mod changeset;
mod config;
mod doctor;
// The adapter is integrated into consumers incrementally within US-093. Keep
// its complete typed surface available before the first call site lands.
#[allow(dead_code)]
mod harness_protocol;
mod interface;
mod pr;
mod retention;
mod run;
mod state;
mod sync;
mod web;
mod work;

use clap::Parser;

fn main() {
    let cli = interface::Cli::parse();
    if let Err(error) = interface::run(cli) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
