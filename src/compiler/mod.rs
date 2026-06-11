pub mod opcode;
pub mod pbc;

use std::{collections::HashMap, error::Error, fs::File, io::Write, rc::Rc};

use crate::lexer::Token;
use crate::value::{Pattern, RuntimeValue, RuntimeValueT};
pub use opcode::OpCode;

const MAGIC: [u8; 3] = *b"JUZ";
const VERSION: [u8; 3] = [0x00, 0x01, 0x00];

#[derive(Debug)]
pub struct PbcFile {
    pub magic: [u8; 3],
    pub version: [u8; 3],
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub enum Section {
    ConstantPool(Vec<Constant>),
    Bytecode(Vec<u8>),
    Unknown { tag: u8, payload: Vec<u8> },
}

impl Section {
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            Section::ConstantPool(constants) => {
                bytes.push(0x01);
                let mut payload = Vec::new();
                payload.extend_from_slice(&(constants.len() as u16).to_le_bytes());
                for constant in constants {
                    payload.extend(constant.to_bytes());
                }
                bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                bytes.extend(payload);
            }
            Section::Bytecode(items) => {
                bytes.push(0x02);
                bytes.extend_from_slice(&(items.len() as u32).to_le_bytes());
                bytes.extend(items);
            }
            Section::Unknown { tag, payload } => {
                bytes.push(tag);
                bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
                bytes.extend(payload);
            }
        }
        bytes
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    String(String),
    Integer(i64),
    Function {
        inputs: Vec<Pattern>,
        outputs: Vec<Pattern>,
        chunk: Vec<u8>,
    },
}

impl Constant {
    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::new();
        match self {
            Constant::String(s) => {
                bytes.push(0x01);
                bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
                bytes.extend_from_slice(s.as_bytes());
            }
            Constant::Integer(i) => {
                bytes.push(0x02);
                bytes.extend_from_slice(&i.to_le_bytes());
            }
            Constant::Function {
                inputs,
                outputs,
                chunk,
            } => {
                bytes.push(0x03);
                bytes.extend_from_slice(&(inputs.len() as u16).to_le_bytes());
                for input in inputs {
                    bytes.extend(pattern_to_bytes(input));
                }
                bytes.extend_from_slice(&(outputs.len() as u16).to_le_bytes());
                for output in outputs {
                    bytes.extend(pattern_to_bytes(output));
                }
                bytes.extend_from_slice(&(chunk.len() as u32).to_le_bytes());
                bytes.extend(chunk);
            }
        }
        bytes
    }
}

impl PbcFile {
    pub fn new() -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            sections: vec![],
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 6 {
            return Err("file too short".to_string());
        }
        let magic: [u8; 3] = bytes[0..3].try_into().unwrap();
        if magic != MAGIC {
            return Err("invalid magic bytes".to_string());
        }
        let version: [u8; 3] = bytes[3..6].try_into().unwrap();
        let mut cursor = 6;
        let mut sections = Vec::new();

        while cursor < bytes.len() {
            let tag = bytes[cursor];
            cursor += 1;
            if tag == 0xFF {
                break;
            }
            if cursor + 4 > bytes.len() {
                return Err("truncated section length".to_string());
            }
            let length = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;
            if cursor + length > bytes.len() {
                return Err("truncated section payload".to_string());
            }
            let payload = &bytes[cursor..cursor + length];
            sections.push(match tag {
                0x01 => Section::ConstantPool(parse_constants(payload)?),
                0x02 => Section::Bytecode(payload.to_vec()),
                _ => Section::Unknown {
                    tag,
                    payload: payload.to_vec(),
                },
            });
            cursor += length;
        }

        Ok(Self {
            magic,
            version,
            sections,
        })
    }

    pub fn compile_to_file(&self, file_name: &str) -> Result<(), Box<dyn Error>> {
        let mut file = File::create(file_name)?;
        file.write_all(&self.to_bytes())?;
        Ok(())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.magic);
        bytes.extend_from_slice(&self.version);
        for section in &self.sections {
            bytes.extend(section.clone().to_bytes());
        }
        bytes.push(0xFF);
        bytes
    }

    pub fn parts(self) -> Result<(Vec<Constant>, Vec<u8>), String> {
        let mut constants = Vec::new();
        let mut bytecode = None;
        for section in self.sections {
            match section {
                Section::ConstantPool(pool) => constants = pool,
                Section::Bytecode(bytes) => bytecode = Some(bytes),
                Section::Unknown { .. } => {}
            }
        }
        Ok((
            constants,
            bytecode.ok_or_else(|| "missing bytecode section".to_string())?,
        ))
    }
}

pub fn compile(tokens: Vec<Token>, file_name: &str) -> Result<(), Box<dyn Error>> {
    compile_to_pbc(tokens)?.compile_to_file(file_name)
}

pub fn compile_to_pbc(tokens: Vec<Token>) -> Result<PbcFile, String> {
    let mut compiler = BytecodeCompiler::default();
    let bytecode = compiler.compile_tokens(&tokens)?;
    let mut file = PbcFile::new();
    file.sections
        .push(Section::ConstantPool(compiler.constants));
    file.sections.push(Section::Bytecode(bytecode));
    Ok(file)
}

