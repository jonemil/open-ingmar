//! Strict source entries for tests that require every type alternative to fit.
//! IR validation remains gradual; malformed-IR tests must use disjoint contracts.
#![allow(dead_code)]

use ciq_compiler::{CompilationUnit, CompileError, CompileOptions, Function, TypeCheckLevel};

fn options() -> CompileOptions {
    CompileOptions {
        type_check: TypeCheckLevel::Strict,
        ..Default::default()
    }
}

/// Compile one function with ambiguity treated as an error.
pub fn compile_function(source: &str) -> Result<Function, CompileError> {
    ciq_compiler::compile_function_with_options(source, options()).map(|output| output.artifact)
}

/// Compile source files with ambiguity treated as an error.
pub fn compile_sources(sources: &[&str]) -> Result<CompilationUnit, CompileError> {
    ciq_compiler::compile_sources_with_options(sources, options()).map(|output| output.artifact)
}
