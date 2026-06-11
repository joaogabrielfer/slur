#![allow(dead_code)]

use crate::value::RuntimeValue;

pub type NativeResult = Result<(), String>;
pub type NativeFn = fn(&mut Vec<RuntimeValue>) -> NativeResult;

pub struct NativeEntry {
    pub name: &'static str,
    pub function: NativeFn,
}

pub fn registry() -> Vec<NativeEntry> {
    Vec::new()
}