pub fn run(bytes: Vec<u8>) -> Result<(), Box<dyn Error>> {
    let file = PbcFile::from_bytes(&bytes).map_err(|e| format!("invalid pbc file: {e}"))?;
    let (constants, bytecode) = file.parts().map_err(|e| format!("invalid pbc file: {e}"))?;
    let mut vm = BytecodeVm::new(constants);
    vm.execute_chunk(&bytecode, 0, None)?;
    if !vm.stack.is_empty() {
        println!("stack: {:?}", vm.stack);
    }
    Ok(())
}

#[derive(Default)]
struct BytecodeCompiler {
    constants: Vec<Constant>,
}

impl BytecodeCompiler {
    fn compile_tokens(&mut self, tokens: &[Token]) -> Result<Vec<u8>, String> {
        let mut bytecode = Vec::new();
        let mut cursor = 0;
        while cursor < tokens.len() {
            self.compile_one(tokens, &mut cursor, &mut bytecode)?;
        }
        bytecode.push(OpCode::Halt as u8);
        Ok(bytecode)
    }

    fn compile_block(&mut self, tokens: &[Token]) -> Result<Vec<u8>, String> {
        let mut bytecode = Vec::new();
        let mut cursor = 0;
        while cursor < tokens.len() {
            self.compile_one(tokens, &mut cursor, &mut bytecode)?;
        }
        bytecode.push(OpCode::Return as u8);
        Ok(bytecode)
    }

    fn compile_one(
        &mut self,
        tokens: &[Token],
        cursor: &mut usize,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let token = tokens
            .get(*cursor)
            .ok_or_else(|| "unexpected end of tokens".to_string())?
            .clone();
        *cursor += 1;
        match token {
            Token::Push => self.compile_push(tokens, cursor, out),
            Token::NumberLit(n) => self.emit_int(n, out),
            Token::QuotedLit(s) => self.emit_const(Constant::String(s), out),
            Token::BoolLit(true) => {
                out.push(OpCode::PushTrue as u8);
                Ok(())
            }
            Token::BoolLit(false) => {
                out.push(OpCode::PushFalse as u8);
                Ok(())
            }
            Token::TypeLit(t) => {
                out.push(OpCode::PushType as u8);
                out.push(type_to_byte(t));
                Ok(())
            }
            Token::Add => emit_op(out, OpCode::Add),
            Token::Sub => emit_op(out, OpCode::Sub),
            Token::Mul => emit_op(out, OpCode::Mul),
            Token::Div => emit_op(out, OpCode::Div),
            Token::Neg => emit_op(out, OpCode::Neg),
            Token::Eq => emit_op(out, OpCode::Eq),
            Token::Gt => emit_op(out, OpCode::Gt),
            Token::Lt => emit_op(out, OpCode::Lt),
            Token::And => emit_op(out, OpCode::And),
            Token::Or => emit_op(out, OpCode::Or),
            Token::Not => emit_op(out, OpCode::Not),
            Token::Drop => emit_op(out, OpCode::Drop),
            Token::Clear => emit_op(out, OpCode::Clear),
            Token::Dup => emit_op(out, OpCode::Dup),
            Token::Swap => emit_op(out, OpCode::Swap),
            Token::Rot => emit_op(out, OpCode::Rot),
            Token::Over => emit_op(out, OpCode::Over),
            Token::Roll => emit_op(out, OpCode::Roll),
            Token::Pick => emit_op(out, OpCode::Pick),
            Token::ToInt => emit_op(out, OpCode::ToInt),
            Token::ToString => emit_op(out, OpCode::ToString),
            Token::ToBool => emit_op(out, OpCode::ToBool),
            Token::ToChar => emit_op(out, OpCode::ToChar),
            Token::TypeOf => emit_op(out, OpCode::TypeOf),
            Token::Eval => emit_op(out, OpCode::Eval),
            Token::Ret => emit_op(out, OpCode::Return),
            Token::Into => {
                let name = match tokens.get(*cursor) {
                    Some(Token::UnquotedLit(name)) => name.clone(),
                    other => return Err(format!("expected name after into, got {other:?}")),
                };
                *cursor += 1;
                let index = self.add_constant(Constant::String(name));
                out.push(OpCode::StoreGlobal as u8);
                out.extend_from_slice(&index.to_le_bytes());
                Ok(())
            }
            Token::ElementCall(name) => {
                let index = self.add_constant(Constant::String(name));
                out.push(OpCode::LoadGlobal as u8);
                out.extend_from_slice(&index.to_le_bytes());
                out.push(OpCode::Eval as u8);
                Ok(())
            }
            Token::UnquotedLit(name) => {
                let index = self.add_constant(Constant::String(name));
                out.push(OpCode::LoadGlobal as u8);
                out.extend_from_slice(&index.to_le_bytes());
                Ok(())
            }
            Token::OpenParen => {
                let inputs = parse_patterns(tokens, cursor)?;
                let outputs = if tokens.get(*cursor) == Some(&Token::Arrow) {
                    *cursor += 1;
                    expect_token(tokens, cursor, Token::OpenParen)?;
                    parse_patterns(tokens, cursor)?
                } else {
                    Vec::new()
                };
                if tokens.get(*cursor) == Some(&Token::When) {
                    return Err("guards are not compiled to pvm bytecode yet".to_string());
                }
                expect_token(tokens, cursor, Token::OpenCurly)?;
                let block_tokens = collect_block(tokens, cursor)?;
                let chunk = self.compile_block(&block_tokens)?;
                self.emit_const(
                    Constant::Function {
                        inputs,
                        outputs,
                        chunk,
                    },
                    out,
                )
            }
            Token::Quit => emit_op(out, OpCode::Halt),
            other => Err(format!(
                "token {other:?} is not supported by the pvm compiler yet"
            )),
        }
    }

