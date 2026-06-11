# pyx / pasm Transition Plan: Tree-Walker to pvm Bytecode VM

This document describes the complete plan for transitioning the pasm interpreter in the pyx project into the pvm bytecode virtual machine, resolving error handling across the project, laying the groundwork for function typing, and organizing the codebase into a modular structure.

---

## 1. Project Architecture & Modular Structure

We will refactor the codebase to separate concerns, separating the CLI interface, REPL session, compiler front-end, VM back-end, and shared types.

```
src/
├── main.rs              — Entry Point for `pasm`: parses CLI arguments and runs CLI/REPL
├── cli.rs               — CLI Subcommand & Flag parsing (using clap)
├── repl.rs              — REPL Session loop (using rustyline), history, and colon-commands
├── error.rs             — Error enums: LexError, CompileError, RuntimeError
├── lexer.rs             — Tokenizer: tokenize()
├── value.rs             — Shared data structures: RuntimeValue, RuntimeValueT, Pattern, Element
├── compiler/
│   ├── mod.rs           — Compiler: compile(tokens) -> PbcFile
│   ├── opcode.rs        — Bytecode Instruction definitions: OpCode enum
│   └── pbc.rs           — Binary serialization format: PbcFile
└── vm/
    ├── mod.rs           — pvm: Bytecode execution loop
    └── native.rs        — Registry: Rust native functions (I/O, List operations, etc.)
```

### Dependencies
```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
rustyline = { version = "15", features = ["derive"] }
colored = "3"
```

### 1.1 Naming
- **Project name**: `pyx`.
- **Low-level stack language**: `pasm`.
- **Virtual machine**: `pvm`.
- **CLI binary**: `pasm`.
- **Source extension**: `.pasm`.
- **Bytecode extension**: `.pbc`.

### 1.2 CLI Commands & Flags
- `pasm` (no arguments) → Enter the interactive REPL.
- `pasm <file.pasm>` → Compile in-memory and execute directly.
- `pasm build <file.pasm> [-o <output>]` → Compile source file to `.pbc` bytecode file.
- `pasm run <file.pbc>` → Run a compiled bytecode file.
- `pasm build-run <file.pasm> [-o <output>]` → Compile to `.pbc` bytecode file, and run it immediately.

**Flags:**
- `-o`, `--output <path>`: Specifies output `.pbc` file path (defaults to `<input>.pbc`).
- `-i`, `--interactive <file>`: Pre-loads a `.pasm` file into the REPL session.
- `--dump-tokens`: Tokenizes source code, prints tokens to stdout, and exits.
- `--dump-bytecode`: Disassembles the bytecode, prints to stdout, and exits.
- `--trace`: Enables execution tracing (prints each opcode and stack state as it runs).
- `--no-std`: Skips loading the standard library.
- `--stack-limit <n>`: Configures maximum data stack depth (default: 10,000).
- `--no-color`: Disables colored CLI output.

---

## 2. REPL Features & Colon-Commands

The REPL will be updated to provide a premium interactive prompt:
- **Prompt**: `λ> ` (colored green/cyan).
- **Stack Summary**: Automatically print a compact stack representation after each line.
- **Persistent History**: Saves command history to `~/.pasm_history` via `rustyline`.
- **Completion**: Autocomplete element names (variables and functions in the env).
- **Multi-line input**: Detect unclosed `{ }` or `[ ]` braces to continue input.

### REPL Commands
- `:help` / `:h` — Show help.
- `:quit` / `:q` — Exit the REPL.
- `:stack` / `:s` — Show index and type of all elements currently on the stack.
- `:clear` / `:c` — Clear the stack.
- `:reset` — Reset the VM state (clear stack + all environment elements).
- `:env` / `:e` — List all defined variables/functions and their types.
- `:type <name>` / `:t <name>` — Query type or signature of a variable, function, or built-in operation.
- `:load <file>` / `:l <file>` — Load and run a `.pasm` file.
- `:reload` / `:r` — Reload the last loaded file.
- `:disasm <expr>` / `:d <expr>` — Disassemble bytecode of compiled expression.
- `:tokens <expr>` — Tokenize expression and print tokens.
- `:trace` — Toggle execution tracing.
- `:time <expr>` — Run expression and print execution duration.

