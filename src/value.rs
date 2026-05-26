use std::error::Error;
use std::{collections::HashMap, fmt::Display, fs::File, rc::Rc};
use crate::lexer::Token;
use crate::compiler::*;

pub struct PVM {
    pub data_stack: Vec<RuntimeValue>,
    pub call_stack: Vec<CallFrame>,
    pub elements: HashMap<String, Element>,
    pub file_index: Vec<FileDescriptor>,
}

impl PVM {
    pub fn new(file: PbcFile) -> Result<Self, Box<dyn Error>> {
        let mut constants = Vec::new();
        let mut bytecode: Option<Vec<u8>> = None;

        // Loop through the sections EXACTLY ONCE. Extremely fast.
        for section in file.sections {
            match section {
                Section::ConstantPool(pool) => constants = pool,
                Section::Bytecode(b) => bytecode = Some(b),
                Section::Unknown { .. } => { /* Ignore or log warning */ }
                // _ => {} for future sections like DebugLines
            }
        }

        // Safely throw an error if no bytecode was found!
        let ops = bytecode.ok_or("Invalid PBC File: Missing Bytecode Section")?;

        Ok(PVM {
            data_stack: Vec::new(),
            call_stack: vec![CallFrame{
                // instructions: ops,
                instructions: vec![],
                frame_pointer: 0,
                ip: 0,
            }],
            // elements: constants.into_elements(),
            elements: HashMap::new(),
            file_index: vec![FileDescriptor::Stdin, FileDescriptor::Stdout, FileDescriptor::Stderr]
        })
    }
}

pub enum FileDescriptor {
    Stdin,
    Stdout,
    Stderr,
    DiskFile(File),
    Empty,
}

#[derive(Clone, Debug)]
pub enum Element{
    Var(RuntimeValue),
    Function {
        patterns: Vec<Pattern>,
        guard: Option<Vec<Token>>,
        block: Vec<Token>,
    }
}

pub struct CallFrame {
    pub instructions: Vec<Token>,
    pub ip: usize,
    pub frame_pointer: usize,
}
impl CallFrame {
    pub fn peek(&self) -> Option<&Token> {
        self.instructions.get(self.ip)
    }