    fn compile_push(
        &mut self,
        tokens: &[Token],
        cursor: &mut usize,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let mut emitted = false;
        while let Some(token) = tokens.get(*cursor).cloned() {
            match token {
                Token::NumberLit(n) => self.emit_int(n, out)?,
                Token::QuotedLit(s) => self.emit_const(Constant::String(s), out)?,
                Token::BoolLit(true) => out.push(OpCode::PushTrue as u8),
                Token::BoolLit(false) => out.push(OpCode::PushFalse as u8),
                Token::TypeLit(t) => {
                    out.push(OpCode::PushType as u8);
                    out.push(type_to_byte(t));
                }
                _ => break,
            }
            emitted = true;
            *cursor += 1;
        }
        if emitted {
            Ok(())
        } else {
            Err("push requires at least one literal".to_string())
        }
    }

    fn emit_int(&mut self, n: i64, out: &mut Vec<u8>) -> Result<(), String> {
        if (0..=u8::MAX as i64).contains(&n) {
            out.push(OpCode::PushInt8 as u8);
            out.push(n as u8);
            Ok(())
        } else {
            self.emit_const(Constant::Integer(n), out)
        }
    }

    fn emit_const(&mut self, constant: Constant, out: &mut Vec<u8>) -> Result<(), String> {
        let index = self.add_constant(constant);
        out.push(OpCode::PushConst as u8);
        out.extend_from_slice(&index.to_le_bytes());
        Ok(())
    }

    fn add_constant(&mut self, constant: Constant) -> u16 {
        self.constants.push(constant);
        (self.constants.len() - 1) as u16
    }
}

fn emit_op(out: &mut Vec<u8>, op: OpCode) -> Result<(), String> {
    out.push(op as u8);
    Ok(())
}

pub struct BytecodeVm {
    constants: Vec<Constant>,
    globals: HashMap<String, RuntimeValue>,
    pub stack: Vec<RuntimeValue>,
}

impl BytecodeVm {
    pub fn new(constants: Vec<Constant>) -> Self {
        Self {
            constants,
            globals: HashMap::new(),
            stack: Vec::new(),
        }
    }

    pub fn set_constants(&mut self, constants: Vec<Constant>) {
        self.constants = constants;
    }

    pub fn clear_stack(&mut self) {
        self.stack.clear();
    }

    pub fn reset(&mut self) {
        self.stack.clear();
        self.globals.clear();
    }

    pub fn globals_summary(&self) -> Vec<String> {
        let mut items: Vec<_> = self
            .globals
            .iter()
            .map(|(name, value)| format!("{name}: @{}", value.get_type()))
            .collect();
        items.sort();
        items
    }

