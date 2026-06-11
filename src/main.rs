mod cli;
mod compiler;
mod error;
mod lexer;
mod repl;
mod value;
mod vm;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    cli::run_cli()
}
