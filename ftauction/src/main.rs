use clap::Parser as _;
use ftauction::BaseArgs;

pub fn main() -> anyhow::Result<()> {
    let args = BaseArgs::parse();
    args.evaluate()
}
