use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-env-changed=AWMKIT_LAUNCHER_PAYLOAD");

    let out_dir = match env::var("OUT_DIR") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("OUT_DIR not set: {err}");
            std::process::exit(1);
        }
    };
    let generated_path = Path::new(&out_dir).join("launcher_payload.rs");

    let maybe_payload = env::var("AWMKIT_LAUNCHER_PAYLOAD")
        .ok()
        .filter(|value| !value.trim().is_empty());

    let generated = if let Some(payload_path) = maybe_payload {
        println!("cargo:rerun-if-changed={payload_path}");
        let payload_bytes = match fs::read(&payload_path) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("failed to read launcher payload {payload_path}: {err}");
                std::process::exit(1);
            }
        };
        let payload_hash = hex::encode(Sha256::digest(&payload_bytes));
        let copied_payload_path = Path::new(&out_dir).join("launcher_payload.zip");
        if let Err(err) = fs::write(&copied_payload_path, payload_bytes) {
            eprintln!(
                "failed to write copied payload {}: {err}",
                copied_payload_path.display()
            );
            std::process::exit(1);
        }
        format!(
            "pub const PAYLOAD: &[u8] = include_bytes!(concat!(env!(\"OUT_DIR\"), \"/launcher_payload.zip\"));\n\
             pub const PAYLOAD_SHA256: &str = \"{payload_hash}\";\n"
        )
    } else {
        "pub const PAYLOAD: &[u8] = &[];\n\
         pub const PAYLOAD_SHA256: &str = \"\";\n"
            .to_string()
    };

    let mut file = match fs::File::create(&generated_path) {
        Ok(value) => value,
        Err(err) => {
            eprintln!(
                "failed to create generated payload file {}: {err}",
                generated_path.display()
            );
            std::process::exit(1);
        }
    };
    if let Err(err) = file.write_all(generated.as_bytes()) {
        eprintln!(
            "failed to write generated payload file {}: {err}",
            generated_path.display()
        );
        std::process::exit(1);
    }
}
