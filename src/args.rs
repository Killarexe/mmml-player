use std::path::PathBuf;
use clap::Parser;

// A µMML player / synthesier
#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct MMMLPlayerArgs {
    /// Input file in .mbf, .mmmldata or .mmml
    pub input_file: PathBuf,
    /// Output file (In wav format)
    #[arg(short, long)]
    output_file: Option<PathBuf>,
    /// Mute channel 1
    #[arg(long)]
    pub ch1_muted: bool,
    /// Mute channel 2
    #[arg(long)]
    pub ch2_muted: bool,
    /// Mute channel 3
    #[arg(long)]
    pub ch3_muted: bool,
    /// Mute channel 4
    #[arg(long)]
    pub ch4_muted: bool
}

impl MMMLPlayerArgs {
    pub fn get_output_path(&self) -> PathBuf {
        self.output_file.clone().unwrap_or(self.input_file.with_extension("wav"))
    }
}