    pub fn execute_chunk(
        &mut self,
        bytecode: &[u8],
        frame_pointer: usize,
        returns: Option<&[Pattern]>,
    ) -> Result<(), String> {
        let mut ip = 0;
        while ip < bytecode.len() {
            let op = OpCode::from_byte(bytecode[ip])?;
            ip += 1;
            match op {
                OpCode::PushConst => {
                    let index = read_u16(bytecode, &mut ip)? as usize;
                    let value = self.constant_to_value(index)?;
                    self.stack.push(value);
                }
                OpCode::PushInt8 => self
                    .stack
                    .push(RuntimeValue::Int(read_u8(bytecode, &mut ip)? as i64)),
                OpCode::PushTrue => self.stack.push(RuntimeValue::Bool(true)),
                OpCode::PushFalse => self.stack.push(RuntimeValue::Bool(false)),
                OpCode::PushType => self.stack.push(RuntimeValue::Type(byte_to_type(read_u8(
                    bytecode, &mut ip,
                )?))),
                OpCode::Drop => {
                    self.stack
                        .pop()
                        .ok_or_else(|| "stack underflow on drop".to_string())?;
                }
                OpCode::Clear => self.stack.clear(),
                OpCode::Dup => self.stack.push(
                    self.stack
                        .last()
                        .ok_or_else(|| "stack underflow on dup".to_string())?
                        .clone(),
                ),
                OpCode::Swap => {
                    let len = self.stack.len();
                    if len < 2 {
                        return Err("stack underflow on swap".to_string());
                    }
                    self.stack.swap(len - 1, len - 2);
                }
                OpCode::Rot => {
                    let len = self.stack.len();
                    if len < 3 {
                        return Err("stack underflow on rot".to_string());
                    }
                    let value = self.stack.remove(len - 3);
                    self.stack.push(value);
                }
                OpCode::Over => {
                    let len = self.stack.len();
                    if len < 2 {
                        return Err("stack underflow on over".to_string());
                    }
                    self.stack.push(self.stack[len - 2].clone());
                }
                OpCode::Roll => {
                    let index = self.pop_int()? as usize;
                    if index == 0 || index > self.stack.len() {
                        return Err("roll index out of bounds".to_string());
                    }
                    let value = self.stack.remove(self.stack.len() - index);
                    self.stack.push(value);
                }
                OpCode::Pick => {
                    let index = self.pop_int()? as usize;
                    if index == 0 || index > self.stack.len() {
                        return Err("pick index out of bounds".to_string());
                    }
                    self.stack
                        .push(self.stack[self.stack.len() - index].clone());
                }
                OpCode::Add => self.binary_int(|a, b| a + b)?,
                OpCode::Sub => self.binary_int(|a, b| a - b)?,
                OpCode::Mul => self.binary_int(|a, b| a * b)?,
                OpCode::Div => {
                    let b = self.pop_int()?;
                    if b == 0 {
                        return Err("division by zero".to_string());
                    }
                    let a = self.pop_int()?;
                    self.stack.push(RuntimeValue::Int(a / b));
                }
                OpCode::Neg => {
                    let value = self.pop_int()?;
                    self.stack.push(RuntimeValue::Int(-value));
                }
                OpCode::Eq => {
                    let b = self
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on eq".to_string())?;
                    let a = self
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on eq".to_string())?;
                    self.stack.push(RuntimeValue::Bool(a == b));
                }
                OpCode::Gt => self.compare_int(|a, b| a > b)?,
                OpCode::Lt => self.compare_int(|a, b| a < b)?,
                OpCode::And => self.binary_bool(|a, b| a && b)?,
                OpCode::Or => self.binary_bool(|a, b| a || b)?,
                OpCode::Not => {
                    let value = self.pop_bool()?;
                    self.stack.push(RuntimeValue::Bool(!value));
                }
                OpCode::ToInt => self.cast_int()?,
                OpCode::ToString => self.cast_string()?,
                OpCode::ToBool => self.cast_bool()?,
                OpCode::ToChar => self.cast_char()?,
                OpCode::TypeOf => {
                    let value = self
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on typeof".to_string())?;
                    self.stack.push(RuntimeValue::Type(value.get_type()));
                }
                OpCode::StoreGlobal => {
                    let key = self.constant_string(read_u16(bytecode, &mut ip)? as usize)?;
                    let value = self
                        .stack
                        .pop()
                        .ok_or_else(|| "stack underflow on store".to_string())?;
                    self.globals.insert(key, value);
                }
                OpCode::LoadGlobal => {
                    let key = self.constant_string(read_u16(bytecode, &mut ip)? as usize)?;
                    let value = self
                        .globals
                        .get(&key)
                        .ok_or_else(|| format!("undeclared global {key}"))?
                        .clone();
                    self.stack.push(value);
                }
                OpCode::Eval => self.eval_top()?,
                OpCode::Return => {
                    if let Some(returns) = returns {
                        self.validate_returns(frame_pointer, returns)?;
                    }
                    return Ok(());
                }
                OpCode::Halt => break,
            }
        }
        if let Some(returns) = returns {
            self.validate_returns(frame_pointer, returns)?;
        }
        Ok(())
    }

    fn eval_top(&mut self) -> Result<(), String> {
        match self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow on eval".to_string())?
        {
            RuntimeValue::CompiledFunction {
                patterns,
                outputs,
                chunk,
            } => {
                let fp = self.match_inputs(&patterns)?;
                self.execute_chunk(&chunk, fp, Some(&outputs))
            }
            other => Err(format!("eval expected function, got {}", other.type_name())),
        }
    }

    fn match_inputs(&self, patterns: &[Pattern]) -> Result<usize, String> {
        if self.stack.len() < patterns.len() {
            return Err(format!(
                "function expected {} input(s), got {}",
                patterns.len(),
                self.stack.len()
            ));
        }
        let fp = self.stack.len() - patterns.len();
        for (index, pattern) in patterns.iter().enumerate() {
            if !pattern.check(&self.stack[fp + index]) {
                return Err(format!("function input {index} did not match {pattern:?}"));
            }
        }
        Ok(fp)
    }

    fn validate_returns(&self, frame_pointer: usize, returns: &[Pattern]) -> Result<(), String> {
        let got = self.stack.len().saturating_sub(frame_pointer);
        if got != returns.len() {
            return Err(format!(
                "return arity mismatch: expected {}, got {got}",
                returns.len()
            ));
        }
        for (index, pattern) in returns.iter().enumerate() {
            if !pattern.check(&self.stack[frame_pointer + index]) {
                return Err(format!("return {index} did not match {pattern:?}"));
            }
        }
        Ok(())
    }

    fn constant_to_value(&self, index: usize) -> Result<RuntimeValue, String> {
        match self
            .constants
            .get(index)
            .ok_or_else(|| format!("constant index {index} out of bounds"))?
        {
            Constant::String(s) => Ok(RuntimeValue::String(Rc::new(s.clone()))),
            Constant::Integer(i) => Ok(RuntimeValue::Int(*i)),
            Constant::Function {
                inputs,
                outputs,
                chunk,
            } => Ok(RuntimeValue::CompiledFunction {
                patterns: inputs.clone(),
                outputs: outputs.clone(),
                chunk: chunk.clone(),
            }),
        }
    }

