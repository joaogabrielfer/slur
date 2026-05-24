use std::fs::{File, read_to_string};
use std::io::{Read, Write};
use std::process::exit;
use std::error::Error;
use std::rc::Rc;

use crate::error::{LangError, ret_error};
use crate::lexer::{Token, tokenize};
use crate::value::*;

pub static STD_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/std");

pub enum Flow{
    Next,
    Return
}

type PatternResult = Result<Option<(usize, Vec<RuntimeValue>)>, Box<dyn Error>>;

impl PVM {
    pub fn interpret(&mut self) -> Result<Flow, Box<dyn Error>>{
        while !self.call_stack.is_empty() {
            if let Flow::Return = self.interpret_step()? {
                return Ok(Flow::Return);
            }
        }
        Ok(Flow::Next)
    }

    pub fn interpret_step(&mut self) -> Result<Flow, Box<dyn Error>> {
        let mut frame = match self.call_stack.pop() {
            Some(f) => f,
            None => return Ok(Flow::Next),
        };

        if frame.ip >= frame.instructions.len(){
            return Ok(Flow::Next);
        }

        let tk = frame.instructions[frame.ip].clone();
        frame.ip += 1;

        let mut pending_call: Option<CallFrame> = None;

            match tk {
                Token::Quit => {
                    println!("Exiting program...");
                    exit(0)
                }
                Token::Push => {
                    let mut is_first = true;

                    loop {
                        if !is_first {
                            match frame.peek() {
                                Some(&Token::NumberLit(_))
                                    | Some(&Token::QuotedLit(_))
                                    | Some(&Token::CharLit(_))
                                    | Some(&Token::TypeLit(_))
                                    | Some(&Token::CloseSquare)
                                    | Some(&Token::OpenSquare)=> { }
                                _ => break
                            }
                        }

                        match frame.next() {
                            Some(Token::NumberLit(n)) => self.data_stack.push(RuntimeValue::Int(*n)),
                            Some(Token::TypeLit(t)) => self.data_stack.push(RuntimeValue::Type(t.clone())),
                            Some(Token::QuotedLit(s)) => {
                                let trimmed = s.trim_matches('\"').to_string();
                                self.data_stack.push(RuntimeValue::String(Rc::new(unescape_string(trimmed.as_str()))));
                            }
                            Some(Token::CharLit(c)) => self.data_stack.push(RuntimeValue::Char(*c)),
                            Some(Token::UnquotedLit(s)) => {
                                if self.elements.contains_key(s) {
                                    match self.elements[s].clone(){
                                        Element::Var(runtime_value) => self.data_stack.push(runtime_value),
                                        Element::Function { patterns, guard, block } => {
                                            self.data_stack.push(RuntimeValue::Function {
                                                patterns,
                                                guard,
                                                block
                                            })
                                        }
                                    }
                                } else {
                                    ret_error!(UndeclaredObject { t: "variable", name: s })
                                }
                            }
                            Some(Token::OpenSquare) => {
                                let mut list: Vec<RuntimeValue> = vec![];
                                while frame.peek() != Some(&Token::CloseSquare){
                                    match frame.next() {
                                        Some(Token::NumberLit(n)) => list.push(RuntimeValue::Int(*n)),
                                        Some(Token::TypeLit(t)) => list.push(RuntimeValue::Type(t.clone())),
                                        Some(Token::QuotedLit(s)) => {
                                            let trimmed = s.trim_matches('\"').to_string();
                                            list.push(RuntimeValue::String(Rc::new(unescape_string(trimmed.as_str()))));
                                        }
                                        other => ret_error!(UnexpectedToken,[QuotedLit, UnquotedLit, NumberLit], other)
                                    }
                                }
                                frame.next();
                                self.data_stack.push(RuntimeValue::List(list));
                            }
                            other => {
                                ret_error!(UnexpectedToken,[QuotedLit, UnquotedLit, NumberLit], other)
                            }
                        }
                        is_first = false;
                    }
                }
                Token::Add => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }
                    let b = self.data_stack.pop().unwrap();
                    let a = self.data_stack.pop().unwrap();
                    match (a.clone(), b.clone()){
                        (RuntimeValue::Int(_), RuntimeValue::Int(_)) => self.data_stack.push(a + b),
                        (type1, type2) => ret_error!(UnexpectedTypes,[RuntimeValue::Int(0), RuntimeValue::Int(0)], vec![type1, type2])
                    }
                }
                Token::Mul =>{
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }
                    let b = self.data_stack.pop().unwrap();
                    let a = self.data_stack.pop().unwrap();
                    match (a.clone(), b.clone()){
                        (RuntimeValue::Int(_), RuntimeValue::Int(_)) => self.data_stack.push(a * b),
                        (type1, type2) => ret_error!(UnexpectedTypes,[RuntimeValue::Int(0), RuntimeValue::Int(0)], vec![type1, type2])
                    }
                }
                Token::Sub =>{
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }
                    let b = self.data_stack.pop().unwrap();
                    let a = self.data_stack.pop().unwrap();
                    match (a.clone(), b.clone()){
                        (RuntimeValue::Int(_), RuntimeValue::Int(_)) => self.data_stack.push(a - b),
                        (type1, type2) => ret_error!(UnexpectedTypes, [RuntimeValue::Int(0), RuntimeValue::Int(0)], vec![type1, type2])
                    }
                }
                Token::Div =>{
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }
                    let b = self.data_stack.pop().unwrap();
                    let a = self.data_stack.pop().unwrap();
                    match (a.clone(), b.clone()){
                        (RuntimeValue::Int(_), RuntimeValue::Int(_)) => self.data_stack.push(a / b),
                        (type1, type2) => ret_error!(UnexpectedTypes,[RuntimeValue::Int(0), RuntimeValue::Int(0)], vec![type1, type2])
                    }
                }
                Token::Drop => {
                    if self.data_stack.len() <= frame.frame_pointer{
                        ret_error!(StackUnderflow)
                    }
                    self.data_stack.pop().unwrap_or_else(|| unreachable!("drop"));
                }
                Token::Neg => {
                    if let Some(n) = self.data_stack.pop(){
                        match n {
                            RuntimeValue::Int(_) => self.data_stack.push(-n),
                            other => ret_error!(UnexpectedTypes, [RuntimeValue::Int(0)], vec![other])
                        }
                    } else {
                        ret_error!(UnsufficientValues { op: "neg", exp: 1_usize, got: self.data_stack.len() })
                    }
                }
                Token::Dup => {
                    if self.data_stack.is_empty() {
                        ret_error!(UnsufficientValues { op: "dup", exp: 1_usize, got: self.data_stack.len() })
                    }

                    self.data_stack.push(self.data_stack[self.data_stack.len() - 1].clone())
                }
                Token::NumberLit(num) => ret_error!(InvalidToken, Token::NumberLit(num)),
                Token::QuotedLit(s) => ret_error!(InvalidToken, Token::QuotedLit(s.to_string())),
                Token::CharLit(c) => ret_error!(InvalidToken, Token::CharLit(c)),
                Token::UnquotedLit(s) => {
                    if !self.elements.contains_key(&s){
                        ret_error!(UndeclaredObject { t: "element", name: s })
                    }

                    match self.elements[&s].clone(){
                        Element::Var(v) => {
                            match v {
                                RuntimeValue::List(ref l) if l.iter().all(|x| matches!(x, RuntimeValue::Function{..})) => {
                                    pending_call = self.execute_function_or_list(v)?;
                                }
                                _ => self.data_stack.push(v),
                            }
                        }
                        Element::Function { patterns, guard, block, ..} => {
                            pending_call = self.execute_function_or_list(RuntimeValue::Function { patterns, guard, block })?;
                        }
                    }

                }
                Token::Into => {
                    let var_name_opt = frame.next();
                    let var_name: String = match var_name_opt {
                        Some(Token::UnquotedLit(s)) => s.to_string(),
                        other => ret_error!(UnexpectedToken, [UnquotedLit], other),
                    };

                    match self.data_stack.len() - frame.frame_pointer {
                        0..1 => ret_error!(StackUnderflow),
                        1.. => {
                            let var = self.data_stack.pop().unwrap_or_else(|| unreachable!("into"));
                            match var {
                                RuntimeValue::Function { patterns, guard, block  } => self.elements.insert(var_name, Element::Function{patterns, guard, block}),
                                RuntimeValue::List(ref l) if l.iter().all(|x| matches!(x, RuntimeValue::Function{..})) => {
                                    self.elements.insert(var_name, Element::Var(var))
                                }
                                _ => self.elements.insert(var_name, Element::Var(var)),
                            };
                        }
                    }
                }
                Token::Swap => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }
                    let n1 = self.data_stack.pop().unwrap_or_else(|| unreachable!("Swap"));
                    let n2 = self.data_stack.pop().unwrap_or_else(|| unreachable!("Swap"));

                    self.data_stack.push(n1);
                    self.data_stack.push(n2);
                }
                Token::Rot => {
                    if self.data_stack.len() - frame.frame_pointer < 3 {
                        ret_error!(StackUnderflow)
                    }

                    let a = self.data_stack.remove(self.data_stack.len() - 3);

                    self.data_stack.push(a);
                }
                Token::Over => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    self.data_stack.push(self.data_stack[self.data_stack.len() - 2].clone());
                }
                Token::BoolLit(b) => self.data_stack.push(RuntimeValue::Bool(b)),
                Token::Eq => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let b = self.data_stack.pop().unwrap_or_else(|| unreachable!("eq"));
                    let a = self.data_stack.pop().unwrap_or_else(|| unreachable!("eq"));

                    self.data_stack.push(RuntimeValue::Bool(a == b));
                }
                Token::Gt => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let b = self.data_stack.pop().unwrap_or_else(|| unreachable!("gt"));
                    let a = self.data_stack.pop().unwrap_or_else(|| unreachable!("gt"));

                    match (a, b) {
                        (RuntimeValue::Int(n1), RuntimeValue::Int(n2)) => {
                            self.data_stack.push(RuntimeValue::Bool(n1 > n2));
                        }
                        (type1, type2) => {
                            ret_error!(UnexpectedTypes,[RuntimeValue::Int(0), RuntimeValue::Int(0)], vec![type1, type2])
                        }
                    }
                }
                Token::Lt => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let b = self.data_stack.pop().unwrap_or_else(|| unreachable!("lt"));
                    let a = self.data_stack.pop().unwrap_or_else(|| unreachable!("lt"));

                    match (a, b) {
                        (RuntimeValue::Int(n1), RuntimeValue::Int(n2)) => {
                            self.data_stack.push(RuntimeValue::Bool(n1 < n2));
                        }
                        (type1, type2) => {
                            ret_error!(UnexpectedTypes, [RuntimeValue::Int(0), RuntimeValue::Int(0)], vec![type1, type2])
                        }
                    }
                }
                Token::If => {
                    if self.data_stack.is_empty() {
                        ret_error!(UnsufficientValues { op: "if", exp: 1_usize, got: self.data_stack.len() })
                    }
                    let a = self.data_stack.pop().unwrap_or_else(|| unreachable!("if"));

                    let condition = match a {
                        RuntimeValue::Bool(c) => c,
                        other => ret_error!(UnexpectedTypes,[RuntimeValue::Bool(true)], vec![other])
                    };

                    expect_open_curly(frame.next())?;

                    let if_branch_vec = collect_tokens_into_block(&mut frame)?;
                    if let Some(&Token::Else) = frame.peek() {
                        frame.next();

                        expect_open_curly(frame.next())?;

                        let else_branch_vec = collect_tokens_into_block(&mut frame)?;

                        if condition {
                            pending_call = Some(CallFrame {
                                instructions: if_branch_vec,
                                ip: 0,
                                frame_pointer: 0
                            })
                        } else {
                            pending_call = Some(CallFrame {
                                instructions: else_branch_vec,
                                ip: 0,
                                frame_pointer: 0
                            })
                        }
                    } else {
                        pending_call = Some(CallFrame {
                            instructions: if_branch_vec,
                            ip: 0,
                            frame_pointer: 0
                        })
                    }
                }
                Token::Else => ret_error!(InvalidToken, Token::Else),
                Token::OpenCurly => {
                    let block = collect_tokens_into_block(&mut frame)?;
                    match self.data_stack.pop() {
                        Some(RuntimeValue::Function { patterns, guard, .. }) => {
                            self.data_stack.push(RuntimeValue::Function {
                                patterns,
                                guard,
                                block
                            });
                        }
                        Some(other) => {
                            self.data_stack.push(other);
                            self.data_stack.push(RuntimeValue::Block(block));
                        }
                        None => {
                            self.data_stack.push(RuntimeValue::Block(block));
                        }
                    }
                }
                Token::CloseCurly => { },
                Token::And => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let b = self.data_stack.pop().unwrap_or_else(|| unreachable!("and"));
                    let a = self.data_stack.pop().unwrap_or_else(|| unreachable!("and"));

                    match (a, b) {
                        (RuntimeValue::Bool(b1), RuntimeValue::Bool(b2)) => {
                            self.data_stack.push(RuntimeValue::Bool(b1 && b2));
                        }
                        (type1, type2) => ret_error!(UnexpectedTypes, [RuntimeValue::Bool(false), RuntimeValue::Bool(false)], vec![type1, type2])
                    }
                }
                Token::Or => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let b = self.data_stack.pop().unwrap_or_else(|| unreachable!("or"));
                    let a = self.data_stack.pop().unwrap_or_else(|| unreachable!("or"));

                    match (a, b) {
                        (RuntimeValue::Bool(b1), RuntimeValue::Bool(b2)) => {
                            self.data_stack.push(RuntimeValue::Bool(b1 || b2));
                        }
                        (type1, type2) => ret_error!(UnexpectedTypes,[RuntimeValue::Bool(false), RuntimeValue::Bool(false)], vec![type1, type2])
                    }
                }
                Token::Not => {
                    if self.data_stack.len() <= frame.frame_pointer{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("not")){
                        RuntimeValue::Bool(b) => self.data_stack.push(RuntimeValue::Bool(!b)),
                        other => ret_error!(UnexpectedTypes, [RuntimeValue::Bool(false)], vec![other])
                    }
                }
                Token::ElementCall(element_name) => {
                    if element_name.is_empty(){
                        ret_error!(UnexpectedToken, [UnquotedLit], None::<Token>)
                    }
                    if !self.elements.contains_key(&element_name) {
                        ret_error!(UndeclaredObject { t: "function", name: element_name })
                    }

                    match self.elements[element_name.as_str()].clone() {
                        Element::Var(v) => {
                            match v {
                                RuntimeValue::List(ref l) if l.iter().all(|x| matches!(x, RuntimeValue::Function{..})) => {
                                    pending_call = self.execute_function_or_list(v)?;
                                }
                                _ => self.data_stack.push(v),
                            }
                        }
                        Element::Function { patterns, guard, block } => {
                            pending_call = self.execute_function_or_list(RuntimeValue::Function { patterns, guard, block })?;
                        }
                    }
                }
                Token::Len => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("len")){
                        RuntimeValue::List(l) => {
                            self.data_stack.push(RuntimeValue::Int(l.len() as i64));
                        }
                        RuntimeValue::String(s) => {
                            self.data_stack.push(RuntimeValue::Int(s.len() as i64));
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };
                }
                Token::StackLen => self.data_stack.push(RuntimeValue::Int(self.data_stack.len().try_into().unwrap())),
                Token::Ret => return Ok(Flow::Return),
                Token::Include => {
                    match frame.next(){
                        Some(Token::QuotedLit(_s)) => {

                        },
                        Some(Token::UnquotedLit(s)) => {
                            if s.contains('/') || s.contains('\\') || s.contains("..") {
                                return Err(LangError::InvalidPath(s.clone()).into());
                            }
                            let new_s = format!("{s}.slur");

                            let target_path = std::path::Path::new(STD_LIB_PATH).join(new_s);

                            match read_to_string(&target_path) {
                                Ok(content) => {
                                    let tokens = tokenize(content);
                                    pending_call = Some(CallFrame{
                                        instructions: tokens,
                                        ip: 0,
                                        frame_pointer: self.data_stack.len()
                                    })
                                }
                                Err(_) => ret_error!(FileNotFound { file: s.clone(), reason: "No module with this name." })
                            }
                        }
                        other => ret_error!(UnexpectedToken,[QuotedLit, UnquotedLit], other)
                    }
                }
                Token::ToChar => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("tochar")){
                        RuntimeValue::Char(c) => self.data_stack.push(RuntimeValue::Char(c)),
                        RuntimeValue::String(s) => {
                            if s.len() != 1 {
                                self.data_stack.push(RuntimeValue::Bool(false));
                            } else {
                                self.data_stack.push(RuntimeValue::Char((*s).chars().nth(0).unwrap_or_else(|| unreachable!())));
                                self.data_stack.push(RuntimeValue::Bool(true));
                            }
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_string(), _default_runtime_char()], vec![other])
                    }
                }
                Token::ToInt => {
                    if self.data_stack.len() <= frame.frame_pointer{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("intb")){
                        RuntimeValue::String(s) => match (*s).parse::<i64>(){
                            Ok(n) => {
                                self.data_stack.push(RuntimeValue::Int(n));
                                self.data_stack.push(RuntimeValue::Bool(true));
                            }
                            Err(_) => self.data_stack.push(RuntimeValue::Bool(false)),
                        }
                        RuntimeValue::Bool(b) => if b {
                            self.data_stack.push(RuntimeValue::Int(1));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        } else {
                            self.data_stack.push(RuntimeValue::Int(0));
                            self.data_stack.push(RuntimeValue::Bool(false));
                        }
                        RuntimeValue::Int(i) =>{
                            self.data_stack.push(RuntimeValue::Int(i));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        other => ret_error!(UnexpectedTypes, [RuntimeValue::Int(0), RuntimeValue::Bool(false), _default_runtime_string()], vec![other])
                    };
                }
                Token::ToString => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("tostring")){
                        RuntimeValue::Int(i) => {
                            self.data_stack.push(RuntimeValue::String(Rc::new(i.to_string())));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::Bool(b) => if b {
                            self.data_stack.push(RuntimeValue::String(Rc::new(b.to_string())));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::String(s) =>{
                            self.data_stack.push(RuntimeValue::String(s));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::List(l) =>{
                            self.data_stack.push(RuntimeValue::String(Rc::new(format!("{:?}", l))));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::Type(t) =>{
                            self.data_stack.push(RuntimeValue::String(Rc::new(t.to_string())));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        _ => self.data_stack.push(RuntimeValue::Bool(false)),
                    };
                }
                Token::ToBool => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("tostring")){
                        RuntimeValue::Int(i) => {
                            if i != 0{
                                self.data_stack.push(RuntimeValue::Bool(true));
                            } else {
                                self.data_stack.push(RuntimeValue::Bool(false));
                            }
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::Bool(b) => if b {
                            self.data_stack.push(RuntimeValue::Bool(b));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::String(s) =>{
                            self.data_stack.push(RuntimeValue::Bool(!s.is_empty()));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        RuntimeValue::List(l) =>{
                            self.data_stack.push(RuntimeValue::Bool(!l.is_empty()));
                            self.data_stack.push(RuntimeValue::Bool(true));
                        }
                        other => ret_error!(UnexpectedTypes, [RuntimeValue::Int(0), RuntimeValue::Bool(false), _default_runtime_string()], vec![other])
                    };
                }
                Token::Clear => {
                    if self.data_stack.len() <= frame.frame_pointer{
                        ret_error!(StackUnderflow)
                    }
                    self.data_stack.clear();
                }
                Token::Roll => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let index = match self.data_stack.pop().unwrap_or_else(||unreachable!("Roll")){
                        RuntimeValue::Int(i) => i as usize,
                        other => ret_error!(UnexpectedTypes, [RuntimeValue::Int(0)], vec![other])
                    };

                    if index > self.data_stack.len(){
                        ret_error!(StackIndexOutOfRange { op: "Roll".to_string(), index: index, stack_len: self.data_stack.len() })
                    }

                    let result = self.data_stack.remove(self.data_stack.len() - index);
                    self.data_stack.push(result);
                }
                Token::Pick => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let index = match self.data_stack.pop().unwrap_or_else(||unreachable!("Pick")){
                        RuntimeValue::Int(i) => i as usize,
                        other => ret_error!(UnexpectedTypes, [RuntimeValue::Int(0)], vec![other])
                    };

                    if index > self.data_stack.len(){
                        ret_error!(StackIndexOutOfRange { op: "Pick".to_string(), index: index, stack_len: self.data_stack.len() })
                    }

                    self.data_stack.push(self.data_stack[self.data_stack.len() - index].clone());
                }
                Token::OpenParen => {
                    let patterns = parse_patterns(&mut frame)?;
                    let mut guard = None;
                    if frame.peek() == Some(&Token::When) {
                        frame.next(); // consume 'when'
                        expect_open_curly(frame.next())?;
                        guard = Some(collect_tokens_into_block(&mut frame)?);
                    }
                    self.data_stack.push(RuntimeValue::Function{
                        patterns,
                        guard,
                        block: vec![],
                    });
                }
                Token::CloseParen => ret_error!(InvalidToken, Token::CloseParen),
                Token::Eval => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(||unreachable!("eval")){
                        RuntimeValue::Function { patterns, guard, block } => {
                            pending_call = self.execute_function_or_list(RuntimeValue::Function { patterns, guard, block })?;
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_function()], vec![other] ),
                    };
                }
                Token::OpenSquare => {
                    self.data_stack.push(RuntimeValue::Type(RuntimeValueT::List(vec![])));
                }
                Token::CloseSquare => {
                    let mut list = vec![];
                    let mut found_sentinel = false;
                    while let Some(val) = self.data_stack.pop() {
                        if let RuntimeValue::Type(RuntimeValueT::List(ref inner)) = val && inner.is_empty() {
                                found_sentinel = true;
                                break;
                        }
                        list.push(val);
                    }
                    if !found_sentinel {
                        ret_error!(InvalidToken, Token::CloseSquare);
                    }
                    list.reverse();
                    self.data_stack.push(RuntimeValue::List(list));
                }
                Token::When => ret_error!(InvalidToken, Token::When),
                Token::Pipe => ret_error!(InvalidToken, Token::Pipe),
                Token::Fallback => ret_error!(InvalidToken, Token::Fallback),
                Token::RangeOp => ret_error!(InvalidToken, Token::RangeOp),
                Token::TypeLit(t) => ret_error!(InvalidToken, Token::TypeLit(t)),
                Token::TypeOf => {
                    if self.data_stack.len() - frame.frame_pointer < 1 {
                        ret_error!(StackUnderflow)
                    }

                    let t = self.data_stack.pop().unwrap_or_else(|| unreachable!("typeof"));
                    self.data_stack.push(RuntimeValue::Type(t.get_type()));
                }
                Token::Take => {
                    let var_name_opt = frame.next();
                    let var_name: String = match var_name_opt {
                        Some(Token::UnquotedLit(s)) => s.to_string(),
                        other => ret_error!(UnexpectedToken, [UnquotedLit], other),
                    };

                    match self.elements.remove(&var_name){
                        Some(Element::Var(runtime_value) )=> self.data_stack.push(runtime_value),
                        Some(Element::Function { patterns, guard, block }) => {
                            self.data_stack.push(RuntimeValue::Function {
                                patterns,
                                guard,
                                block
                            })
                        }
                        _ => ret_error!(UndeclaredObject { t: "variable", name: var_name })
                    }
                }
                Token::Delete => {
                    let var_name_opt = frame.next();
                    let var_name: String = match var_name_opt {
                        Some(Token::UnquotedLit(s)) => s.to_string(),
                        other => ret_error!(UnexpectedToken, [UnquotedLit], other),
                    };

                    if self.elements.remove(&var_name).is_none(){
                       ret_error!(UndeclaredObject { t: "variable", name: var_name })
                    }
                }
                Token::SysOpen => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    let path = match self.data_stack.pop().unwrap_or_else(|| unreachable!("sysopen")){
                        RuntimeValue::String(s) => s,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_string()], vec![other])
                    };

                    let fd = File::open(std::path::Path::new(path.to_string().as_str()))?;
                    let mut runtime_fd = self.file_index.len();
                    for (i, f) in self.file_index.iter().enumerate(){
                        if let FileDescriptor::Empty = f{
                            runtime_fd = i;
                        }
                    }

                    self.file_index.push(FileDescriptor::DiskFile(fd));
                    self.data_stack.push(RuntimeValue::Int(runtime_fd as i64));

                }
                Token::SysClose => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    let runtime_fd = match self.data_stack.pop().unwrap_or_else(|| unreachable!("sysopen")){
                        RuntimeValue::Int(i) => i,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other])
                    };

                    if self.file_index.len() < runtime_fd as usize{
                        todo!("Return error for invalid file descriptor")
                    }

                    drop(self.file_index.remove(runtime_fd as usize));
                    self.file_index.insert(runtime_fd as usize, FileDescriptor::Empty);
                }
                Token::SysRead => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let num_bytes = match self.data_stack.pop().unwrap_or_else(|| unreachable!("sysopen")){
                        RuntimeValue::Int(i) => i,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other])
                    };

                    let runtime_fd = match self.data_stack.pop().unwrap_or_else(|| unreachable!("sysopen")){
                        RuntimeValue::Int(i) => i,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other])
                    };

                    if self.file_index.len() < runtime_fd as usize{
                        todo!("Return error for invalid file descriptor")
                    }

                    let mut read_bytes = vec![0u8; num_bytes as usize];
                    match &mut self.file_index[runtime_fd as usize]{
                        FileDescriptor::Stdin => {
                            _ = std::io::stdin().read(&mut read_bytes)?;
                        }
                        FileDescriptor::Stdout => todo!("throw bad fd error"),
                        FileDescriptor::Stderr => todo!("throw bad fd error"),
                        FileDescriptor::DiskFile(file) => {
                            _ = file.read(&mut read_bytes)?;
                        }
                        FileDescriptor::Empty => unreachable!("sys-read"),
                    };
                    self.data_stack.push(RuntimeValue::String(Rc::new(unescape_string(str::from_utf8(read_bytes.as_ref())?))));



                }
                Token::SysWrite => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let content = self.data_stack.pop().unwrap_or_else(|| unreachable!("sysopen"));

                    let runtime_fd = match self.data_stack.pop().unwrap_or_else(|| unreachable!("sysopen")){
                        RuntimeValue::Int(i) => i,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other])
                    };

                    if self.file_index.len() < runtime_fd as usize{
                        todo!("Return error for invalid file descriptor")
                    }

                    match &mut self.file_index[runtime_fd as usize]{
                        FileDescriptor::Stdin => todo!("throw bad fd error"),
                        FileDescriptor::Stdout => std::io::stdout().write_all(content.to_string().as_bytes())?,
                        FileDescriptor::Stderr => std::io::stderr().write_all(content.to_string().as_bytes())?,
                        FileDescriptor::DiskFile(file) => file.write_all(content.to_string().as_bytes())?,
                        FileDescriptor::Empty => unreachable!("sys-read"),
                    };
                }
                Token::Concat => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!()){
                        RuntimeValue::List(mut r) => {
                            match self.data_stack.pop().unwrap_or_else(|| unreachable!()){
                                RuntimeValue::List(mut l) => {
                                    l.append(& mut r);
                                    self.data_stack.push(RuntimeValue::List(l));
                                },
                                other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                            };
                        }
                        RuntimeValue::String(r) => {
                            match self.data_stack.pop().unwrap_or_else(|| unreachable!()){
                                RuntimeValue::String(l) => {
                                    self.data_stack.push(RuntimeValue::String(Rc::new(format!("{l}{r}"))));
                                },
                                RuntimeValue::Char(l) => {
                                    self.data_stack.push(RuntimeValue::String(Rc::new(format!("{l}{r}"))));
                                },
                                other => ret_error!(UnexpectedTypes, [_default_runtime_string(), _default_runtime_char()], vec![other] ),
                            };
                        }
                        RuntimeValue::Char(r) => {
                            match self.data_stack.pop().unwrap_or_else(|| unreachable!()){
                                RuntimeValue::String(l) => {
                                    self.data_stack.push(RuntimeValue::String(Rc::new(format!("{l}{r}"))));
                                },
                                RuntimeValue::Char(l) => {
                                    self.data_stack.push(RuntimeValue::String(Rc::new(format!("{l}{r}"))));
                                },
                                other => ret_error!(UnexpectedTypes, [_default_runtime_string(), _default_runtime_char()], vec![other] ),
                            };
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list(), _default_runtime_string(), _default_runtime_char()], vec![other] ),
                    };
                }
                Token::Cons => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let head = self.data_stack.pop().unwrap_or_else(|| unreachable!("cons"));
                    let tail = self.data_stack.pop().unwrap_or_else(|| unreachable!("cons"));

                    match tail {
                        RuntimeValue::List(mut list) => {
                            list.insert(0, head);
                            self.data_stack.push(RuntimeValue::List(list));
                        }
                        RuntimeValue::String(s) => {
                            let mut new_s = match head{
                                RuntimeValue::Char(c) => c,
                                other => ret_error!(UnexpectedTypes, [_default_runtime_char()], vec![other] )
                            }.to_string();
                            new_s.push_str(&s);
                            self.data_stack.push(RuntimeValue::String(Rc::new(new_s)));
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list(), _default_runtime_string()], vec![other] ),
                    }
                }
                Token::Uncon => {
                    if self.data_stack.len() - frame.frame_pointer < 1{
                        ret_error!(StackUnderflow)
                    }

                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("uncon")){
                        RuntimeValue::List(mut list) => {
                            if list.is_empty(){
                                todo!("stack len insufficient") // TODO: improve error handling
                            }
                            let value = list.remove(0);
                            self.data_stack.push(RuntimeValue::List(list));
                            self.data_stack.push(value);
                        }
                        RuntimeValue::String(s) => {
                            if s.is_empty() {
                                todo!("empty string uncon error")
                            }
                            let mut chars = s.chars();
                            let head_char = chars.next().unwrap();
                            let tail_str = chars.collect::<String>();
                            self.data_stack.push(RuntimeValue::String(Rc::new(tail_str)));
                            self.data_stack.push(RuntimeValue::Char(head_char));
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list(), _default_runtime_string()], vec![other] ),
                    };
                }
                Token::At => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let index = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Int(n) => n,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other] ),
                    };
                    let list = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::List(l) => l,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };

                    if index as usize >= list.len(){
                        ret_error!(IndexOutOfRange, index)
                    }

                    self.data_stack.push(list[index as usize].clone());
                }
                Token::Explode => {
                    if self.data_stack.len() - frame.frame_pointer < 1 {
                        ret_error!(StackUnderflow)
                    }

                    let list = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::List(l) => l,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };

                    for e in list {
                        self.data_stack.push(e)
                    }
                }
                Token::Pack => {
                    if self.data_stack.len() - frame.frame_pointer < 2 {
                        ret_error!(StackUnderflow)
                    }

                    let n = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Int(n) => n,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other] ),
                    };

                    let mut list = Vec::<RuntimeValue>::new();
                    for _ in 0..n{
                        match self.data_stack.pop(){
                            Some(v) => list.push(v),
                            None => ret_error!(StackUnderflow)
                        }
                    }

                    list.reverse();
                    self.data_stack.push(RuntimeValue::List(list));
                }
                Token::First => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let n = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Int(n) => n,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other] ),
                    };
                    let mut list = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::List(l) => l,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };

                    list.truncate(n as usize);
                    self.data_stack.push(RuntimeValue::List(list))
                }
                Token::Last => {
                    if self.data_stack.len() - frame.frame_pointer < 2{
                        ret_error!(StackUnderflow)
                    }

                    let n = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Int(n) => n,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other] ),
                    };
                    let mut list = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::List(l) => l,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };

                    self.data_stack.push(RuntimeValue::List(list.split_off(n as usize)))
                }
                Token::SubStrB => {
                    if self.data_stack.len() - frame.frame_pointer < 3{
                        ret_error!(StackUnderflow)
                    }
                    let idx = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Int(n) => n,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other] ),
                    };
                    let length = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Int(n) => n,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_int()], vec![other] ),
                    };
                    let rt_str = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::String(s) => s,
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };

                    if (idx + length) as usize > rt_str.len(){
                        ret_error!(IndexOutOfRange, idx + length)
                    }

                    self.data_stack.push(RuntimeValue::String(Rc::new(rt_str.as_str()[idx as usize..(idx + length) as usize].to_string())));
                }
                Token::FindB => {
                    match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                        RuntimeValue::Char(c) => {
                            let rt_str = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                                RuntimeValue::String(s) => s,
                                other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                            };
                            if let Some(n) = rt_str.find(c){
                                self.data_stack.push(RuntimeValue::Int(n as i64));
                                self.data_stack.push(RuntimeValue::Bool(true));
                            } else {
                                self.data_stack.push(RuntimeValue::Bool(false));
                            }
                        }
                        RuntimeValue::String(s) => {
                            let rt_str = match self.data_stack.pop().unwrap_or_else(|| unreachable!("at")){
                                RuntimeValue::String(s) => s,
                                other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                            };
                            if let Some(n) = rt_str.find(&*s){
                                self.data_stack.push(RuntimeValue::Int(n as i64));
                                self.data_stack.push(RuntimeValue::Bool(true));
                            } else {
                                self.data_stack.push(RuntimeValue::Bool(false));
                            }
                        }
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    }

                }
                Token::Match => {
                    if self.data_stack.len() - frame.frame_pointer < 1 {
                        ret_error!(StackUnderflow)
                    }
                    let f: crate::RuntimeValue= match self.data_stack.pop().unwrap_or_else(|| unreachable!("match")){
                        RuntimeValue::List(l) => RuntimeValue::List(l),
                        other => ret_error!(UnexpectedTypes, [_default_runtime_list()], vec![other] ),
                    };

                    pending_call = self.execute_function_or_list(f)?;
                }
            }
            self.call_stack.push(frame);

            if let Some(new_frame) = pending_call {
                self.call_stack.push(new_frame);
            }

            Ok(Flow::Next)
            }
    pub fn try_match_function(&mut self, patterns: &[Pattern], guard: &Option<Vec<Token>>) ->  PatternResult{
        let variadic_pos = patterns.iter().position(|t| matches!(t, Pattern::Variadic(_)));
        let fp;

        if let Some(v_idx) = variadic_pos {
            let fixed_after = patterns.len() - 1 - v_idx;
            let fixed_before = v_idx;

            if self.data_stack.len() < fixed_before + fixed_after {
                return Ok(None);
            }

            let end_v = self.data_stack.len() - fixed_after;
            let mut start_v = end_v;

            let inner_pattern = if let Pattern::Variadic(inner_box) = &patterns[v_idx] {
                *inner_box.clone()
            } else {
                unreachable!()
            };

            while start_v > fixed_before {
                if inner_pattern.check(&self.data_stack[start_v - 1]) {
                    start_v -= 1;
                } else {
                    break;
                }
            }

            fp = start_v - fixed_before;
        } else {
            if self.data_stack.len() < patterns.len() {
                return Ok(None);
            }
            fp = self.data_stack.len() - patterns.len();
        }

        for i in fp..self.data_stack.len() {
            let k = i - fp;
            let expected_pat = match variadic_pos {
                Some(v_idx) => {
                    let variadic_count = self.data_stack.len() - fp - (patterns.len() - 1);
                    if k < v_idx {
                        patterns[k].clone()
                    } else if k >= v_idx && k < v_idx + variadic_count {
                        if let Pattern::Variadic(inner_box) = &patterns[v_idx] {
                            *inner_box.clone()
                        } else {
                            unreachable!()
                        }
                    } else {
                        patterns[k - variadic_count + 1].clone()
                    }
                }
                None => patterns[k].clone()
            };

            if !expected_pat.check(&self.data_stack[i]) {
                return Ok(None);
            }
        }

        let mut new_args = vec![];
        for i in fp..self.data_stack.len() {
            let k = i - fp;
            let expected_pat = match variadic_pos {
                Some(v_idx) => {
                    let variadic_count = self.data_stack.len() - fp - (patterns.len() - 1);
                    if k < v_idx {
                        patterns[k].clone()
                    } else if k >= v_idx && k < v_idx + variadic_count {
                        if let Pattern::Variadic(inner_box) = &patterns[v_idx] {
                            *inner_box.clone()
                        } else {
                            unreachable!()
                        }
                    } else {
                        patterns[k - variadic_count + 1].clone()
                    }
                }
                None => patterns[k].clone()
            };

            fn expand_pat(p: &Pattern, v: &RuntimeValue) -> Vec<RuntimeValue> {
                match p {
                    Pattern::List(pat_list) => {
                        if let RuntimeValue::List(val_list) = v {
                            // Lenient case: if pattern is a single type, push the list itself
                            if pat_list.len() == 1 && matches!(pat_list[0], Pattern::Type(_)) && val_list.len() != 1 {
                                return vec![v.clone()];
                            }

                            let mut res = vec![];
                            let variadic_pos = pat_list.iter().position(|p| matches!(p, Pattern::Variadic(_)));
                            if let Some(v_idx) = variadic_pos {
                                let fixed_before = v_idx;
                                let fixed_after = pat_list.len() - 1 - v_idx;
                                let val_after_start = val_list.len() - fixed_after;

                                for i in 0..fixed_before {
                                    res.extend(expand_pat(&pat_list[i], &val_list[i]));
                                }

                                // Group variadic elements into a list
                                let variadic_elements = val_list[fixed_before..val_after_start].to_vec();
                                res.push(RuntimeValue::List(variadic_elements));

                                for i in 0..fixed_after {
                                    res.extend(expand_pat(&pat_list[v_idx + 1 + i], &val_list[val_after_start + i]));
                                }
                            } else {
                                for (p_item, v_item) in pat_list.iter().zip(val_list.iter()) {
                                    res.extend(expand_pat(p_item, v_item));
                                }
                            }
                            res
                        } else if let RuntimeValue::String(s) = v {
                            if pat_list.len() == 1 {
                                return expand_pat(&pat_list[0], v);
                            }
                            let mut res = vec![];
                            let chars: Vec<char> = s.chars().collect();
                            for (p_item, c_item) in pat_list.iter().zip(chars.iter()) {
                                res.extend(expand_pat(p_item, &RuntimeValue::Char(*c_item)));
                            }
                            res
                        } else {
                            vec![v.clone()]
                        }
                    }
                    Pattern::Destructure(head_pat, tail_pat) => {
                        if let RuntimeValue::List(val_list) = v {
                            if val_list.is_empty() { return vec![]; }
                            let head_val = &val_list[0];
                            let tail_val = RuntimeValue::List(val_list[1..].to_vec());
                            let mut res = expand_pat(head_pat, head_val);
                            res.extend(expand_pat(tail_pat, &tail_val));
                            res
                        } else if let RuntimeValue::String(s) = v {
                            let mut chars = s.chars();
                            if let Some(head_char) = chars.next() {
                                let tail_str = chars.collect::<String>();
                                let head_val = RuntimeValue::Char(head_char);
                                let tail_val = RuntimeValue::String(Rc::new(tail_str));
                                let mut res = expand_pat(head_pat, &head_val);
                                res.extend(expand_pat(tail_pat, &tail_val));
                                res
                            } else {
                                vec![v.clone()]
                            }
                        } else {
                            vec![v.clone()]
                        }
                    }
                    _ => vec![v.clone()]
                }
            }
            new_args.extend(expand_pat(&expected_pat, &self.data_stack[i]));
        }

        let mut new_stack = self.data_stack[0..fp].to_vec();
        new_stack.extend(new_args);

        if let Some(guard_block) = guard {
            let original_stack = std::mem::replace(&mut self.data_stack, new_stack.clone());
            let target_depth = self.call_stack.len();

            self.call_stack.push(CallFrame {
                instructions: guard_block.clone(),
                ip: 0,
                frame_pointer: fp
            });

            while self.call_stack.len() > target_depth {
                if let Flow::Return = self.interpret_step()? {
                    break;
                }
            }

            let mut guard_passed = false;
            if let Some(RuntimeValue::Bool(b)) = self.data_stack.pop() {
                guard_passed = b;
            }

            self.data_stack = original_stack;

            if !guard_passed {
                return Ok(None);
            }
        }

        Ok(Some((fp, new_stack)))
    }

    pub fn execute_function_or_list(&mut self, val: RuntimeValue) -> Result<Option<CallFrame>, Box<dyn Error>> {
        match val {
            RuntimeValue::Function { patterns, guard, block } => {
                if let Some((fp, new_stack)) = self.try_match_function(&patterns, &guard)? {
                    self.data_stack = new_stack;
                    Ok(Some(CallFrame {
                        instructions: block,
                        ip: 0,
                        frame_pointer: fp
                    }))
                } else {
                    Ok(None)
                }
            }
            RuntimeValue::List(list) => {
                for item in list {
                    if let RuntimeValue::Function { patterns, guard, block } = item {
                        if let Some((fp, new_stack)) = self.try_match_function(&patterns, &guard)? {
                            self.data_stack = new_stack;
                            return Ok(Some(CallFrame {
                                instructions: block,
                                ip: 0,
                                frame_pointer: fp
                            }));
                        }
                    } else {
                        ret_error!(UnexpectedTypes, [_default_runtime_function()], vec![item])
                    }
                }
                Ok(None)
            }
            other => ret_error!(UnexpectedTypes, [_default_runtime_function()], vec![other])
        }
    }

}


