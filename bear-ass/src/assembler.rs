use crate::parser::ast;
use crate::processor;

#[derive(Debug)]
pub enum Error {
    Unknown,
    ExpressionCannotBeSimplified(ast::Expression),
}

#[derive(Default)]
pub struct ImageBuilder {
    bits: Vec<u8>,
}

impl ImageBuilder {
    fn assemble_u8(&mut self, value: u8) {
        self.bits.push(value);
    }

    fn assemble_u16(&mut self, value: u16) {
        self.bits.extend(&value.to_le_bytes());
    }

    fn assemble_u32(&mut self, value: u32) {
        self.bits.extend(&value.to_le_bytes());
    }

    fn assemble_string(&mut self, value: String) {
        self.bits.extend(value.as_bytes());
    }
}

#[derive(Default)]
pub struct Assembler {}

impl Assembler {
    pub fn assemble(p: processor::Processor) -> Result<Vec<u8>, Error> {
        let ass = Assembler {};
        let mut bin = ImageBuilder::default();

        for proc in p.processed.iter() {
            if bin.bits.len() < proc.address {
                bin.bits.resize(proc.address, 0);
            }
            if bin.bits.len() != proc.address {
                panic!("stream malformed: {}, {:?}", bin.bits.len(), proc);
            }
            match &proc.body {
                ast::LineBody::Data(data) => ass.assemble_data(data.clone(), &mut bin)?,
                ast::LineBody::Simple(op) => bin.assemble_u8(op.into_u8()),
                // By this point all of the preprocessor directives should have been handled.
                // If a preprocessor directive is encountered, then something has gone wrong.
                ast::LineBody::Directive(dir) => {
                    panic!("Preprocessor error; encountered directive: {:?}", dir)
                }
                body => panic!("Assembler encountered '{:?}'.", body),
            }
        }

        // The output is padded to a multiple of 4.
        while bin.bits.len() % 4 != 0 {
            bin.assemble_u8(0);
        }

        return Ok(bin.bits);
    }

    fn assemble_data(&self, data: ast::Data, bin: &mut ImageBuilder) -> Result<(), Error> {
        Ok(match data {
            ast::Data::D(size, expr) => {
                if let Some(p) = expr.as_primitive() {
                    match size {
                        ast::Size::S8 => bin.assemble_u8(p.assemble_8().unwrap()),
                        ast::Size::S16 => bin.assemble_u16(p.assemble_16().unwrap()),
                        ast::Size::S32 => bin.assemble_u32(p.assemble_32().unwrap()),
                    }
                } else {
                    // Expressions must evaluate to values at compile time.
                    eprintln!("Expression cannot be simplified: {:?}", expr);
                    return Err(Error::ExpressionCannotBeSimplified(expr));
                }
            }
            ast::Data::Str(ast::StringTag::R, text) => {
                bin.assemble_string(text);
            }
            ast::Data::Str(ast::StringTag::C, text) => {
                bin.assemble_string(text);
                bin.assemble_u8(0);
            }
            ast::Data::Str(ast::StringTag::S, text) => {
                bin.assemble_u32(text.as_bytes().len() as u32);
                bin.assemble_string(text);
            }
        })
    }
}
