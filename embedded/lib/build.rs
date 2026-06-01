use std::fs;

fn main() {
    // env setup.
    println!("cargo:rerun-if-changed=.env");
    let contents = fs::read_to_string("../../.env").expect(".env missing");
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, val)) = line.split_once('=') {
            println!("cargo:rustc-env={}={}", key.trim(), val.trim());
        }
    }
}
