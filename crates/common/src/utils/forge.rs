use eyre::Result;
use tracing::{error, info};

/// Checks if `forge` is installed. If forge is not installed, prompts the user to install it.
pub fn ensure_forge_installed() -> Result<()> {
    // ensure `forge` is installed with `which forge`
    if which::which("forge").is_err() {
        const YELLOW_ANSI_CODE: &str = "\u{001b}[33m";
        const LIGHT_GRAY_ANSI_CODE: &str = "\u{001b}[90m";
        const RESET_ANSI_CODE: &str = "\u{001b}[0m";
        print!(
                "{LIGHT_GRAY_ANSI_CODE}{}  {YELLOW_ANSI_CODE}WARN{RESET_ANSI_CODE} `forge` is not installed. would you like to install it now? [Y/n] ",
                // include microsecond precision
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, true),
            );
        std::io::Write::flush(&mut std::io::stdout())?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "" {
            info!("Installing foundryup via `curl -L https://foundry.paradigm.xyz | bash`");

            // silently install foundryup via bash
            let status = std::process::Command::new("bash")
                .arg("-c")
                .arg("curl -L https://foundry.paradigm.xyz | bash")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .expect("Failed to install `foundryup`.");

            if !status.success() {
                error!("Failed to install `foundryup`.");
                std::process::exit(1);
            }

            // silently run foundryup
            info!("Installing forge via `foundryup`");
            let status = std::process::Command::new("foundryup")
                .stderr(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .status()
                .expect("Failed to install `forge`.");

            if !status.success() {
                error!("Failed to install `forge`.");
                std::process::exit(1);
            }

            info!("Successfully installed `forge`.");
        } else {
            error!("`forge` is required by this command. Please install it and try again.");
            std::process::exit(1);
        }
    };
    Ok(())
}