    fn constant_string(&self, index: usize) -> Result<String, String> {
        match self
            .constants
            .get(index)
            .ok_or_else(|| format!("constant index {index} out of bounds"))?
        {
            Constant::String(s) => Ok(s.clone()),
            other => Err(format!("expected string constant, got {other:?}")),
        }
    }

    fn pop_int(&mut self) -> Result<i64, String> {
        match self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow while reading int".to_string())?
        {
            RuntimeValue::Int(i) => Ok(i),
            other => Err(format!("expected int, got {}", other.type_name())),
        }
    }

    fn pop_bool(&mut self) -> Result<bool, String> {
        match self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow while reading bool".to_string())?
        {
            RuntimeValue::Bool(b) => Ok(b),
            other => Err(format!("expected bool, got {}", other.type_name())),
        }
    }

    fn binary_int(&mut self, op: impl FnOnce(i64, i64) -> i64) -> Result<(), String> {
        let b = self.pop_int()?;
        let a = self.pop_int()?;
        self.stack.push(RuntimeValue::Int(op(a, b)));
        Ok(())
    }

    fn compare_int(&mut self, op: impl FnOnce(i64, i64) -> bool) -> Result<(), String> {
        let b = self.pop_int()?;
        let a = self.pop_int()?;
        self.stack.push(RuntimeValue::Bool(op(a, b)));
        Ok(())
    }

    fn binary_bool(&mut self, op: impl FnOnce(bool, bool) -> bool) -> Result<(), String> {
        let b = self.pop_bool()?;
        let a = self.pop_bool()?;
        self.stack.push(RuntimeValue::Bool(op(a, b)));
        Ok(())
    }

    fn cast_int(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow on int?".to_string())?;
        match value {
            RuntimeValue::Int(i) => {
                self.stack.push(RuntimeValue::Int(i));
                self.stack.push(RuntimeValue::Bool(true));
            }
            RuntimeValue::Bool(b) => {
                self.stack.push(RuntimeValue::Int(if b { 1 } else { 0 }));
                self.stack.push(RuntimeValue::Bool(true));
            }
            RuntimeValue::String(s) => match s.parse::<i64>() {
                Ok(i) => {
                    self.stack.push(RuntimeValue::Int(i));
                    self.stack.push(RuntimeValue::Bool(true));
                }
                Err(_) => self.stack.push(RuntimeValue::Bool(false)),
            },
            _ => self.stack.push(RuntimeValue::Bool(false)),
        }
        Ok(())
    }

    fn cast_string(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow on string?".to_string())?;
        self.stack
            .push(RuntimeValue::String(Rc::new(value.to_string())));
        self.stack.push(RuntimeValue::Bool(true));
        Ok(())
    }

    fn cast_bool(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow on bool?".to_string())?;
        let result = match value {
            RuntimeValue::Bool(b) => b,
            RuntimeValue::Int(i) => i != 0,
            RuntimeValue::String(s) => !s.is_empty(),
            RuntimeValue::List(l) => !l.is_empty(),
            _ => false,
        };
        self.stack.push(RuntimeValue::Bool(result));
        self.stack.push(RuntimeValue::Bool(true));
        Ok(())
    }

    fn cast_char(&mut self) -> Result<(), String> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| "stack underflow on char?".to_string())?;
        match value {
            RuntimeValue::Char(c) => {
                self.stack.push(RuntimeValue::Char(c));
                self.stack.push(RuntimeValue::Bool(true));
            }
            RuntimeValue::String(s) if s.chars().count() == 1 => {
                self.stack
                    .push(RuntimeValue::Char(s.chars().next().unwrap()));
                self.stack.push(RuntimeValue::Bool(true));
            }
            _ => self.stack.push(RuntimeValue::Bool(false)),
        }
        Ok(())
    }
}

fn read_u8(bytecode: &[u8], ip: &mut usize) -> Result<u8, String> {
    let byte = *bytecode
        .get(*ip)
        .ok_or_else(|| "truncated u8 operand".to_string())?;
    *ip += 1;
    Ok(byte)
}

fn read_u16(bytecode: &[u8], ip: &mut usize) -> Result<u16, String> {
    if *ip + 2 > bytecode.len() {
        return Err("truncated u16 operand".to_string());
    }
    let value = u16::from_le_bytes(bytecode[*ip..*ip + 2].try_into().unwrap());
    *ip += 2;
    Ok(value)
}

fn expect_token(tokens: &[Token], cursor: &mut usize, expected: Token) -> Result<(), String> {
    match tokens.get(*cursor) {
        Some(actual) if *actual == expected => {
            *cursor += 1;
            Ok(())
        }
        other => Err(format!("expected {expected:?}, got {other:?}")),
    }
}

fn collect_block(tokens: &[Token], cursor: &mut usize) -> Result<Vec<Token>, String> {
    let mut depth = 1;
    let mut block = Vec::new();
    while *cursor < tokens.len() {
        let token = tokens[*cursor].clone();
        *cursor += 1;
        match token {
            Token::OpenCurly => {
                depth += 1;
                block.push(token);
            }
            Token::CloseCurly => {
                depth -= 1;
                if depth == 0 {
                    return Ok(block);
                }
                block.push(token);
            }
            _ => block.push(token),
        }
    }
    Err("unclosed function block".to_string())
}

