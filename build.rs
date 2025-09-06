use std::process::Command;

fn main() {
    #[cfg(target_os = "windows")]
    {
        winfsp::build::winfsp_link_delayload();
    }
    let git_hash: String = Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or("none".to_owned());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}