fn expect_open_curly(nt: Option<&Token>) -> Result<(), Box<dyn Error>>{
    match nt {
        Some(next_token) => {
            if *next_token != Token::OpenCurly {
                ret_error!(UnexpectedToken, [OpenCurly], Some(next_token))
            } else {
                Ok(())
            }
        }
        None => {
            ret_error!(UnexpectedToken, [OpenCurly], None::<Token>)
        }
    }
}

fn collect_tokens_into_block(frame: &mut CallFrame) -> Result<Vec<Token>, Box<dyn Error>>{
    let mut block_vector: Vec<Token> = vec![];
    let mut brace_depth = 1;

    while frame.ip < frame.instructions.len() {
        let tk = frame.instructions[frame.ip].clone();
        frame.ip += 1;
        match tk {
            Token::OpenCurly => {
                brace_depth += 1;
                block_vector.push(tk);
            }
            Token::CloseCurly => {
                brace_depth -= 1;
                if brace_depth == 0 {
                    break;
                } else {
                    block_vector.push(tk);
                }
            }
            _ => {
                block_vector.push(tk);
            }
        }
    };
    if brace_depth != 0 {
        // Replace this with however your VM handles syntax errors (e.g., ret_error!)
        return Err("Syntax Error: Reached end of file but missing a closing '}'".to_string().into());
    }
    Ok(block_vector)
}

