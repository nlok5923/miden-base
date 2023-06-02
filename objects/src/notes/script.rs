use miden_core::code_blocks::CodeBlock;

use super::{Assembler, AssemblyContext, Digest, NoteError, ProgramAst};

#[derive(Debug, Clone)]
pub struct NoteScript {
    code_block: CodeBlock,
    code: ProgramAst,
}

impl NoteScript {
    pub fn new(
        code: ProgramAst,
        assembler: &mut Assembler,
        context: &mut AssemblyContext,
    ) -> Result<Self, NoteError> {
        let code_block = assembler
            .compile_in_context(code.clone(), context)
            .map_err(NoteError::ScriptCompilationError)?;
        Ok(Self { code_block, code })
    }

    pub fn hash(&self) -> Digest {
        self.code_block.hash()
    }

    pub fn code_block(&self) -> &CodeBlock {
        &self.code_block
    }

    pub fn code(&self) -> &ProgramAst {
        &self.code
    }
}
