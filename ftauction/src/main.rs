use clap::Parser as _;
use ftauction::BaseArgs;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let args = BaseArgs::parse();
    args.evaluate().await
}