    pub fn next(&mut self) -> Option<&Token> {
        let result = self.instructions.get(self.ip);
        self.ip += 1;
        result
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum Pattern {
    Type(RuntimeValueT),
    Literal(RuntimeValue),
    Range { start: i64, end: i64, inclusive: bool },
    List(Vec<Pattern>),
    Destructure(Box<Pattern>, Box<Pattern>),
    Fallback,
    Variadic(Box<Pattern>),
}

impl Pattern {
    pub fn check(&self, value: &RuntimeValue) -> bool {
        match self {
            Pattern::Type(t) => value.compare_type(t.clone()),
            Pattern::Literal(lit) => value == lit,
            Pattern::Range { start, end, inclusive } => {
                if let RuntimeValue::Int(n) = value {
                    if *inclusive { n >= start && n <= end } else { n >= start && n < end }
                } else {
                    false
                }
            }
            Pattern::Fallback => true,
            Pattern::List(pat_list) => {
                if let RuntimeValue::List(val_list) = value {
                    let variadic_pos = pat_list.iter().position(|p| matches!(p, Pattern::Variadic(_)));
                    if let Some(v_idx) = variadic_pos {
                        let fixed_before = v_idx;
                        let fixed_after = pat_list.len() - 1 - v_idx;
                        if val_list.len() < fixed_before + fixed_after {
                            return false;
                        }
                        for i in 0..fixed_before {
                            if !pat_list[i].check(&val_list[i]) { return false; }
                        }
                        let val_after_start = val_list.len() - fixed_after;
                        for i in 0..fixed_after {
                            if !pat_list[v_idx + 1 + i].check(&val_list[val_after_start + i]) { return false; }
                        }
                        let inner_pat = if let Pattern::Variadic(inner) = &pat_list[v_idx] { inner } else { unreachable!() };
                        for i in fixed_before..val_after_start {
                            if !inner_pat.check(&val_list[i]) { return false; }
                        }
                        true
                    } else {
                        if pat_list.len() == 1 && matches!(pat_list[0], Pattern::Type(_)) && val_list.len() != 1 {
                             let t = &pat_list[0];
                             for v in val_list {
                                 if !t.check(v) { return false; }
                             }
                             return true;
                        }
                        if pat_list.len() != val_list.len() {
                            return false;
                        }
                        for (p, v) in pat_list.iter().zip(val_list.iter()) {
                            if !p.check(v) {
                                return false;
                            }
                        }
                        true
                    }
                } else if let RuntimeValue::String(s) = value {
                    if pat_list.len() == 1 {
                        return pat_list[0].check(value);
                    }
                    let chars: Vec<char> = s.chars().collect();
                    if pat_list.len() != chars.len() { return false; }
                    for (p, c) in pat_list.iter().zip(chars.iter()) {
                        if !p.check(&RuntimeValue::Char(*c)) { return false; }
                    }
                    true
                } else {
                    false
                }
            }
            Pattern::Destructure(head_pat, tail_pat) => {
                if let RuntimeValue::List(val_list) = value {
                    if val_list.is_empty() { return false; }
                    let head_val = &val_list[0];
                    let tail_val = RuntimeValue::List(val_list[1..].to_vec());
                    head_pat.check(head_val) && tail_pat.check(&tail_val)
                } else if let RuntimeValue::String(s) = value {
                    if s.is_empty() { return false; }
                    let mut chars = s.chars();
                    let head_char = chars.next().unwrap();
                    let tail_str = chars.collect::<String>();
                    let head_val = RuntimeValue::Char(head_char);
                    let tail_val = RuntimeValue::String(Rc::new(tail_str));
                    head_pat.check(&head_val) && tail_pat.check(&tail_val)
                } else {
                    false
                }
            }
            Pattern::Variadic(_) => true, // Variadic checks are handled by the call frame resolver
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum RuntimeValue {
    Int(i64),
    Bool(bool),
    String(Rc<String>),
    Char(char),
    Block(Vec<Token>),
    Function{
        patterns: Vec<Pattern>,
        guard: Option<Vec<Token>>,
        block: Vec<Token>,
    },
    List(Vec<RuntimeValue>),
    Type(RuntimeValueT),
}

pub fn _default_runtime_int() -> RuntimeValue{
    RuntimeValue::Int(0)
}

pub fn _default_runtime_bool() -> RuntimeValue{
    RuntimeValue::Bool(false)
}

pub fn _default_runtime_string() -> RuntimeValue{
    RuntimeValue::String(Rc::new("".to_string()))
}

pub fn _default_runtime_char() -> RuntimeValue{
    RuntimeValue::Char('\0')
}

pub fn _default_runtime_block() -> RuntimeValue{
    RuntimeValue::Block(vec![])
}

pub fn _default_runtime_function() -> RuntimeValue{
    RuntimeValue::Function{patterns: vec![], guard: None, block:vec![]}
}

pub fn _default_runtime_list() -> RuntimeValue{
    RuntimeValue::Block(vec![])
}

pub fn _default_runtime_type() -> RuntimeValue{
    RuntimeValue::Type(RuntimeValueT::Type)
}

#[derive(Clone, PartialEq, Debug)]
pub enum RuntimeValueT {
    Int,
    Bool,
    String,
    Char,
    Block,
    Function,
    Any,
    Variadic(Box<RuntimeValueT>),
    List(Vec<RuntimeValueT>),
    Type,
    Unknown,
}

impl Display for RuntimeValueT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            RuntimeValueT::Int         => write!(f, "int"),
            RuntimeValueT::Bool        => write!(f, "bool" ),
            RuntimeValueT::String      => write!(f, "string" ),
            RuntimeValueT::Char        => write!(f, "char" ),
            RuntimeValueT::Block       => write!(f, "block" ),
            RuntimeValueT::Function    => write!(f, "function" ),
            RuntimeValueT::Any         => write!(f, "any" ),
            RuntimeValueT::Variadic(v) => write!(f, "variadic({v})"),
            RuntimeValueT::List(l)     => write!(f, "list({:?})", l),
            RuntimeValueT::Type        => write!(f, "type"),
            RuntimeValueT::Unknown     => panic!("tried to print Unknown type"),
        }
    }
}

pub fn get_type_from_str(s: &str) -> RuntimeValueT{
    match s {
        "string"   => RuntimeValueT::String,
        "char"     => RuntimeValueT::Char,
        "int"      => RuntimeValueT::Int,
        "bool"     => RuntimeValueT::Bool,
        "function" => RuntimeValueT::Function,
        "any"      => RuntimeValueT::Any,
        "type"     => RuntimeValueT::Type,
        _ => RuntimeValueT::Unknown,
    }
}

impl RuntimeValue {
    pub fn type_name(&self) -> &'static str {
        match self {
            RuntimeValue::Int(_)       => "int",
            RuntimeValue::Bool(_)      => "bool",
            RuntimeValue::String(_)    => "str",
            RuntimeValue::Char(_)      => "char",
            RuntimeValue::Block(_)     => "block",
            RuntimeValue::Function{..} => "function",
            RuntimeValue::List(_)      => "list",
            RuntimeValue::Type(_)      => "type",
        }
    }
    pub fn compare_type(&self, t: RuntimeValueT) -> bool {
        if t == RuntimeValueT::Any { return true }
        match self {
            RuntimeValue::Int(_)       => t == RuntimeValueT::Int,
            RuntimeValue::Bool(_)      => t == RuntimeValueT::Bool,
            RuntimeValue::String(_)    => t == RuntimeValueT::String,
            RuntimeValue::Char(_)      => t == RuntimeValueT::Char,
            RuntimeValue::Block(_)     => t == RuntimeValueT::Block,
            RuntimeValue::Function{..} => t == RuntimeValueT::Function,
            RuntimeValue::List(l)      => t == RuntimeValueT::List(l.iter().map(|t| t.get_type()).collect()),
            RuntimeValue::Type(_)      => t == RuntimeValueT::Type
        }
    }

    pub fn get_type(&self) -> RuntimeValueT{
        match self {
            RuntimeValue::Int(_)          => RuntimeValueT::Int,
            RuntimeValue::Bool(_)         => RuntimeValueT::Bool,
            RuntimeValue::String(_)       => RuntimeValueT::String,
            RuntimeValue::Char(_)         => RuntimeValueT::Char,
            RuntimeValue::Block(_)        => RuntimeValueT::Block,
            RuntimeValue::Function { .. } => RuntimeValueT::Function,
            RuntimeValue::List(_)         => RuntimeValueT::List(vec![]),
            RuntimeValue::Type(_)         => RuntimeValueT::Type,
        }
    }
}

impl PartialOrd for RuntimeValue {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a.partial_cmp(b),
            (Self::Bool(a), Self::Bool(b)) => a.partial_cmp(b),
            (Self::Char(a), Self::Char(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            RuntimeValue::Int(n) => write!(f, "{n}"),
            RuntimeValue::String(s) => write!(f, "{s}"),
            RuntimeValue::Char(c) => write!(f, "{c}"),
            RuntimeValue::Bool(b) => write!(f, "{b}"),
            RuntimeValue::Block(b) => write!(f, "{:?}", b),
            RuntimeValue::Function { patterns, guard, block } => write!(f, "({:?}) when {:?} {{{:?}}}", patterns, guard, block),
            RuntimeValue::List(v) => write!(f, "{:?}", v),
            RuntimeValue::Type(t) => write!(f, "@{t}"),
        }
    }
}

impl std::fmt::Debug for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self, f)
    }
}

impl std::ops::Neg for RuntimeValue {
    type Output = Self;

    fn neg(self) -> Self::Output {
        match self{
            Self::Int(n) => Self::Int(-n),
            _ => panic!("Mismatch types while negating RuntimeValue")
        }
    }
}

impl std::ops::Div for RuntimeValue {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs){
            (Self::Int(n1), Self::Int(n2)) => Self::Int(n1 / n2),
            _ => panic!("Mismatch types while dividing RuntimeValue")
        }
    }
}

impl std::ops::Sub for RuntimeValue {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs){
            (Self::Int(n1), Self::Int(n2)) => Self::Int(n1 - n2),
            _ => panic!("Mismatch types while subtractin RuntimeValue")
        }
    }
}

impl std::ops::Mul for RuntimeValue {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs){
            (Self::Int(n1), Self::Int(n2)) => Self::Int(n1 * n2),
            _ => panic!("Mismatch types while multiplying RuntimeValue")
        }
    }
}

impl std::ops::Add for RuntimeValue {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs){
            (Self::Int(n1), Self::Int(n2)) => Self::Int(n1 + n2),
            _ => panic!("Mismatch types while adding RuntimeValue")
        }
    }
}
