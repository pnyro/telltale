mod daemon;
mod output;
mod sources;

use std::error::Error;

fn print_usage() {
    eprintln!("Usage: telltale daemon");
}

fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        Some("daemon") => daemon::run(),
        Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => {
            eprintln!("Unknown command: {other}");
            print_usage();
            Ok(())
        }
    }
}

fn main() {
    if let Err(err) = run() {
        eprintln!("fatal: {err}");
        std::process::exit(1);
    }
}
