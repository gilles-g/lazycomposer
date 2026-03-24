use std::process::Command;

fn main() {
    // Git commit hash
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=LC_GIT_COMMIT={commit}");

    // Build date (UTC, ISO 8601)
    let build_date = Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=LC_BUILD_DATE={build_date}");

    // OS and arch
    println!("cargo:rustc-env=LC_OS={}", std::env::consts::OS);
    println!("cargo:rustc-env=LC_ARCH={}", std::env::consts::ARCH);

    // Rebuild if git HEAD changes
    println!("cargo:rerun-if-changed=.git/HEAD");
}
