//! clap [Args](clap::Args) for logging configuration.
// Mostly taken from [reth](https://github.com/paradigmxyz/reth)

use clap::{ArgAction, Args, ValueEnum};
use shadow_tracing::{
    tracing_subscriber::filter::Directive, FileWorkerGuard, LayerInfo, LogFormat, ShadowTracer,
    Tracer,
};
use std::{
    fmt::{self, Display},
    str::FromStr,
};
use tracing::{level_filters::LevelFilter, Level};

#[derive(Debug, Args)]
#[clap(next_help_heading = "LOGGING")]
pub(crate) struct LogArgs {
    /// The format to use for logs written to stdout.
    #[clap(long = "log.stdout.format", value_name = "FORMAT", global = true, default_value_t = LogFormat::Terminal)]
    pub(crate) log_stdout_format: LogFormat,

    /// The filter to use for logs written to stdout.
    #[clap(long = "log.stdout.filter", value_name = "FILTER", global = true, default_value = "")]
    pub(crate) log_stdout_filter: String,

    /// Sets whether or not the formatter emits ANSI terminal escape codes for colors and other
    /// text formatting.
    #[clap(
        long,
        value_name = "COLOR",
        global = true,
        default_value_t = ColorMode::Always
    )]
    pub(crate) color: ColorMode,

    /// The verbosity settings for the tracer.
    #[clap(flatten)]
    pub(crate) verbosity: Verbosity,
}

impl LogArgs {
    /// Creates a [LayerInfo] instance.
    fn layer(&self, format: LogFormat, filter: String, use_color: bool) -> LayerInfo {
        LayerInfo::new(
            format,
            self.verbosity.directive().to_string(),
            filter,
            if use_color { Some(self.color.to_string()) } else { None },
        )
    }

    /// Initializes tracing with the configured options from cli args.
    pub(crate) fn init_tracing(&self) -> eyre::Result<Option<FileWorkerGuard>> {
        let mut tracer = ShadowTracer::new();

        let stdout = self.layer(self.log_stdout_format, self.log_stdout_filter.clone(), true);
        tracer = tracer.with_stdout(stdout);

        let guard = tracer.init()?;
        Ok(guard)
    }
}

/// The color mode for the cli.
#[derive(Debug, Copy, Clone, ValueEnum, Eq, PartialEq)]
pub(crate) enum ColorMode {
    /// Colors on
    Always,
    /// Colors on
    Auto,
    /// Colors off
    Never,
}

impl Display for ColorMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorMode::Always => write!(f, "always"),
            ColorMode::Auto => write!(f, "auto"),
            ColorMode::Never => write!(f, "never"),
        }
    }
}

impl FromStr for ColorMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "always" => Ok(ColorMode::Always),
            "auto" => Ok(ColorMode::Auto),
            "never" => Ok(ColorMode::Never),
            _ => Err(format!("Invalid color mode: {}", s)),
        }
    }
}

#[derive(Debug, Copy, Clone, Args)]
#[clap(next_help_heading = "DISPLAY")]
pub(crate) struct Verbosity {
    /// Set the minimum log level.
    ///
    /// -v     Warnings & Errors
    /// -vv    Info
    /// -vvv   Debug
    /// -vvvv  Traces (warning: very verbose!)
    #[clap(short, long, action = ArgAction::Count, global = true, default_value_t = 1, verbatim_doc_comment, help_heading = "DISPLAY")]
    verbosity: u8,

    /// Silence all log output.
    #[clap(long, alias = "silent", short = 'q', global = true, help_heading = "DISPLAY")]
    quiet: bool,
}

impl Verbosity {
    /// Get the corresponding [Directive] for the given verbosity, or none if the verbosity
    /// corresponds to silent.
    pub(crate) fn directive(&self) -> Directive {
        if self.quiet {
            LevelFilter::OFF.into()
        } else {
            let level = match self.verbosity - 1 {
                0 => Level::INFO,
                1 => Level::DEBUG,
                _ => Level::TRACE,
            };

            level.into()
        }
    }
}