fn parse_single_pattern(frame: &mut CallFrame, first_tk: Token) -> Result<Pattern, Box<dyn Error>> {
    match first_tk {
        Token::TypeLit(RuntimeValueT::Variadic(t)) => Ok(Pattern::Variadic(Box::new(Pattern::Type(*t)))),
        Token::TypeLit(t) => Ok(Pattern::Type(t)),
        Token::NumberLit(n) => {
            if frame.peek() == Some(&Token::RangeOp) {
                frame.next();
                if let Some(Token::NumberLit(end)) = frame.next() {
                    Ok(Pattern::Range { start: n, end: *end, inclusive: false })
                } else {
                    ret_error!(UnknownType, "Expected end of range".to_string())
                }
            } else {
                Ok(Pattern::Literal(RuntimeValue::Int(n)))
            }
        }
        Token::QuotedLit(s) => {
            let trimmed = s.trim_matches('"').to_string();
            Ok(Pattern::Literal(RuntimeValue::String(Rc::new(unescape_string(&trimmed)))))
        }
        Token::BoolLit(b) => Ok(Pattern::Literal(RuntimeValue::Bool(b))),
        Token::Fallback => Ok(Pattern::Fallback),
        Token::OpenSquare => {
            let mut list = vec![];
            let mut destructure = None;
            while frame.ip < frame.instructions.len() {
                let tk = frame.instructions[frame.ip].clone();
                frame.ip += 1;
                match tk {
                    Token::CloseSquare => break,
                    Token::CloseParen => break,
                    Token::Pipe => {
                        if frame.ip < frame.instructions.len() {
                            let next_tk = frame.instructions[frame.ip].clone();
                            frame.ip += 1;
                            let tail = parse_single_pattern(frame, next_tk)?;
                            let mut current = tail;
                            while let Some(head) = list.pop() {
                                current = Pattern::Destructure(Box::new(head), Box::new(current));
                            }
                            destructure = Some(current);
                        }
                    }
                    _ => list.push(parse_single_pattern(frame, tk)?),
                }
            }
            if let Some(d) = destructure {
                Ok(d)
            } else {
                Ok(Pattern::List(list))
            }
        }
        other => ret_error!(UnknownType, format!("{:?}", other))
    }
}

fn parse_patterns(frame: &mut CallFrame) -> Result<Vec<Pattern>, Box<dyn Error>>{
    let mut patterns: Vec<Pattern> = vec![];

    while frame.ip < frame.instructions.len() {
        let tk = frame.instructions[frame.ip].clone();
        frame.ip += 1;
        match tk {
            Token::CloseParen => {
                break;
            }
            other => {
                patterns.push(parse_single_pattern(frame, other)?);
            }
        }
    };
    Ok(patterns)
}

fn unescape_string(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next_c) = chars.next() {
                match next_c {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '0' => result.push('\0'),
                    _ => {
                        result.push('\\');
                        result.push(next_c);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(c);
        }
    }

    result
}
