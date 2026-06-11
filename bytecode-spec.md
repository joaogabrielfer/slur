# pyx Bytecode File Layout (`.pbc`)

This document specifies the bytecode format for `pasm` programs executed by `pvm`.
The goal is feature parity with the original tree-walker interpreter, with one source-level compatibility break: functions now declare return patterns with `(<inputs>) -> (<outputs>)`.

All multi-byte integers are little-endian.

## 1. File Header

| Name | Length | Description |
| ---- | ------ | ----------- |
| Magic | 3 bytes | ASCII `JUZ` |
| Version | 3 bytes | `MAJOR.MINOR.PATCH`, for example `0x00 0x01 0x00` for `0.1.0` |

## 2. Sections

After the header, a file contains tagged sections:

| Name | Length | Description |
| ---- | ------ | ----------- |
| Section Tag | 1 byte | Section identifier |
| Section Length | 4 bytes | Payload length as `u32` |
| Payload | variable | Section payload |

Section tags:

| Tag | Name | Required | Description |
| --- | ---- | -------- | ----------- |
| `0x01` | Constant Pool | yes | Runtime constants and function chunks |
| `0x02` | Bytecode | yes | Top-level bytecode chunk |
| `0x03` | Debug Lines | no | Source line mapping |
| `0x04` | Exports | no | Module exports |
| `0x05` | Native Names | no | Optional native registry name table |
| `0xFF` | EOF | yes | End marker; no length follows |

Unknown sections must be skipped using their section length.

## 3. Runtime Values

Runtime values in pvm mirror the original interpreter:

| Type ID | Runtime Type | Notes |
| ------- | ------------ | ----- |
| `0x00` | Unknown | Internal/invalid sentinel |
| `0x01` | Int | signed `i64` |
| `0x02` | Bool | `0x00` false, non-zero true |
| `0x03` | String | UTF-8 bytes |
| `0x04` | Char | Unicode scalar as `u32` |
| `0x05` | Block | Delayed bytecode chunk |
| `0x06` | Function | Function constant or overload set item |
| `0x07` | Any | Pattern/type wildcard |
| `0x08` | Type | Type literal |
| `0x09` | Variadic | Pattern/type wrapper |
| `0x0A` | List | Heterogeneous list |
| `0x0B` | Range | Integer range pattern/value |
| `0x0C` | FileDescriptor | pvm-managed descriptor |

## 4. Constant Pool

Payload:

| Field | Length | Description |
| ----- | ------ | ----------- |
| Item Count | 2 bytes | `u16` number of constants |
| Items | variable | Repeated constants |

Constant tags:

| Tag | Name | Payload |
| --- | ---- | ------- |
| `0x01` | String | `u32` byte length, UTF-8 bytes |
| `0x02` | Integer | `i64` |
| `0x03` | Function | function payload below |
| `0x04` | Char | `u32` Unicode scalar |
| `0x05` | Type | `u8` runtime type ID |
| `0x06` | List | `u16` item count, repeated constant indexes (`u16`) |
| `0x07` | Bool | `u8` |
| `0x08` | Block | `u32` chunk length, bytecode bytes |
| `0x09` | Pattern | encoded pattern |

Function payload:

| Field | Length | Description |
| ----- | ------ | ----------- |
| Input Pattern Count | 2 bytes | Number of input patterns |
| Input Patterns | variable | Encoded patterns |
| Output Pattern Count | 2 bytes | Number of return patterns |
| Output Patterns | variable | Encoded patterns |
| Guard Chunk Length | 4 bytes | `0` if no guard |
| Guard Chunk | variable | Bytecode that must leave `@bool` |
| Body Chunk Length | 4 bytes | Function body bytecode length |
| Body Chunk | variable | Function bytecode, usually ending in `Return` |

An overload set is represented as a list whose elements are function constants. `Call`, `Eval`, and `Match` choose the first function whose input patterns match and whose guard returns true.

## 5. Pattern Encoding

Patterns are stored in function constants and pattern constants.

| Tag | Pattern | Payload |
| --- | ------- | ------- |
| `0x01` | Type | `u8` runtime type ID |
| `0x02` | Literal | encoded runtime literal |
| `0x03` | Range | `i64` start, `i64` end, `u8` inclusive flag |
| `0x04` | List | `u16` count, repeated patterns |
| `0x05` | Destructure | head pattern, tail pattern |
| `0x06` | Fallback | none; source `..` |
| `0x07` | Variadic | inner pattern; source `..@type` |

Return pattern matching uses the same encoding and rules as input pattern matching.

## 6. Bytecode Instructions

The bytecode payload is a sequence of opcodes and operands. pvm stops at the section length for top-level chunks and at `Return` or chunk length for nested chunks.

### Constants

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x00` | PushConst | `u16 const_idx` | `-- value` |
| `0x01` | PushInt8 | `u8` | `-- @int` |
| `0x02` | PushTrue | none | `-- @bool` |
| `0x03` | PushFalse | none | `-- @bool` |
| `0x04` | PushType | `u8 type_id` | `-- @type` |
| `0x05` | PushChar | `u32 scalar` | `-- @char` |

### Stack

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x10` | Drop | none | `a --` |
| `0x11` | Clear | none | `.. --` |
| `0x12` | Dup | none | `a -- a a` |
| `0x13` | Swap | none | `a b -- b a` |
| `0x14` | Rot | none | `a b c -- b c a` |
| `0x15` | Over | none | `a b -- a b a` |
| `0x16` | Roll | none | `.. n -- ..` |
| `0x17` | Pick | none | `.. n -- .. value` |
| `0x18` | StackLen | none | `-- @int` |

