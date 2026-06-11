use crate::error::LexError;
use crate::value::{RuntimeValueT, get_type_from_str};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Push,
    Drop,
    Clear,
    SysOpen,
    SysClose,
    SysRead,
    SysWrite,
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Dup,
    StackLen,
    Len,
    Into,
    Take,
    Delete,
    ToInt,
    ToString,
    ToBool,
    ToChar,
    Concat,
    Cons,
    Uncon,
    At,
    Explode,
    Pack,
    First,
    Last,
    FindB,
    SubStrB,
    Swap,
    Rot,
    Over,
    Roll,
    Pick,
    Eq,
    Gt,
    Lt,
    OpenCurly,
    CloseCurly,
    OpenParen,
    CloseParen,
    OpenSquare,
    CloseSquare,
    If,
    Else,
    Match,
    And,
    Or,
    Not,
    When,
    Pipe,
    Arrow,
    Fallback,
    RangeOp,
    ElementCall(String),
    Eval,
    TypeLit(RuntimeValueT),
    TypeOf,
    BoolLit(bool),
    QuotedLit(String),
    UnquotedLit(String),
    NumberLit(i64),
    CharLit(char),
    Quit,
    Ret,
    Include,
}

impl Token {
    #[allow(dead_code)]
    pub fn type_name(&self) -> &'static str {
        match self {
            Token::Push => "Push",
            Token::Drop => "Drop",
            Token::Clear => "Clear",
            Token::Add => "Add",
            Token::Sub => "Sub",
            Token::Mul => "Mul",
            Token::Div => "Div",
            Token::Neg => "Neg",
            Token::Dup => "Dup",
            Token::Len => "Len",
            Token::Into => "Into",
            Token::ToInt => "ToInt",
            Token::ToChar => "ToChar",
            Token::Swap => "Swap",
            Token::Rot => "Rot",
            Token::Over => "Over",
            Token::Roll => "Roll",
            Token::Pick => "Pick",
            Token::Eq => "Eq",
            Token::Gt => "Gt",
            Token::Lt => "Lt",
            Token::OpenCurly => "OpenCurly",
            Token::CloseCurly => "CloseCurly",
            Token::OpenParen => "OpenParen",
            Token::CloseParen => "CloseParen",
            Token::If => "If",
            Token::Else => "Else",
            Token::And => "And",
            Token::Or => "Or",
            Token::Not => "Not",
            Token::When => "When",
            Token::Pipe => "Pipe",
            Token::Arrow => "Arrow",
            Token::Fallback => "Fallback",
            Token::RangeOp => "RangeOp",
            Token::ElementCall(_) => "ElementCall",
            Token::Eval => "Eval",
            Token::BoolLit(_) => "BoolLit",
            Token::QuotedLit(_) => "QuotedLit",
            Token::UnquotedLit(_) => "UnquotedLit",
            Token::NumberLit(_) => "NumberLit",
            Token::Quit => "Quit",
            Token::Ret => "Ret",
            Token::Include => "Include",
            Token::OpenSquare => "OpenSquare",
            Token::CloseSquare => "CloseSquare",
            Token::TypeLit(_) => "TypeLit",
            Token::TypeOf => "TypeOf",
            Token::Take => "Take",
            Token::Delete => "Delete",
            Token::SysOpen => "SysOpen",
            Token::SysClose => "SysClose",
            Token::SysRead => "SysRead",
            Token::SysWrite => "SysWrite",
            Token::Concat => "Concat",
            Token::Cons => "Cons",
            Token::Uncon => "Uncon",
            Token::At => "At",
            Token::StackLen => "StackLen",
            Token::Explode => "Explode",
            Token::Pack => "Pack",
            Token::First => "First",
            Token::Last => "Last",
            Token::ToString => "ToString",
            Token::ToBool => "ToBoll",
            Token::FindB => "FindB",
            Token::SubStrB => "SubStrB",
            Token::CharLit(_) => "CharLit",
            Token::Match => "Match",
        }
    }
}

pub fn tokenize(content: String) -> Vec<Token> {
    try_tokenize(content).expect("tokenization failed")
}

