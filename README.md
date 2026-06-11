# pyx

pyx is the project that contains `pasm`, a small stack-oriented language, and `pvm`, the bytecode virtual machine for `.pbc` files.

The command-line binary is currently named `pasm`.

## Build

```bash
cargo build --release
```

## Run Source

Running a `.pasm` file directly compiles it in memory and executes it on pvm:

```bash
cargo run --bin pasm -- foo.pasm
```

Starting `pasm` without arguments opens the REPL:

```bash
cargo run --bin pasm
```

The REPL uses `rustyline`, stores history at `~/.pasm_history`, supports multiline `{}` and `[]` input, and includes colon commands:

```text
:help :quit :stack :clear :reset :env :type :load :reload :disasm :tokens :trace :time
```

## Build Bytecode

`pasm build` compiles `.pasm` source into a `.pbc` bytecode file:

```bash
cargo run --bin pasm -- build foo.pasm -o /tmp/foo.pbc
```

If `-o` is omitted, the output defaults to the input path with a `.pbc` extension.

## Run Bytecode

`pasm run` executes a `.pbc` file on pvm:

```bash
cargo run --bin pasm -- run /tmp/foo.pbc
```

The current pvm implementation supports literal pushes, integer math, boolean logic, stack operations, globals via `into`, `call name`, `eval`, casts, and functions with explicit return signatures. The bytecode spec now covers the original interpreter surface so the remaining VM work can be implemented without another format break.

## Source Layout

```text
src/
├── main.rs
├── cli.rs
├── repl.rs
├── error.rs
├── lexer.rs
├── value.rs
├── compiler/
│   ├── mod.rs
│   ├── opcode.rs
│   └── pbc.rs
└── vm/
    ├── mod.rs
    └── native.rs
```

## Functions

Functions declare input and output stack contracts:

```pasm
(@int @int) -> (@int) {
    add
} into sum

push 2 3
call sum
```

Multiple return values and zero-return functions are valid:

```pasm
() -> (@string @int) {
    push "ok" 200
} into status

(@int) -> () {
    drop
} into consume
```

pvm validates return arity and return types whenever a compiled function returns.

The standard library in [std/std.pasm](std/std.pasm) uses the pvm-compatible function syntax with explicit return signatures.

## Bytecode Format

The `.pbc` format is documented in [bytecode-esp.md](bytecode-esp.md). Current files use:

- magic bytes: `JUZ`
- version bytes: `0.1.0`
- tagged sections for constants and bytecode
- function constants that preserve input patterns, output patterns, and bytecode chunks

## Neovim Tree-sitter Highlighting

A Tree-sitter grammar lives in [tree-sitter-pasm](tree-sitter-pasm).

With `nvim-treesitter`, add a local parser config:

```lua
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()

parser_config.pasm = {
  install_info = {
    url = "/absolute/path/to/pyx/tree-sitter-pasm",
    files = { "src/parser.c" },
    generate_requires_npm = true,
    requires_generate_from_grammar = true,
  },
  filetype = "pasm",
}

vim.filetype.add({
  extension = {
    pasm = "pasm",
  },
})
```

Then install and enable it:

```vim
:TSInstall pasm
:set filetype=pasm
```

If you do not use `nvim-treesitter`, generate the parser manually:

```bash
cd tree-sitter-pasm
npm install
npx tree-sitter generate
```

## Tests

```bash
cargo test
```
