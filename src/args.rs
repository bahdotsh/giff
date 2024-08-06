use clap::Parser;

#[derive(Parser)]
#[command(author="bahdotsh", version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "main")]
    pub branch: String,
}