fn parse_patterns(tokens: &[Token], cursor: &mut usize) -> Result<Vec<Pattern>, String> {
    let mut patterns = Vec::new();
    while *cursor < tokens.len() {
        let token = tokens[*cursor].clone();
        *cursor += 1;
        if token == Token::CloseParen {
            return Ok(patterns);
        }
        patterns.push(parse_single_pattern(tokens, cursor, token)?);
    }
    Err("unclosed pattern list".to_string())
}

fn parse_single_pattern(
    tokens: &[Token],
    cursor: &mut usize,
    token: Token,
) -> Result<Pattern, String> {
    match token {
        Token::TypeLit(RuntimeValueT::Variadic(t)) => {
            Ok(Pattern::Variadic(Box::new(Pattern::Type(*t))))
        }
        Token::TypeLit(t) => Ok(Pattern::Type(t)),
        Token::NumberLit(n) => {
            if tokens.get(*cursor) == Some(&Token::RangeOp) {
                *cursor += 1;
                match tokens.get(*cursor) {
                    Some(Token::NumberLit(end)) => {
                        *cursor += 1;
                        Ok(Pattern::Range {
                            start: n,
                            end: *end,
                            inclusive: false,
                        })
                    }
                    other => Err(format!("expected range end, got {other:?}")),
                }
            } else {
                Ok(Pattern::Literal(RuntimeValue::Int(n)))
            }
        }
        Token::QuotedLit(s) => Ok(Pattern::Literal(RuntimeValue::String(Rc::new(s)))),
        Token::BoolLit(b) => Ok(Pattern::Literal(RuntimeValue::Bool(b))),
        Token::Fallback => Ok(Pattern::Fallback),
        other => Err(format!("unsupported bytecode pattern token {other:?}")),
    }
}

fn parse_constants(payload: &[u8]) -> Result<Vec<Constant>, String> {
    let mut constants = Vec::new();
    let mut cursor = 0;
    if payload.len() < 2 {
        return Err("truncated constant pool".to_string());
    }
    let count = u16::from_le_bytes(payload[cursor..cursor + 2].try_into().unwrap());
    cursor += 2;
    for _ in 0..count {
        let tag = read_payload_u8(payload, &mut cursor)?;
        constants.push(match tag {
            0x01 => {
                let len = read_payload_u32(payload, &mut cursor)? as usize;
                if cursor + len > payload.len() {
                    return Err("truncated string constant".to_string());
                }
                let value = str::from_utf8(&payload[cursor..cursor + len])
                    .map_err(|e| e.to_string())?
                    .to_string();
                cursor += len;
                Constant::String(value)
            }
            0x02 => {
                if cursor + 8 > payload.len() {
                    return Err("truncated integer constant".to_string());
                }
                let value = i64::from_le_bytes(payload[cursor..cursor + 8].try_into().unwrap());
                cursor += 8;
                Constant::Integer(value)
            }
            0x03 => {
                let input_count = read_payload_u16(payload, &mut cursor)?;
                let mut inputs = Vec::new();
                for _ in 0..input_count {
                    inputs.push(parse_pattern(payload, &mut cursor)?);
                }
                let output_count = read_payload_u16(payload, &mut cursor)?;
                let mut outputs = Vec::new();
                for _ in 0..output_count {
                    outputs.push(parse_pattern(payload, &mut cursor)?);
                }
                let chunk_len = read_payload_u32(payload, &mut cursor)? as usize;
                if cursor + chunk_len > payload.len() {
                    return Err("truncated function chunk".to_string());
                }
                let chunk = payload[cursor..cursor + chunk_len].to_vec();
                cursor += chunk_len;
                Constant::Function {
                    inputs,
                    outputs,
                    chunk,
                }
            }
            _ => return Err(format!("unknown constant tag 0x{tag:02x}")),
        });
    }
    Ok(constants)
}

fn pattern_to_bytes(pattern: Pattern) -> Vec<u8> {
    let mut bytes = Vec::new();
    match pattern {
        Pattern::Type(t) => {
            bytes.push(0x01);
            bytes.push(type_to_byte(t));
        }
        Pattern::Literal(value) => {
            bytes.push(0x02);
            bytes.extend(runtime_value_to_bytes(value));
        }
        Pattern::Range {
            start,
            end,
            inclusive,
        } => {
            bytes.push(0x03);
            bytes.extend_from_slice(&start.to_le_bytes());
            bytes.extend_from_slice(&end.to_le_bytes());
            bytes.push(u8::from(inclusive));
        }
        Pattern::List(items) => {
            bytes.push(0x04);
            bytes.extend_from_slice(&(items.len() as u16).to_le_bytes());
            for item in items {
                bytes.extend(pattern_to_bytes(item));
            }
        }
        Pattern::Destructure(head, tail) => {
            bytes.push(0x05);
            bytes.extend(pattern_to_bytes(*head));
            bytes.extend(pattern_to_bytes(*tail));
        }
        Pattern::Fallback => bytes.push(0x06),
        Pattern::Variadic(inner) => {
            bytes.push(0x07);
            bytes.extend(pattern_to_bytes(*inner));
        }
    }
    bytes
}

