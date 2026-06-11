pub mod native;

use crate::compiler::{BytecodeVm, Constant, PbcFile};

pub type Pvm = BytecodeVm;

pub fn run_pbc_file(file: PbcFile) -> Result<Pvm, String> {
    let (constants, bytecode) = file.parts()?;
    let mut pvm = Pvm::new(constants);
    pvm.execute_chunk(&bytecode, 0, None)?;
    Ok(pvm)
}

#[allow(dead_code)]
pub fn run_pbc_parts(constants: Vec<Constant>, bytecode: Vec<u8>) -> Result<Pvm, String> {
    let mut pvm = Pvm::new(constants);
    pvm.execute_chunk(&bytecode, 0, None)?;
    Ok(pvm)
}
