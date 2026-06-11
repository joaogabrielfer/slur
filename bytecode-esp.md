# pyx Bytecode File Layout (`.pbc`)

The `.pbc` file is divided into a static header, followed by a dynamic sequence of Tagged Sections. All multi-byte integers are stored in **Little-Endian (LE)** format.

## 1. File Header
| Name       | Length in B | Description                                                           |
| ---------- | ----------- | --------------------------------------------------------------------- |
| Magic      | 3           | 'JUZ' in ASCII to identify the format     |
| Version    | 3           | Version of the compiler/VM layout in MAJOR.MINOR.PATCH format (e.g., `0x01`, `0x04`, `0x0a` for 1.04.10)                      |

---

## 2. Dynamic Sections
After the header, the file consists of dynamic sections. The VM reads the `Section Tag`, reads the `Section Length`, and then either parses the payload or skips it if the tag is unknown.

| Name           | Length in B | Description                                                           |
| -------------- | ----------- | --------------------------------------------------------------------- |
| Section Tag    | 1           | Identifies what section comes next (e.g., `0x01` for Constants)       |
| Section Length | 4           | Total length of this section's payload in bytes (LE)                  |
| Payload        | Variable    | The actual data of the section (size matches Section Length)          |

### Section Tags:
- `0x01` - Constant Pool
- `0x02` - Bytecode (Instructions)
- `0x03` - Debug Lines (Optional)
- `0x04` - Exports (Optional, for modules)
- `0xFF` - EOF (End of File - no length or payload follows this tag)

---

## 3. Section Payloads

### 0x01 - Constant Pool Section Payload
| Name            | Length in B | Description                                                         |
| --------------- | ----------- | ------------------------------------------------------------------- |
| Item Count      | 2           | Number of constant entries in the pool (`u16`, LE)                  |
| **[Item Loop]** | -           | The following fields repeat for exactly `Item Count` iterations:    |
| Type Tag        | 1           | The type of the constant (see below)                                |
| Value Data      | Variable    | The value itself, parsed based on the Type Tag                      |

#### Constant Types:
- **`0x01` String:** `4 bytes` indicating length (`u32` LE), followed by that many UTF-8 bytes.
- **`0x02` Integer:** `8 bytes` containing the `i64` value (LE).
- **`0x03` Function:** input pattern count (`u16` LE), encoded input patterns, output pattern count (`u16` LE), encoded output patterns, `4 bytes` indicating chunk length (`u32` LE), followed by the raw bytecode belonging to this user-defined function.

Function constants preserve the declared pasm contract from `(<inputs>) -> (<outputs>) { ... }`. pvm uses the input patterns for dispatch and the output patterns to validate the return stack segment when the function frame exits.

### 0x02 - Bytecode Section Payload
| Name           | Length in B | Description                                                         |
| -------------- | ----------- | ------------------------------------------------------------------- |
| Instruction    | 1           | The OpCode byte (see Instructions table)                            |
| Operand(s)     | Variable    | Optional arguments following the OpCode (depends on instruction)    |
*(Note: The VM knows to stop reading instructions when it has consumed the `Section Length` bytes).*

---

## 4. Instructions (OpCodes)