fn parse_pattern(payload: &[u8], cursor: &mut usize) -> Result<Pattern, String> {
    Ok(match read_payload_u8(payload, cursor)? {
        0x01 => Pattern::Type(byte_to_type(read_payload_u8(payload, cursor)?)),
        0x02 => Pattern::Literal(parse_runtime_value(payload, cursor)?),
        0x03 => {
            let start = read_payload_i64(payload, cursor)?;
            let end = read_payload_i64(payload, cursor)?;
            let inclusive = read_payload_u8(payload, cursor)? != 0;
            Pattern::Range {
                start,
                end,
                inclusive,
            }
        }
        0x04 => {
            let count = read_payload_u16(payload, cursor)?;
            let mut items = Vec::new();
            for _ in 0..count {
                items.push(parse_pattern(payload, cursor)?);
            }
            Pattern::List(items)
        }
        0x05 => {
            let head = parse_pattern(payload, cursor)?;
            let tail = parse_pattern(payload, cursor)?;
            Pattern::Destructure(Box::new(head), Box::new(tail))
        }
        0x06 => Pattern::Fallback,
        0x07 => Pattern::Variadic(Box::new(parse_pattern(payload, cursor)?)),
        other => return Err(format!("unknown pattern tag 0x{other:02x}")),
    })
}

fn runtime_value_to_bytes(value: RuntimeValue) -> Vec<u8> {
    let mut bytes = Vec::new();
    match value {
        RuntimeValue::Int(i) => {
            bytes.push(0x01);
            bytes.extend_from_slice(&i.to_le_bytes());
        }
        RuntimeValue::Bool(b) => {
            bytes.push(0x02);
            bytes.push(u8::from(b));
        }
        RuntimeValue::String(s) => {
            bytes.push(0x03);
            bytes.extend_from_slice(&(s.len() as u32).to_le_bytes());
            bytes.extend_from_slice(s.as_bytes());
        }
        RuntimeValue::Char(c) => {
            bytes.push(0x04);
            bytes.extend_from_slice(&(c as u32).to_le_bytes());
        }
        RuntimeValue::Type(t) => {
            bytes.push(0x05);
            bytes.push(type_to_byte(t));
        }
        _ => bytes.push(0x00),
    }
    bytes
}

fn parse_runtime_value(payload: &[u8], cursor: &mut usize) -> Result<RuntimeValue, String> {
    Ok(match read_payload_u8(payload, cursor)? {
        0x01 => RuntimeValue::Int(read_payload_i64(payload, cursor)?),
        0x02 => RuntimeValue::Bool(read_payload_u8(payload, cursor)? != 0),
        0x03 => {
            let len = read_payload_u32(payload, cursor)? as usize;
            if *cursor + len > payload.len() {
                return Err("truncated string literal pattern".to_string());
            }
            let value = str::from_utf8(&payload[*cursor..*cursor + len])
                .map_err(|e| e.to_string())?
                .to_string();
            *cursor += len;
            RuntimeValue::String(Rc::new(value))
        }
        0x04 => {
            RuntimeValue::Char(char::from_u32(read_payload_u32(payload, cursor)?).unwrap_or('\0'))
        }
        0x05 => RuntimeValue::Type(byte_to_type(read_payload_u8(payload, cursor)?)),
        _ => RuntimeValue::Type(RuntimeValueT::Unknown),
    })
}

fn type_to_byte(t: RuntimeValueT) -> u8 {
    match t {
        RuntimeValueT::Int => 0x01,
        RuntimeValueT::Bool => 0x02,
        RuntimeValueT::String => 0x03,
        RuntimeValueT::Char => 0x04,
        RuntimeValueT::Block => 0x05,
        RuntimeValueT::Function => 0x06,
        RuntimeValueT::Any => 0x07,
        RuntimeValueT::Type => 0x08,
        RuntimeValueT::Variadic(_) => 0x09,
        RuntimeValueT::List(_) => 0x0A,
        RuntimeValueT::Unknown => 0x00,
    }
}

fn byte_to_type(byte: u8) -> RuntimeValueT {
    match byte {
        0x01 => RuntimeValueT::Int,
        0x02 => RuntimeValueT::Bool,
        0x03 => RuntimeValueT::String,
        0x04 => RuntimeValueT::Char,
        0x05 => RuntimeValueT::Block,
        0x06 => RuntimeValueT::Function,
        0x07 => RuntimeValueT::Any,
        0x08 => RuntimeValueT::Type,
        0x09 => RuntimeValueT::Variadic(Box::new(RuntimeValueT::Any)),
        0x0A => RuntimeValueT::List(vec![]),
        _ => RuntimeValueT::Unknown,
    }
}

fn read_payload_u8(payload: &[u8], cursor: &mut usize) -> Result<u8, String> {
    let value = *payload
        .get(*cursor)
        .ok_or_else(|| "truncated payload".to_string())?;
    *cursor += 1;
    Ok(value)
}

