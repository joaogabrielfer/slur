use std::{error::Error, fs, path::PathBuf};

use clap::{Parser, Subcommand};

use crate::{
    compiler::{PbcFile, compile, compile_to_pbc, run},
    lexer::tokenize,
    repl, vm,
};

#[derive(Parser, Debug)]
#[command(name = "pasm", version, about = "pasm compiler, REPL, and pvm runner")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[arg(value_name = "FILE")]
    file: Option<PathBuf>,

    #[arg(short = 'i', long = "interactive", value_name = "FILE")]
    interactive: Option<PathBuf>,

    #[arg(long)]
    dump_tokens: bool,

    #[arg(long)]
    dump_bytecode: bool,

    #[arg(long)]
    trace: bool,

    #[arg(long)]
    no_std: bool,

    #[arg(long, default_value_t = 10_000)]
    stack_limit: usize,

    #[arg(long)]
    no_color: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    Build {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    Run {
        file: PathBuf,
    },
    BuildRun {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
}

pub fn run_cli() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let _ = (cli.trace, cli.no_std, cli.stack_limit, cli.no_color);

    match cli.command {
        Some(Command::Build { file, output }) => {
            build_file(file, output, cli.dump_tokens, cli.dump_bytecode)
        }
        Some(Command::Run { file }) => run_pbc_path(file),
        Some(Command::BuildRun { file, output }) => {
            let output = output.unwrap_or_else(|| file.with_extension("pbc"));
            build_file(
                file,
                Some(output.clone()),
                cli.dump_tokens,
                cli.dump_bytecode,
            )?;
            run_pbc_path(output)
        }
        None => {
            if let Some(file) = cli.file {
                run_source_path(file, cli.dump_tokens, cli.dump_bytecode)
            } else {
                repl::run_repl(cli.interactive)
            }
        }
    }
}

fn build_file(
    file: PathBuf,
    output: Option<PathBuf>,
    dump_tokens: bool,
    dump_bytecode: bool,
) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(&file)?;
    let tokens = tokenize(source);
    if dump_tokens {
        for token in &tokens {
            println!("{token:?}");
        }
        return Ok(());
    }

    let output = output.unwrap_or_else(|| file.with_extension("pbc"));
    if dump_bytecode {
        let pbc = compile_to_pbc(tokens).map_err(|e| format!("compile failed: {e}"))?;
        dump_pbc(&pbc);
        pbc.compile_to_file(output.to_string_lossy().as_ref())?;
    } else {
        compile(tokens, output.to_string_lossy().as_ref())?;
    }
    Ok(())
}

fn run_source_path(
    file: PathBuf,
    dump_tokens: bool,
    dump_bytecode: bool,
) -> Result<(), Box<dyn Error>> {
    let source = fs::read_to_string(file)?;
    let tokens = tokenize(source);
    if dump_tokens {
        for token in &tokens {
            println!("{token:?}");
        }
        return Ok(());
    }
    let pbc = compile_to_pbc(tokens).map_err(|e| format!("compile failed: {e}"))?;
    if dump_bytecode {
        dump_pbc(&pbc);
        return Ok(());
    }
    let pvm = vm::run_pbc_file(pbc).map_err(|e| format!("pvm failed: {e}"))?;
    if !pvm.stack.is_empty() {
        println!("stack: {:?}", pvm.stack);
    }
    Ok(())
}

fn run_pbc_path(file: PathBuf) -> Result<(), Box<dyn Error>> {
    let bytes = fs::read(file)?;
    run(bytes)
}

pub fn dump_pbc(pbc: &PbcFile) {
    for byte in pbc.to_bytes() {
        print!("{byte:02x} ");
    }
    println!();
}