| Code   | Name          | Arg Len | Args                   | Description                                                                 |
| ------ | ------------- | ------- | ---------------------- | --------------------------------------------------------------------------- |
| **0x00** | **Constants** |         |                        |                                                                             |
| `0x00` | PushConst     | 2       | `u16` pool index       | Pushes a value from the Constant Pool onto the stack                        |
| `0x01` | PushInt8      | 1       | `u8` value             | Pushes an immediate small integer onto the stack                            |
| `0x02` | PushTrue      | 0       | None                   | Pushes boolean `true`                                                       |
| `0x03` | PushFalse     | 0       | None                   | Pushes boolean `false`                                                      |
| `0x04` | PushType      | 1       | `u8` type ID           | Pushes a raw type literal representation                                    |
| **0x10** | **Stack** |         |                        |                                                                             |
| `0x10` | Drop          | 0       | None                   | Pops the top item and discards it                                           |
| `0x11` | Clear         | 0       | None                   | Empties the entire stack                                                    |
| `0x12` | Dup           | 0       | None                   | Duplicates the top item on the stack                                        |
| `0x13` | Swap          | 0       | None                   | Swaps the positions of the top two items                                    |
| `0x14` | Rot           | 0       | None                   | Rotates the top three items                                                 |
| `0x15` | Over          | 0       | None                   | Duplicates the second item to the top of the stack                          |
| `0x16` | Roll          | 0       | None                   | Pops `n`, rolls the `n`th item from the top to the top                      |
| `0x17` | Pick          | 0       | None                   | Pops `n`, copies the `n`th item to the top                                  |
| **0x20** | **Math & Log**|         |                        |                                                                             |
| `0x20` | Add           | 0       | None                   | Pops `b`, `a`. Pushes `a + b`                                               |
| `0x21` | Sub           | 0       | None                   | Pops `b`, `a`. Pushes `a - b`                                               |
| `0x22` | Mul           | 0       | None                   | Pops `b`, `a`. Pushes `a * b`                                               |
| `0x23` | Div           | 0       | None                   | Pops `b`, `a`. Pushes `a / b`                                               |
| `0x24` | Neg           | 0       | None                   | Negates the top integer                                                     |
| `0x25` | Eq            | 0       | None                   | Pops `b`, `a`. Pushes `true` if `a == b`                                    |
| `0x26` | Gt            | 0       | None                   | Pops `b`, `a`. Pushes `true` if `a > b`                                     |
| `0x27` | Lt            | 0       | None                   | Pops `b`, `a`. Pushes `true` if `a < b`                                     |
| `0x28` | And           | 0       | None                   | Logical AND of the top two booleans                                         |
| `0x29` | Or            | 0       | None                   | Logical OR of the top two booleans                                          |
| `0x2A` | Not           | 0       | None                   | Logical NOT of the top boolean                                              |
| **0x30** | **Casting** |         |                        |                                                                             |
| `0x30` | ToInt         | 0       | None                   | Casts top value to Int                                                      |
| `0x31` | ToString      | 0       | None                   | Casts top value to String                                                   |
| `0x32` | ToBool        | 0       | None                   | Casts top value to Bool                                                     |
| `0x33` | ToChar        | 0       | None                   | Casts top value to Char                                                     |
| `0x34` | TypeOf        | 0       | None                   | Pops value, pushes its Type literal                                         |
| **0x40** | **Data Types**|         |                        |                                                                             |
| `0x40` | Concat        | 0       | None                   | Joins two strings or arrays                                                 |
| `0x41` | Cons          | 0       | None                   | Pops `item`, `list`. Prepends `item` to `list`                              |
| `0x42` | Uncons        | 0       | None                   | Pops `list`. Pushes `Tail`, then `Head`                                     |
| `0x43` | Pack          | 0       | None                   | Pops `n`. Pops `n` items into an Array                                      |
| `0x44` | Explode       | 0       | None                   | Unpacks an array entirely onto the stack                                    |
| `0x45` | MakeRange     | 0       | None                   | Pops `b`, `a`. Pushes a `Range(a, b)` structure                             |
| **0x50** | **Execution** |         |                        |                                                                             |
| `0x50` | StoreGlobal   | 2       | `u16` const pool idx   | Pops value, stores it globally using string at const pool index as key      |
| `0x51` | LoadGlobal    | 2       | `u16` const pool idx   | Pushes the global variable associated with string at const pool index       |
| `0x52` | Eval          | 0       | None                   | Pops a Function chunk and executes it immediately                           |
| `0x53` | CallNative    | 2       | `u16` registry idx     | Executes the Rust native function mapped to the registry index              |
| **0x60** | **Control** |         |                        |                                                                             |
| `0x60` | Jump          | 2       | `i16` offset           | Moves the instruction pointer by `offset` bytes (forward or backward)       |
| `0x61` | JumpIfFalse   | 2       | `i16` offset           | Pops a bool. If false, moves the instruction pointer by `offset` bytes      |
| `0x62` | Return        | 0       | None                   | Exits current Chunk frame and returns to caller                             |
| `0x63` | Halt          | 0       | None                   | Immediately terminates the VM (Panic/Quit)                                  |
