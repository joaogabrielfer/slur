mod error;
mod lexer;
mod value;
mod parser;
mod compiler;

use std::{error::Error, fs, io::{self, Write}, process::exit};

use lexer::*;
use value::*;
use parser::*;
use compiler::*;

fn main() -> Result<(), Box<dyn Error>>{
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        let mut input = String::new();
        let mut pvm = PVM::new();
        loop{
            print!("> ");
            io::stdout().flush()?;
            io::stdin().read_line(&mut input)?;

            pvm.call_stack.push(CallFrame {
                instructions: tokenize(input.clone()),
                ip: 0,
                frame_pointer: 0
            });
            if cfg!(feature = "token-logging"){
                    #[cfg(feature = "token-logging")]
                    log_tokens(pvm.call_stack.last().unwrap().instructions.clone());
                    exit(0);
            }

            match pvm.interpret(){
                Ok(Flow::Return) => return Ok(()),
                Err(e) => {
                    eprintln!("ERROR: {e}");
                    print_stack(&pvm.data_stack);
                }
                _ => {
                    print_stack(&pvm.data_stack);
                }
            }
            input.clear();
        }

    } else if args[1] == "compile" {
        let content = fs::read_to_string(args[2].clone())?;
        let tokens = tokenize(content);

        log_tokens(tokens.clone());
        compile(tokens, "foo")?;
    } else if args[1] == "run" {
        let bytes = fs::read(args[2].clone())?;

        run(bytes)?;
    } else {
        let content = fs::read_to_string(args[1].clone())?;
        let mut pvm = PVM::new();
        pvm.call_stack.push(CallFrame {
            instructions: tokenize(content.clone()),
            ip: 0,
            frame_pointer: 0
        });

        if cfg!(feature = "token-logging"){
            #[cfg(feature = "token-logging")]
            log_tokens(pvm.call_stack[0].instructions.clone());
            exit(0);
        }

        match pvm.interpret(){
            Ok(Flow::Return) => return Ok(()),
            Err(e) => {
                eprintln!("ERROR: {e}");
                exit(0);
            }
            _ => { }
        }


        if !pvm.data_stack.is_empty(){
            println!("warning: trailing number still in the stack: {:?}", pvm.data_stack);
        }
    }

    Ok(())
}

fn print_stack(stack: &[RuntimeValue]){
    print!("stack: [");
    stack
        .iter()
        .enumerate()
        .for_each(|(i, x)| {
            match x{
                RuntimeValue::String(s) => print!("\"{s}\""),
                RuntimeValue::Char(c) => print!("\'{c}\'"),
                x => print!("{x}")
            }
            if i != stack.len() - 1{
                print!(", ");
            }
        });
    println!("]");
}

// #[cfg(feature = "token-logging")]
fn log_tokens(tokens: Vec<Token>){
    tokens
        .iter()
        .for_each(|tk| println!("{:?}", tk));
}
