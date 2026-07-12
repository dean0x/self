mod cmd_doctor;
mod cmd_init;
mod cmd_status;
mod cmd_uninstall;
mod error;
mod markers;
mod paths;
mod registry;
mod runlog;
mod settings;
mod templates;

use error::Result;

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [cmd] if cmd == "init" => cmd_init::run(false),
        [cmd, flag] if cmd == "init" && flag == "--reset" => cmd_init::run(true),
        [cmd] if cmd == "status" => cmd_status::run(),
        [cmd] if cmd == "doctor" => cmd_doctor::run(),
        [cmd] if cmd == "uninstall" => cmd_uninstall::run(),
        _ => {
            eprintln!("Usage: self <command> [--reset]");
            eprintln!();
            eprintln!("Commands:");
            eprintln!("  init [--reset]  Create ~/.self, seed corpus, install adapters.");
            eprintln!("                  --reset restores factory content (requires git repo).");
            eprintln!("  status          Show registry counts, skill stats, and run trends.");
            eprintln!(
                "  doctor          Audit installation health (read-only; exits 1 if findings)."
            );
            eprintln!(
                "  uninstall       Remove the marker block and agent files (leaves ~/.self)."
            );
            std::process::exit(2);
        }
    }
}