---

## 3. Groundwork for Function & Primitive Typing

To support type queries like `:t add` or `:t myFunc` before full static typing is implemented, we lay down a dynamic signature model.

### 3.1 Function Signature Struct
```rust
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionSignature {
    pub inputs: Vec<RuntimeValueT>,
    pub outputs: Vec<RuntimeValueT>,
}
```
Displays like: `(@int @int) -> (@int)` or `(@any) -> (@any @any)`.

### 3.2 Function Return Signatures
User functions declare input and return stack patterns explicitly:
```pasm
(@int @int) -> (@int) {
    add
} into add_two

() -> (@string @int) {
    push "ok" 1
} into status

(@int) -> () {
    drop
} into consume_int
```

Return patterns behave like input patterns:
- `()` means no values.
- Multiple return values are allowed.
- Return values are checked when the function exits normally or via `ret`.
- The function call frame owns the argument slice. On return, pvm validates only values above the frame pointer against the declared return patterns, then leaves those values on the caller stack.
- A return arity or type mismatch is a runtime error.

### 3.3 Built-in Function Signatures
Define a helper function in `value.rs` mapping built-in primitive tokens to their stack signatures:
```rust
pub fn get_token_type(token: &Token) -> Option<FunctionSignature> {
    match token {
        Token::Add | Token::Sub | Token::Mul | Token::Div => Some(FunctionSignature {
            inputs: vec![RuntimeValueT::Int, RuntimeValueT::Int],
            outputs: vec![RuntimeValueT::Int],
        }),
        Token::Neg => Some(FunctionSignature {
            inputs: vec![RuntimeValueT::Int],
            outputs: vec![RuntimeValueT::Int],
        }),
        Token::Dup => Some(FunctionSignature {
            inputs: vec![RuntimeValueT::Any],
            outputs: vec![RuntimeValueT::Any, RuntimeValueT::Any],
        }),
        Token::Drop => Some(FunctionSignature {
            inputs: vec![RuntimeValueT::Any],
            outputs: vec![],
        }),
        Token::Swap => Some(FunctionSignature {
            inputs: vec![RuntimeValueT::Any, RuntimeValueT::Any],
            outputs: vec![RuntimeValueT::Any, RuntimeValueT::Any],
        }),
        // ... list ops, comparison, casts etc.
        _ => None,
    }
}
```

### 3.4 Custom Variable/Function Signatures
- **Patterns to Type strings**: Map `Pattern` variants to readable types.
- **Element types**: Query types for variables or functions from the environment:
```rust
impl Element {
    pub fn get_type(&self) -> String {
        match self {
            Element::Var(val) => format!("@{}", val.get_type()),
            Element::Function { inputs, outputs, .. } => {
                let inputs_str = inputs.iter()
                    .map(|p| p.to_type_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                let outputs_str = outputs.iter()
                    .map(|p| p.to_type_string())
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("({inputs_str}) -> ({outputs_str})")
            }
        }
    }
}
```

---

## 4. Error Handling Refinement

The `ret_error!` macro will be removed, and the monolithic `LangError` will be split into three distinct, structured, phase-specific error enums implementing `std::error::Error`. Functions throughout the pipeline will return a `Result<T, Box<dyn Error>>` to easily propagate errors with the `?` operator.

### 4.1 Layered Error Enums
1. **`LexError`** (Lexing phase):
   - `InvalidCharLiteral { line: usize, col: usize }`
   - `UnclosedStringLiteral { line: usize, col: usize }`
   - `UnexpectedCharacter { line: usize, col: usize, chr: char }`
