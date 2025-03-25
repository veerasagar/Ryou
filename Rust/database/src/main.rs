mod bptree;
mod cli;
mod storage;

use cli::start_cli;

fn main() -> std::io::Result<()> {
    start_cli()
}