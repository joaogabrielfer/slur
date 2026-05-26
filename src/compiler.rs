use std::{error::Error, fs::File, io::Write};
use crate::lexer::*;

#[repr(u8)] #[derive(Clone, Debug)]
pub enum OpCode {
    PushInt8 = 0x01,
    PushConst = 0x02,
    Add = 0x03,
}

#[derive(Debug)]
pub struct PbcFile {
    pub magic: [u8; 4],
    pub version: u8,
    pub sections: Vec<Section>,
}

#[derive(Debug, Clone)]
pub enum Section {
    ConstantPool(Vec<Constant>),
    Bytecode(Vec<u8>),
    Unknown { tag: u8, payload: Vec<u8> },
}

impl Section{
    pub fn to_bytes(self) -> Vec<u8>{
        let mut bytes: Vec<u8> = vec![];

        match self{
            Section::ConstantPool(constants) => {
                bytes.push(0x01);
                for c in constants{
                    bytes.append(&mut c.to_bytes());
                }
            }
            Section::Bytecode(items) => {
                bytes.push(0x02);
                bytes.append(&mut items.clone());
            }
            Section::Unknown { tag, payload } => {
                bytes.push(tag);
                bytes.append(&mut payload.clone());
            }
        }

        bytes
    }
}

#[derive(Debug, Clone)]
pub enum Constant {
    String(String),
    Integer(i64),
    Function(Vec<u8>),
}

impl Constant{
    pub fn to_bytes(self) -> Vec<u8>{
        let mut bytes: Vec<u8> = vec![];

        match self{
            Constant::String(s) => {
                bytes.push(0x01);
                bytes.append(&mut s.into_bytes());
            }
            Constant::Integer(i) => {
                bytes.push(0x02);
                bytes.append(&mut i.to_le_bytes().to_vec());
            }
            Constant::Function(items) => {
                bytes.push(0x03);
                bytes.append(&mut items.clone());
            }
        }

        bytes
    }
}

impl PbcFile {
    pub fn new() -> Self{
        PbcFile { magic: *b"ELEM", version: 0x01, sections: vec![] }
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut cursor = 0;

        let magic: [u8; 4] = bytes[cursor..cursor+4].try_into().unwrap();
        if magic != *b"ELEM" {
            return Err("Invalid Magic Bytes".to_string()); // TODO: error handling
        }
        let version = bytes[4];
        cursor += 5;

        let mut sections = Vec::new();

        while cursor < bytes.len() {
            let tag = bytes[cursor];
            cursor += 1;

            if tag == 0xFF { break; }

            let length_bytes = [bytes[cursor], bytes[cursor+1], bytes[cursor+2], bytes[cursor+3]];
            let length = u32::from_le_bytes(length_bytes) as usize;
            cursor += 4;

            let payload = &bytes[cursor .. cursor + length];

            let section = match tag {
                0x01 => Section::ConstantPool(parse_constants(payload)),
                0x02 => Section::Bytecode(payload.to_vec()),
                _ => Section::Unknown { tag, payload: payload.to_vec() }
            };

            sections.push(section);
            cursor += length;
        }

        Ok(PbcFile { magic, version, sections })
    }

    pub fn compile_to_file(&mut self, file_name: &str) -> Result<(), Box<dyn Error>>{
        let mut file = File::create(file_name)?;
        let mut bytes: Vec<u8> = vec![];

        bytes.append(&mut self.magic.to_vec());
        bytes.push(self.version);

        for s in &mut self.sections{
            bytes.append(&mut s.clone().to_bytes());
        }

        file.write_all(bytes.as_slice())?;
        Ok(())
    }

    // pub fn run(self) -> Result<(), Box<dyn Error>>{
    //     let constants: Vec<Constant>;
    //     let mut ops: Vec<u8>;
    //     for s in self.sections{
    //         match s{
    //             Section::ConstantPool(cs) => {
    //
    //             }
    //             Section::Bytecode(b) => ops = b,
    //             Section::Unknown { tag, payload } => todo!(),
    //         }
    //     }
    //     let mut ip = 0;
    //     let mut stack: Vec<i64> = vec![];
    //     while ip < ops.len(){
    //         let op = ops[ip];
    //         ip += 1;
    //         match op{
    //             0x01 => {
    //                 stack.push(ops[ip] as i64);
    //                 ip += 1;
    //             }
    //             0x02 => todo!(),
    //             0x03 => {
    //                 let b = stack.pop().unwrap();
    //                 let a = stack.pop().unwrap();
    //                 stack.push(a + b);
    //             }
    //             _ => todo!("more ops")
    //         }
    //     }
    //     if let Some(result) = stack.pop(){
    //         println!("{result}")
    //     }
    // Ok(())
    // }
}

fn parse_constants(payload: &[u8]) -> Vec<Constant> {
    let mut constants: Vec<Constant> = vec![];
    let mut cursor = 0;

    let mut constants_len = payload[cursor];
    cursor += 1;

    while constants_len > 0 {
        let tag = payload[cursor];
        cursor += 1;
        let c = match tag{
            0x01 => {
                let len = payload[cursor] as usize;
                cursor += 1;
                let s = str::from_utf8(&payload[cursor..cursor+len]).unwrap(); //TODO: improve error handling in this function
                cursor += len;

               Constant::String(s.to_string())
            }
            0x02 => Constant::Integer(i64::from_le_bytes(payload[cursor..cursor+8].try_into().expect("could not parse payload into integer"))), // TODO: improve here too
            0x03 => todo!(),
            _other => todo!(),
        };
        constants_len -= 1;
        constants.push(c);
    }

    constants
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