2. **`CompileError`** (Compilation phase):
   - `UnexpectedToken { line: usize, col: usize, expected: Vec<Token>, got: Option<Token> }`
   - `UndeclaredObject { line: usize, col: usize, kind: String, name: String }`
   - `InvalidBytecodeSection { reason: String }`
   - `UnknownConstantTag(u8)`
3. **`RuntimeError`** (VM execution phase):
   - `StackUnderflow { ip: usize }`
   - `TypeMismatch { ip: usize, op: String, expected: Vec<RuntimeValueT>, got: Vec<RuntimeValueT> }`
   - `IndexOutOfBounds { ip: usize, index: i64, len: usize }`
   - `StackIndexOutOfBounds { ip: usize, op: String, index: usize, len: usize }`
   - `UndeclaredVariable { ip: usize, name: String }`
   - `InvalidFileDescriptor { ip: usize, fd: i64 }`
   - `DivideByZero { ip: usize }`
   - `EmptyCollection { ip: usize, op: String }`
   - `UnknownOpcode { ip: usize, op: u8 }`

---

## 5. Bytecode VM Transition Specifications

### 5.1 Opcodes
All opcodes will map exactly to the specifications defined in `bytecode-esp.md`.

### 5.2 Serialization Format (`PbcFile`)
- Fix the missing **Section Length** (4 bytes, Little-Endian) in section outputs.
- Fix **ConstantPool** item count (u16) headers.
- Fix **Constant::String** serialization (4-byte length instead of 1-byte).
- Properly append the **EOF marker** (`0xFF`).
- Encode `Constant::Function` as input pattern metadata, return pattern metadata, and bytecode chunk bytes so `.pbc` preserves declared function contracts.

### 5.3 Function & Block Compilation
- Blocks compilation generates inline chunks of bytecode stored as `Constant::Function` in the constant pool.
- Input and return patterns will be compiled into structured representations inside the constant pool, and matched dynamically at runtime by pvm.
- Function bytecode chunks end with `Return`. pvm validates the current frame's return stack segment before unwinding the frame.

### 5.4 Native Call Integration
All unsupported features in the opcode set (`at`, `first`, `last`, `len`, `stack-len`, `sys-open`, `sys-close`, `sys-read`, `sys-write`, etc.) will map to `CallNative` (opcode `0x53`) using a static lookup table registry of Rust functions.

---

## 6. Phase-by-Phase Approach

### Phase 1: Modular Setup & CLI/REPL Skeleton
1. Re-structure directories and add `cli.rs` and `repl.rs` skeletons.
2. Integrate `clap` and `rustyline` in `Cargo.toml`.
3. Fix the compilation errors in `main.rs` to allow the project to build.

### Phase 2: Error & Lexer Update
1. Implement the new `LexError`, `CompileError`, `RuntimeError` types.
2. Update the lexer to return `Result<Vec<Token>, LexError>` and track line/column positions.
3. Eliminate `ret_error!` macro usage and migrate lexer/parser error paths.

### Phase 3: Typing & Return Signature Groundwork
1. Implement `FunctionSignature` and associated traits in `value.rs`.
2. Parse `(<inputs>) -> (<outputs>) { ... }` while preserving `(<inputs>) { ... }` temporarily as a compatibility path only if needed.
3. Add built-in and user-defined signature formatting methods.
4. Validate return arity and types when functions exit.
5. Wire up `:type` / `:t` command in `repl.rs`.

### Phase 4: Serialization & Compiler
1. Implement serialization corrections for `PbcFile` structure.
2. Add bytecode encoding/decoding for function input and return pattern metadata.
3. Implement compiler from `Vec<Token>` to `PbcFile` (handling jumps, constants, and function blocks).

### Phase 5: VM & Integration
1. Implement the bytecode execution engine loop in `vm/mod.rs`.
2. Implement native functions in `vm/native.rs`.
3. Fully replace the tree-walker interpreter.
4. Update integration tests.
5. Add unit tests for return signature parsing, zero-return functions, multi-return functions, return arity/type errors, and `.pbc` function metadata round trips.
