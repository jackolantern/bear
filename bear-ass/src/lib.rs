pub mod assembler;
pub mod parser;
pub mod processor;

extern crate bear_vm;

#[derive(Debug)]
pub enum Error {
    Usage,
    Unknown(String),
    IOError(std::io::Error),
    ParserError(parser::Error),
    SerdeError(serde_json::Error),
    AssemblerError(assembler::Error),
}
