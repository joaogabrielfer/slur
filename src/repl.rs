use std::{error::Error, path::PathBuf, time::Instant};

use colored::Colorize;
use rustyline::DefaultEditor;

use crate::{
    cli::dump_pbc,
    compiler::{BytecodeVm, compile_to_pbc},
    lexer::tokenize,
};

pub fn run_repl(preload: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let mut editor = DefaultEditor::new()?;
    let history = history_path();
    let _ = editor.load_history(&history);
    let mut repl = ReplSession::new();

    if let Some(path) = preload {
        repl.load_file(path)?;
    }

    loop {
        let prompt = if repl.buffer.is_empty() {
            "λ> ".green().to_string()
        } else {
            ".. ".cyan().to_string()
        };
        let line = match editor.readline(&prompt) {
            Ok(line) => line,
            Err(rustyline::error::ReadlineError::Interrupted) => continue,
            Err(rustyline::error::ReadlineError::Eof) => break,
            Err(err) => return Err(err.into()),
        };

        let trimmed = line.trim();
        if repl.buffer.is_empty() && trimmed.starts_with(':') {
            if !repl.handle_command(trimmed)? {
                break;
            }
            continue;
        }

        let _ = editor.add_history_entry(line.as_str());
        repl.buffer.push_str(&line);
        repl.buffer.push('\n');
        if !balanced(&repl.buffer) {
            continue;
        }

        let source = std::mem::take(&mut repl.buffer);
        if let Err(err) = repl.eval(&source) {
            eprintln!("{} {err}", "ERROR:".red());
        }
        repl.print_stack();
    }

    let _ = editor.save_history(&history);
    Ok(())
}

fn history_path() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pasm_history")
}

struct ReplSession {
    pvm: BytecodeVm,
    buffer: String,
    last_loaded: Option<PathBuf>,
    trace: bool,
}

impl ReplSession {
    fn new() -> Self {
        Self {
            pvm: BytecodeVm::new(Vec::new()),
            buffer: String::new(),
            last_loaded: None,
            trace: false,
        }
    }

    fn handle_command(&mut self, command: &str) -> Result<bool, Box<dyn Error>> {
        let mut parts = command.split_whitespace();
        match parts.next().unwrap_or_default() {
            ":help" | ":h" => self.print_help(),
            ":quit" | ":q" => return Ok(false),
            ":stack" | ":s" => self.print_stack_verbose(),
            ":clear" | ":c" => self.pvm.clear_stack(),
            ":reset" => self.pvm.reset(),
            ":env" | ":e" => {
                for item in self.pvm.globals_summary() {
                    println!("{item}");
                }
            }
            ":type" | ":t" => {
                let name = parts.next().unwrap_or_default();
                self.print_type(name);
            }
            ":load" | ":l" => {
                let path = PathBuf::from(parts.next().ok_or(":load requires a file")?);
                self.load_file(path)?;
            }
            ":reload" | ":r" => {
                let path = self.last_loaded.clone().ok_or("no file has been loaded")?;
                self.load_file(path)?;
            }
            ":disasm" | ":d" => {
                let expr = command
                    .split_once(' ')
                    .map(|(_, expr)| expr)
                    .unwrap_or_default();
                let pbc = compile_to_pbc(tokenize(expr.to_string()))
                    .map_err(|e| format!("compile failed: {e}"))?;
                dump_pbc(&pbc);
            }
            ":tokens" => {
                let expr = command
                    .split_once(' ')
                    .map(|(_, expr)| expr)
                    .unwrap_or_default();
                for token in tokenize(expr.to_string()) {
                    println!("{token:?}");
                }
            }
            ":trace" => {
                self.trace = !self.trace;
                println!("trace: {}", if self.trace { "on" } else { "off" });
            }
            ":time" => {
                let expr = command
                    .split_once(' ')
                    .map(|(_, expr)| expr)
                    .unwrap_or_default();
                let start = Instant::now();
                self.eval(expr)?;
                println!("{:?}", start.elapsed());
            }
            other => eprintln!("unknown command: {other}"),
        }
        Ok(true)
    }

    fn eval(&mut self, source: &str) -> Result<(), String> {
        let pbc = compile_to_pbc(tokenize(source.to_string()))?;
        let (constants, bytecode) = pbc.parts()?;
        if self.trace {
            dump_pbc(&compile_to_pbc(tokenize(source.to_string()))?);
        }
        self.pvm.set_constants(constants);
        self.pvm.execute_chunk(&bytecode, 0, None)
    }

    fn load_file(&mut self, path: PathBuf) -> Result<(), Box<dyn Error>> {
        let source = std::fs::read_to_string(&path)?;
        self.eval(&source).map_err(|e| format!("pvm failed: {e}"))?;
        self.last_loaded = Some(path);
        self.print_stack();
        Ok(())
    }

    fn print_stack(&self) {
        print!("stack: [");
        for (idx, value) in self.pvm.stack.iter().enumerate() {
            if idx > 0 {
                print!(", ");
            }
            print!("{value:?}");
        }
        println!("]");
    }

    fn print_stack_verbose(&self) {
        for (idx, value) in self.pvm.stack.iter().enumerate() {
            println!("{idx}: @{} = {value:?}", value.get_type());
        }
    }

    fn print_type(&self, name: &str) {
        if name.is_empty() {
            eprintln!(":type requires a name");
            return;
        }
        if let Some(sig) = crate::value::get_builtin_signature(name) {
            println!("{sig}");
            return;
        }
        for item in self.pvm.globals_summary() {
            if item.starts_with(&format!("{name}:")) {
                println!("{item}");
                return;
            }
        }
        eprintln!("unknown name: {name}");
    }

    fn print_help(&self) {
        println!(
            ":help :quit :stack :clear :reset :env :type :load :reload :disasm :tokens :trace :time"
        );
    }
}

fn balanced(source: &str) -> bool {
    let mut curly = 0isize;
    let mut square = 0isize;
    for ch in source.chars() {
        match ch {
            '{' => curly += 1,
            '}' => curly -= 1,
            '[' => square += 1,
            ']' => square -= 1,
            _ => {}
        }
    }
    curly <= 0 && square <= 0
}
