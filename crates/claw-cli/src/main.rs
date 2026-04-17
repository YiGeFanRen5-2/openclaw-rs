use clap::Parser;
use openclaw_cli::cli::Cli;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let output = cli.run().await?;
    println!("{output}");
    Ok(())
}
