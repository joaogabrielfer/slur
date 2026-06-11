#![allow(dead_code)]

use crate::lexer::Token;
use crate::value::RuntimeValueT;

#[derive(Debug, Clone)]
pub enum LexError {
    InvalidCharLiteral { line: usize, col: usize },
    UnclosedStringLiteral { line: usize, col: usize },
    UnexpectedCharacter { line: usize, col: usize, chr: char },
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidCharLiteral { line, col } => {
                write!(f, "invalid char literal at {line}:{col}")
            }
            Self::UnclosedStringLiteral { line, col } => {
                write!(f, "unclosed string literal at {line}:{col}")
            }
            Self::UnexpectedCharacter { line, col, chr } => {
                write!(f, "unexpected character '{chr}' at {line}:{col}")
            }
        }
    }
}

impl std::error::Error for LexError {}

#[derive(Debug, Clone)]
pub enum CompileError {
    UnexpectedToken {
        line: usize,
        col: usize,
        expected: Vec<Token>,
        got: Option<Token>,
    },
    UndeclaredObject {
        line: usize,
        col: usize,
        kind: String,
        name: String,
    },
    InvalidBytecodeSection {
        reason: String,
    },
    UnknownConstantTag(u8),
    Message(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedToken {
                line,
                col,
                expected,
                got,
            } => write!(
                f,
                "unexpected token at {line}:{col}: expected {expected:?}, got {got:?}"
            ),
            Self::UndeclaredObject {
                line,
                col,
                kind,
                name,
            } => write!(f, "undeclared {kind} '{name}' at {line}:{col}"),
            Self::InvalidBytecodeSection { reason } => {
                write!(f, "invalid bytecode section: {reason}")
            }
            Self::UnknownConstantTag(tag) => write!(f, "unknown constant tag 0x{tag:02x}"),
            Self::Message(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for CompileError {}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    StackUnderflow {
        ip: usize,
    },
    TypeMismatch {
        ip: usize,
        op: String,
        expected: Vec<RuntimeValueT>,
        got: Vec<RuntimeValueT>,
    },
    IndexOutOfBounds {
        ip: usize,
        index: i64,
        len: usize,
    },
    StackIndexOutOfBounds {
        ip: usize,
        op: String,
        index: usize,
        len: usize,
    },
    UndeclaredVariable {
        ip: usize,
        name: String,
    },
    InvalidFileDescriptor {
        ip: usize,
        fd: i64,
    },
    DivideByZero {
        ip: usize,
    },
    EmptyCollection {
        ip: usize,
        op: String,
    },
    UnknownOpcode {
        ip: usize,
        op: u8,
    },
    Message(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StackUnderflow { ip } => write!(f, "stack underflow at ip {ip}"),
            Self::TypeMismatch {
                ip,
                op,
                expected,
                got,
            } => write!(
                f,
                "type mismatch in {op} at ip {ip}: expected {expected:?}, got {got:?}"
            ),
            Self::IndexOutOfBounds { ip, index, len } => {
                write!(f, "index {index} out of bounds for length {len} at ip {ip}")
            }
            Self::StackIndexOutOfBounds { ip, op, index, len } => write!(
                f,
                "{op} stack index {index} out of bounds for length {len} at ip {ip}"
            ),
            Self::UndeclaredVariable { ip, name } => {
                write!(f, "undeclared variable '{name}' at ip {ip}")
            }
            Self::InvalidFileDescriptor { ip, fd } => {
                write!(f, "invalid file descriptor {fd} at ip {ip}")
            }
            Self::DivideByZero { ip } => write!(f, "divide by zero at ip {ip}"),
            Self::EmptyCollection { ip, op } => {
                write!(f, "{op} got an empty collection at ip {ip}")
            }
            Self::UnknownOpcode { ip, op } => write!(f, "unknown opcode 0x{op:02x} at ip {ip}"),
            Self::Message(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for RuntimeError {}
