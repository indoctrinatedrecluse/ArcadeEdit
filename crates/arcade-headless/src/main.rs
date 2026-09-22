//! Headless entry point. This binary intentionally has no UI dependency.

use arcade_protocol::PROTOCOL_VERSION;

fn main() {
    let argument = std::env::args().nth(1);

    match argument.as_deref() {
        Some("--version") | Some("-V") => {
            println!(
                "arcade-headless {} (protocol {})",
                env!("CARGO_PKG_VERSION"),
                PROTOCOL_VERSION
            );
        }
        _ => {
            println!("arcade-headless scaffold — commands will operate on arcade-core.");
        }
    }
}