fn read_payload_u16(payload: &[u8], cursor: &mut usize) -> Result<u16, String> {
    if *cursor + 2 > payload.len() {
        return Err("truncated u16 payload".to_string());
    }
    let value = u16::from_le_bytes(payload[*cursor..*cursor + 2].try_into().unwrap());
    *cursor += 2;
    Ok(value)
}

fn read_payload_u32(payload: &[u8], cursor: &mut usize) -> Result<u32, String> {
    if *cursor + 4 > payload.len() {
        return Err("truncated u32 payload".to_string());
    }
    let value = u32::from_le_bytes(payload[*cursor..*cursor + 4].try_into().unwrap());
    *cursor += 4;
    Ok(value)
}

fn read_payload_i64(payload: &[u8], cursor: &mut usize) -> Result<i64, String> {
    if *cursor + 8 > payload.len() {
        return Err("truncated i64 payload".to_string());
    }
    let value = i64::from_le_bytes(payload[*cursor..*cursor + 8].try_into().unwrap());
    *cursor += 8;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn function_constant_round_trips_input_and_output_patterns() {
        let mut file = PbcFile::new();
        file.sections
            .push(Section::ConstantPool(vec![Constant::Function {
                inputs: vec![
                    Pattern::Type(RuntimeValueT::Int),
                    Pattern::Type(RuntimeValueT::Int),
                ],
                outputs: vec![Pattern::Type(RuntimeValueT::Int)],
                chunk: vec![OpCode::Add as u8],
            }]));
        file.sections
            .push(Section::Bytecode(vec![OpCode::Halt as u8]));

        let parsed = PbcFile::from_bytes(&file.to_bytes()).expect("pbc should parse");
        match &parsed.sections[0] {
            Section::ConstantPool(constants) => match &constants[0] {
                Constant::Function {
                    inputs,
                    outputs,
                    chunk,
                } => {
                    assert_eq!(
                        inputs,
                        &vec![
                            Pattern::Type(RuntimeValueT::Int),
                            Pattern::Type(RuntimeValueT::Int)
                        ]
                    );
                    assert_eq!(outputs, &vec![Pattern::Type(RuntimeValueT::Int)]);
                    assert_eq!(chunk, &vec![OpCode::Add as u8]);
                }
                other => panic!("expected function constant, got {other:?}"),
            },
            other => panic!("expected constant pool, got {other:?}"),
        }
    }

    #[test]
    fn compiles_pasm_to_juz_header() {
        let pbc = compile_to_pbc(tokenize("push 1 2 add".to_string())).expect("compile");
        let bytes = pbc.to_bytes();
        assert_eq!(&bytes[0..3], b"JUZ");
        assert_eq!(&bytes[3..6], &[0, 1, 0]);
    }

    #[test]
    fn pvm_validates_function_returns() {
        let pbc = compile_to_pbc(tokenize("push 1 (@int) -> (@string) { } eval".to_string()))
            .expect("compile");
        let (constants, bytecode) = pbc.parts().expect("parts");
        let mut vm = BytecodeVm::new(constants);
        let err = vm
            .execute_chunk(&bytecode, 0, None)
            .expect_err("return type should fail");
        assert!(err.contains("return 0 did not match"));
    }

    #[test]
    fn pvm_executes_named_function_call() {
        let pbc = compile_to_pbc(tokenize(
            "(@int @int) -> (@int) { add } into sum push 2 3 call sum".to_string(),
        ))
        .expect("compile");
        let (constants, bytecode) = pbc.parts().expect("parts");
        let mut vm = BytecodeVm::new(constants);
        vm.execute_chunk(&bytecode, 0, None).expect("run");
        assert_eq!(vm.stack, vec![RuntimeValue::Int(5)]);
    }

    #[test]
    fn pvm_accepts_zero_return_function() {
        let pbc = compile_to_pbc(tokenize("push 1 (@int) -> () { drop } eval".to_string()))
            .expect("compile");
        let (constants, bytecode) = pbc.parts().expect("parts");
        let mut vm = BytecodeVm::new(constants);
        vm.execute_chunk(&bytecode, 0, None).expect("run");
        assert!(vm.stack.is_empty());
    }

    #[test]
    fn pvm_accepts_multi_return_function() {
        let pbc = compile_to_pbc(tokenize(
            "() -> (@string @int) { push \"ok\" 200 } eval".to_string(),
        ))
        .expect("compile");
        let (constants, bytecode) = pbc.parts().expect("parts");
        let mut vm = BytecodeVm::new(constants);
        vm.execute_chunk(&bytecode, 0, None).expect("run");
        assert_eq!(
            vm.stack,
            vec![
                RuntimeValue::String(std::rc::Rc::new("ok".to_string())),
                RuntimeValue::Int(200)
            ]
        );
    }

    #[test]
    fn pvm_rejects_return_arity_mismatch() {
        let pbc = compile_to_pbc(tokenize("() -> (@int @int) { push 1 } eval".to_string()))
            .expect("compile");
        let (constants, bytecode) = pbc.parts().expect("parts");
        let mut vm = BytecodeVm::new(constants);
        let err = vm
            .execute_chunk(&bytecode, 0, None)
            .expect_err("arity should fail");
        assert!(err.contains("return arity mismatch"));
    }
}
