use crate::constants::{GREEN_ANSI_COLOR, RED_ANSI_COLOR, RESET_ANSI_COLOR};
use crate::{ConfigArgs, Configuration};
use eyre::Result;

/// The `config` command is used to display and edit the current configuration.
/// Note @jon-becker: Not using tracing here because it doesnt look good in interactive mode.
pub fn config(args: ConfigArgs) -> Result<()> {
    if args.interactive {
        Configuration::from_interactive()?;
        return Ok(());
    }

    if !args.key.is_empty() {
        if !args.value.is_empty() {
            let mut config = Configuration::load()?;
            match config.set(&args.key, &args.value) {
                Ok(_) => {
                    println!(
                        "{GREEN_ANSI_COLOR}Success: {RESET_ANSI_COLOR}'{}' set to '{}'.",
                        args.key, args.value
                    );
                    println!("Configuration: {}\n", serde_json::to_string_pretty(&config)?);
                }
                Err(e) => println!("{RED_ANSI_COLOR}Error: {RESET_ANSI_COLOR}{}", e),
            };
        } else {
            println!("{RED_ANSI_COLOR}Error: {RESET_ANSI_COLOR}use `shadow config <KEY> <VALUE>` to set a key/value pair, or `shadow config --interactive` to enter interactive mode.");
        }
    } else {
        let config = Configuration::load()?;
        println!("Configuration: {}\n", serde_json::to_string_pretty(&config)?);
        println!("{GREEN_ANSI_COLOR}Hint: {RESET_ANSI_COLOR}use `shadow config <KEY> <VALUE>` to set a key/value pair, or `shadow config --interactive` to enter interactive mode.");
    }

    Ok(())
}
