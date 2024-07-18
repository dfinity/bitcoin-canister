use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the value of the environment variable
    let chunk_hashes: Vec<String> = option_env!("CHUNK_HASHES_PATH")
        .map(|f| {
            fs::read_to_string(f)
                .expect("chunk hashes file not found.")
                .split('\n')
                .filter(|s| !s.is_empty())
                .map(|hash| {
                    // Each hash is 32-bytes represented in hex, which would be 64 bytes.
                    assert_eq!(hash.len(), 64);
                    hash.to_string()
                })
                .collect()
        })
        .unwrap_or_default();

    // Generate Rust code with the file content
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable not set");
    let dest_path = PathBuf::from(out_dir).join("chunk_hashes.rs");

    fs::write(
        dest_path,
        format!(
            "const CHUNK_HASHES: &[&str] = &[{}];",
            chunk_hashes.join(",")
        ),
    )
    .expect("Failed to write file");
}
