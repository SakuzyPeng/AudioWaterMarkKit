#[cfg(feature = "launcher")]
fn main() {
    match awmkit::launcher::run() {
        Ok(code) => std::process::exit(code),
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
}

#[cfg(not(feature = "launcher"))]
fn main() {
    eprintln!(
        "awmkit launcher not enabled. Build with: cargo build --features launcher --bin awmkit"
    );
    std::process::exit(1);
}
