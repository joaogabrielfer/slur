use std::{error::Error, fs::File, io::Write};
use crate::lexer::*;

#[repr(u8)] #[derive(Clone, Debug)]
pub enum OpCode {
    PushInt8 = 0x01,
    PushConst = 0x02,
    Add = 0x03,
    Len = 0x04,
}

pub fn to_opcodes_from_u8(bytes: Vec<u8>) -> Vec<OpCode>{
    bytes
        .iter()
        .filter(|b| **b < OpCode::Len as u8)
        .map(|b| unsafe { std::mem::transmute(*b)})
        .collect()
}

pub trait ToBytecode {
    fn to_opcodes(self) -> Vec<u8>;
}

impl ToBytecode for Vec<Token> {
    fn to_opcodes(self) -> Vec<u8> {
        let mut bytecode = Vec::new();

        for token in self {
            match token {
                Token::Push => {
                    bytecode.push(OpCode::PushInt8 as u8);
                }
                Token::NumberLit(n) => {
                    bytecode.push(n as u8);
                }
                Token::Add => {
                    bytecode.push(OpCode::Add as u8);
                }
                _ => todo!()
            }
        }

        bytecode
    }
}

pub fn compile(tokens: Vec<Token>, file_name: &str) -> Result<(), Box<dyn Error>>{
    let mut file = File::create(file_name)?;

    file.write_all(b"elem")?;
    println!("{:?}", tokens.clone().to_opcodes());
    file.write_all(&tokens.to_opcodes())?;
    Ok(())
}

pub fn run(mut ops: Vec<u8>) -> Result<(), Box<dyn Error>>{
    ops = ops.split_off(4);
    let mut ip = 0;
    let mut stack: Vec<i64> = vec![];
    while ip < ops.len(){
        let op = ops[ip];
        ip += 1;
        match op{
            0x01 => {
                stack.push(ops[ip] as i64);
                ip += 1;
            }
            0x02 => todo!(),
            0x03 => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(a + b);
            }
            _ => todo!("more ops")
        }
    }
    if let Some(result) = stack.pop(){
        println!("{result}")
    }
    Ok(())
}