pub fn try_tokenize(content: String) -> Result<Vec<Token>, LexError> {
    let mut tokens = Vec::new();
    let mut chars = content.chars().peekable();

    while let Some(&c) = chars.peek() {
        match c {
            _ if c.is_whitespace() => {
                chars.next();
            }
            '{' => {
                tokens.push(Token::OpenCurly);
                chars.next();
            }
            '}' => {
                tokens.push(Token::CloseCurly);
                chars.next();
            }
            '(' => {
                tokens.push(Token::OpenParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::CloseParen);
                chars.next();
            }
            '[' => {
                tokens.push(Token::OpenSquare);
                chars.next();
            }
            ']' => {
                tokens.push(Token::CloseSquare);
                chars.next();
            }
            ';' => {
                chars.next();
                if chars.peek() == Some(&';') {
                    chars.next();
                    let mut last_was_semi = false;
                    for cc in &mut chars {
                        if cc == '\n' {
                            break;
                        }
                        if cc == ';' {
                            if last_was_semi {
                                break;
                            }
                            last_was_semi = true;
                        } else {
                            last_was_semi = false;
                        }
                    }
                }
            }
            '"' => {
                chars.next();
                let mut string_val = String::new();
                for cc in &mut chars {
                    if cc == '"' {
                        break;
                    }
                    string_val.push(cc);
                }
                tokens.push(Token::QuotedLit(string_val));
            }
            '\'' => {
                chars.next();
                match chars.next() {
                    Some(c) => {
                        if chars.next().is_some_and(|nc| nc == '\'') {
                            tokens.push(Token::CharLit(c));
                        } else {
                            return Err(LexError::InvalidCharLiteral { line: 0, col: 0 });
                        }
                    }
                    None => return Err(LexError::InvalidCharLiteral { line: 0, col: 0 }),
                }
            }
            '#' => {
                chars.next();
                let mut elm = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_whitespace()
                        || nc == '{'
                        || nc == '}'
                        || nc == '"'
                        || nc == ';'
                        || nc == '('
                        || nc == ')'
                    {
                        break;
                    }
                    elm.push(nc);
                    chars.next();
                }
                tokens.push(Token::ElementCall(elm));
            }
            '@' => {
                chars.next();
                if chars.peek().is_some_and(|c| c.is_whitespace()) {
                    tokens.push(Token::TypeOf);
                    chars.next();
                } else {
                    let mut t = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc.is_whitespace()
                            || nc == '{'
                            || nc == '}'
                            || nc == '"'
                            || nc == ';'
                            || nc == '('
                            || nc == ')'
                            || nc == '['
                            || nc == ']'
                        {
                            break;
                        }
                        t.push(nc);
                        chars.next();
                    }
                    let rt_t = get_type_from_str(t.as_str());
                    tokens.push(Token::TypeLit(rt_t));
                }
            }
            '.' => {
                chars.next();
                if chars.peek().is_some_and(|c| *c == '.') {
                    chars.next();
                    if chars.peek().is_some_and(|c| *c == '@') {
                        chars.next();
                        let mut t = String::new();
                        while let Some(&nc) = chars.peek() {
                            if nc.is_whitespace()
                                || nc == '{'
                                || nc == '}'
                                || nc == '"'
                                || nc == ';'
                                || nc == '('
                                || nc == ')'
                                || nc == '['
                                || nc == ']'
                            {
                                break;
                            }
                            t.push(nc);
                            chars.next();
                        }
                        let rt_t = get_type_from_str(t.as_str());
                        tokens.push(Token::TypeLit(RuntimeValueT::Variadic(Box::new(rt_t))));
                    } else if chars.peek().is_some_and(|c| *c == '<') {
                        chars.next();
                        tokens.push(Token::RangeOp);
                    } else {
                        tokens.push(Token::Fallback);
                    }
                }
            }
            _ => {
                let mut word = String::new();

                while let Some(&nc) = chars.peek() {
                    if nc.is_whitespace()
                        || nc == '{'
                        || nc == '}'
                        || nc == '"'
                        || nc == ';'
                        || nc == '('
                        || nc == ')'
                        || nc == '['
                        || nc == ']'
                        || nc == '.'
                    {
                        break;
                    }
                    word.push(nc);
                    chars.next();
                }

                let token = match word.as_str() {
                    "push" => Token::Push,
                    "drop" => Token::Drop,
                    "clear" => Token::Clear,
                    "add" => Token::Add,
                    "sub" => Token::Sub,
                    "mul" => Token::Mul,
                    "div" => Token::Div,
                    "neg" => Token::Neg,
                    "dup" => Token::Dup,
                    "into" => Token::Into,
                    "take" => Token::Take,
                    "delete" => Token::Delete,
                    "int?" => Token::ToInt,
                    "char?" => Token::ToChar,
                    "string?" => Token::ToString,
                    "bool?" => Token::ToBool,
                    "swap" => Token::Swap,
                    "len" => Token::Len,
                    "stack-len" => Token::StackLen,
                    "rot" => Token::Rot,
                    "over" => Token::Over,
                    "roll" => Token::Roll,
                    "pick" => Token::Pick,
                    "find?" => Token::FindB,
                    "substr?" => Token::SubStrB,
                    "eq" => Token::Eq,
                    "lt" => Token::Lt,
                    "gt" => Token::Gt,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "when" => Token::When,
                    "|" => Token::Pipe,
                    "->" => Token::Arrow,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    "true" => Token::BoolLit(true),
                    "false" => Token::BoolLit(false),
                    "quit" => Token::Quit,
                    "eval" => Token::Eval,
                    "ret" => Token::Ret,
                    "include" => Token::Include,
                    "sys-open" => Token::SysOpen,
                    "sys-close" => Token::SysClose,
                    "sys-read" => Token::SysRead,
                    "sys-write" => Token::SysWrite,
                    "concat" => Token::Concat,
                    "cons" => Token::Cons,
                    "uncon" => Token::Uncon,
                    "at" => Token::At,
                    "explode" => Token::Explode,
                    "pack" => Token::Pack,
                    "first" => Token::First,
                    "last" => Token::Last,
                    "match" => Token::Match,
                    "call" => {
                        while let Some(&wc) = chars.peek() {
                            if wc.is_whitespace() {
                                chars.next();
                            } else {
                                break;
                            }
                        }

                        let mut fun_name = String::new();
                        while let Some(&nc) = chars.peek() {
                            if nc.is_whitespace() || nc == '{' || nc == '}' || nc == ']' {
                                break;
                            }
                            fun_name.push(nc);
                            chars.next();
                        }

                        Token::ElementCall(fun_name)
                    }

                    _ => {
                        if let Ok(num) = word.parse::<i64>() {
                            Token::NumberLit(num)
                        } else {
                            Token::UnquotedLit(word)
                        }
                    }
                };
                tokens.push(token);
            }
        }
    }

    Ok(tokens)
}