### Math and Logic

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x20` | Add | none | `@int @int -- @int` |
| `0x21` | Sub | none | `@int @int -- @int` |
| `0x22` | Mul | none | `@int @int -- @int` |
| `0x23` | Div | none | `@int @int -- @int` |
| `0x24` | Neg | none | `@int -- @int` |
| `0x25` | Eq | none | `a b -- @bool` |
| `0x26` | Gt | none | `@int @int -- @bool` |
| `0x27` | Lt | none | `@int @int -- @bool` |
| `0x28` | And | none | `@bool @bool -- @bool` |
| `0x29` | Or | none | `@bool @bool -- @bool` |
| `0x2A` | Not | none | `@bool -- @bool` |

### Casting and Types

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x30` | ToInt | none | `a -- @int @bool` or `a -- @bool(false)` |
| `0x31` | ToString | none | `a -- @string @bool` |
| `0x32` | ToBool | none | `a -- @bool @bool` |
| `0x33` | ToChar | none | `a -- @char @bool` or `a -- @bool(false)` |
| `0x34` | TypeOf | none | `a -- @type` |

### Lists, Strings, and Collections

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x40` | Concat | none | `list list -- list`, `string string -- string`, `string char -- string` |
| `0x41` | Cons | none | `list item -- list`, `string char -- string` |
| `0x42` | Uncons | none | `list -- tail head`, `string -- tail head` |
| `0x43` | Pack | none | `.. n -- list` |
| `0x44` | Explode | none | `list -- ..` |
| `0x45` | MakeRange | none | `@int @int -- range` |
| `0x46` | Len | none | `list|string -- @int` |
| `0x47` | At | none | `list @int -- value`, `string @int -- @char` |
| `0x48` | First | none | `list @int -- list`, `string @int -- string` |
| `0x49` | Last | none | `list @int -- list`, `string @int -- string` |
| `0x4A` | Find | none | `string char|string -- @int @bool` or `-- @bool(false)` |
| `0x4B` | Substr | none | `string @int @int -- string` |
| `0x4C` | BeginList | none | pushes internal list sentinel |
| `0x4D` | EndList | none | consumes sentinel and packed values into list |

### Globals, Functions, Dispatch, and Modules

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x50` | StoreGlobal | `u16 name_const_idx` | `value --` |
| `0x51` | LoadGlobal | `u16 name_const_idx` | `-- value` |
| `0x52` | Eval | none | `function -- returns...` |
| `0x53` | CallNative | `u16 native_idx` | native-defined |
| `0x54` | TakeGlobal | `u16 name_const_idx` | `-- value`; removes binding |
| `0x55` | DeleteGlobal | `u16 name_const_idx` | `--` |
| `0x56` | CallGlobal | `u16 name_const_idx` | calls value bound to name |
| `0x57` | Match | none | `overload_set -- returns...` |
| `0x58` | Include | `u16 module_name_const_idx` | loads and executes module |

`LoadGlobal` pushes ordinary values. `CallGlobal`, `Eval`, and `Match` execute functions or overload sets. This preserves the original interpreter distinction between pushing a value and calling a name with `call name` or `#name`.

### Control Flow

| Code | Name | Args | Stack Effect |
| ---- | ---- | ---- | ------------ |
| `0x60` | Jump | `i16 offset` | no stack effect |
| `0x61` | JumpIfFalse | `i16 offset` | `@bool --` |
| `0x62` | Return | none | validates return patterns and exits current chunk |
| `0x63` | Halt | none | exits pvm |
| `0x64` | JumpIfTrue | `i16 offset` | `@bool --` |

`if { ... } else { ... }` compiles to `JumpIfFalse`, branch bytecode, and `Jump`. Guards compile as function guard chunks.

### File and System Operations

The original interpreter exposes `sys-open`, `sys-close`, `sys-read`, and `sys-write`. These may compile either to dedicated opcodes or to `CallNative`; the canonical native mapping is:

| Native Name | Stack Effect |
| ----------- | ------------ |
| `sys-open` | `@string -- @int` |
| `sys-close` | `@int --` |
| `sys-read` | `@int @int -- @string` |
| `sys-write` | `@int @any --` |

Descriptors `0`, `1`, and `2` are stdin, stdout, and stderr.

## 7. Source Compatibility Map

| Source Feature | Bytecode Representation |
| -------------- | ----------------------- |
| `push` literals | `PushInt8`, `PushConst`, `PushTrue`, `PushFalse`, `PushType`, `BeginList`/`EndList` |
| bare name | `LoadGlobal` |
| `call name` / `#name` | `CallGlobal` |
| `into name` | `StoreGlobal` |
| `take name` | `TakeGlobal` |
| `delete name` | `DeleteGlobal` |
| function literal | `PushConst Function` |
| overload list | list of function constants |
| `eval` | `Eval` |
| `match` | `Match` |
| `when { ... }` | function guard chunk |
| `if` / `else` | `JumpIfFalse`, `Jump` |
| list literal `[ ... ]` | `BeginList`, element bytecode, `EndList` |
| destructuring patterns | encoded `Destructure` patterns |
| variadic patterns | encoded `Variadic` patterns |
| `include std` | `Include` with module name constant |
| unsupported/high-level builtins | `CallNative` |

## 8. Backwards Compatibility Rule

Old function syntax:

```pasm
(@int @int) { add } into sum
```

is source-incompatible with pvm bytecode compilation. The pvm-compatible form is:

```pasm
(@int @int) -> (@int) { add } into sum
```

All other original interpreter language features should have a bytecode representation in this document.
