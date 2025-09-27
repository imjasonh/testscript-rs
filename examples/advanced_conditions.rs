//! Example demonstrating network and advanced condition support

use testscript_rs::testscript;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example 1: Basic usage - all conditions detected automatically
    testscript::run("testdata").execute()?;

    // Example 2: Manual condition setting with environment variables

    // Set a test environment variable
    std::env::set_var("CI", "true");

    testscript::run("testdata")
        .condition("net", check_network_available())
        .condition("docker", command_exists("docker"))
        .execute()?;

    Ok(())
}

/// Check if network is available
fn check_network_available() -> bool {
    std::process::Command::new("ping")
        .args(if cfg!(windows) {
            vec!["-n", "1", "-w", "1000", "1.1.1.1"]
        } else {
            vec!["-c", "1", "-W", "1", "1.1.1.1"]
        })
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if a command exists
fn command_exists(program: &str) -> bool {
    #[cfg(windows)]
    let check_cmd = "where";
    #[cfg(not(windows))]
    let check_cmd = "which";

    std::process::Command::new(check_cmd)
        .arg(program)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}
