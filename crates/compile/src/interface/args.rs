use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Compile a shadowed contract with the original contract settings")]
pub struct CompileArgs {
    /// The project's root path
    #[clap(short, long, default_value = ".", hide_default_value = true)]
    pub root: String,
}
