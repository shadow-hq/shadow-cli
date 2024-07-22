use clap::Parser;

/// Arguments for the `config` subcommand
#[derive(Debug, Clone, Parser)]
#[clap(
    about = "Display or edit your shadow CLI configuration.",
    override_usage = "shadow config [OPTIONS]"
)]
pub struct ConfigArgs {
    /// The target key to update.
    #[clap(required = false, default_value = "", hide_default_value = true)]
    pub key: String,

    /// The value to set the key to.
    #[clap(required = false, default_value = "", hide_default_value = true)]
    pub value: String,

    /// Whether to enter interactive mode.
    #[clap(long, short)]
    pub interactive: bool,
}
