use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::parser::ast;

/// This exists to make the code more readable.  It cannot be changed.
const WORD_SIZE: usize = std::mem::size_of::<u32>();

#[derive(Debug)]
pub enum ErrorTag {
    Unknown,
    IOError(std::io::Error),
    ParserError(crate::parser::Error),

    NextMarkNotSet,
    PreviousMarkNotSet,
    UnknownLabel(String),
    LabelAlreadyDefined(String),

    ExpectedList,
    ExpectedExpression,

    ExpressionCannotBeSimplified(ast::Expression),

    UnknownDefinition(String),
    DefinitionAlreadyDefined(String),

    CannotAtToBeforeCurrentPosition,

    DataSizeMismatch { expected: u8, actual: u8 },
}

impl ErrorTag {
    fn to_error(self) -> Error {
        Error { tags: vec![self] }
    }
}

#[derive(Debug)]
pub struct Error {
    tags: Vec<ErrorTag>,
}

#[derive(Debug, Clone)]
enum Definition {
    DefExpr(ast::Expression),
    DefList(Vec<ast::LineBody>),
}

/// Files which have been included via a preprocessor directive.
#[derive(Default)]
struct Includes {
    files: HashMap<PathBuf, ast::Program>,
    // TODO: Give an error for circular references.
    // references: HashMap<PathBuf, HashSet<PathBuf>>
}

impl Includes {
    // TODO: Errors
    fn parse(&mut self, path: &Path) -> Result<ast::Program, ErrorTag> {
        let contents = std::fs::read_to_string(path).map_err(|e| ErrorTag::IOError(e))?;
        return crate::parser::Parser {}
            .parse(&contents)
            .map_err(|e| ErrorTag::ParserError(e));
    }

    fn include_file(&mut self, path: &Path) -> Result<ast::Program, ErrorTag> {
        let full = path.canonicalize().unwrap();
        if !self.files.contains_key(&full) {
            let program = self.parse(&full)?;
            self.files.insert(full.clone(), program);
        }
        return Ok(self.files.get(&full).cloned().unwrap());
    }
}

#[derive(Debug)]
pub struct ProcessedLine {
    /// The content of the line.
    pub body: ast::LineBody,
    /// The computed address in the binary of the instruction encoded by the line.
    pub address: ast::LineAddress,
}

impl ProcessedLine {
    fn new(body: ast::LineBody, address: ast::LineAddress) -> ProcessedLine {
        ProcessedLine { body, address }
    }
}

/// A `Processor` consumes a `Program` and is converted into a binary by an `Assembler`.
#[derive(Default)]
pub struct Processor {
    position: usize,

    marks: Vec<usize>,
    /// Maps label names to addresses.
    labels: HashMap<String, usize>,
    /// Maps names to definitions.
    definitions: HashMap<String, Definition>,
    // TODO: Handle included files.
    /** Maps addresses in the binary to lines in the source code.
     * Primarily used to generate debugging info.
     */
    addresses: HashMap<ast::LineAddress, ast::LineNumber>,
    includes: Includes,

    original: ast::Program,
    pub processed: Vec<ProcessedLine>,
}

impl Processor {
    fn add_mark(&mut self, position: usize) {
        self.marks.push(position);
    }

    fn resolve_prev(&self) -> Result<usize, ErrorTag> {
        match self.marks.binary_search(&self.position) {
            Ok(position) => Ok(self.marks[position]),
            Err(position) => {
                if position == 0 {
                    return Err(ErrorTag::PreviousMarkNotSet);
                }
                self.marks
                    .get(position - 1)
                    .cloned()
                    .ok_or(ErrorTag::PreviousMarkNotSet)
            }
        }
    }

    fn resolve_next(&self, position: usize) -> Option<usize> {
        match self.marks.binary_search(&position) {
            Ok(p) => Some(self.marks[p]),
            Err(p) => self.marks.get(p).cloned(),
        }
    }

    fn resolve_label(&self, label: &str) -> Option<usize> {
        self.labels.get(label).cloned()
    }

    fn resolve_definition(&self, name: &str) -> Option<Definition> {
        self.definitions.get(name).cloned()
    }
}

impl Processor {
    fn expect_definition_list(&self, name: &str) -> Result<Vec<ast::LineBody>, ErrorTag> {
        match self.resolve_definition(&name) {
            Some(Definition::DefList(list)) => Ok(list),
            Some(_) => Err(ErrorTag::ExpectedList),
            None => Err(ErrorTag::UnknownDefinition(name.into())),
        }
    }

    fn expect_definition_expression(&self, name: &str) -> Result<ast::Expression, ErrorTag> {
        match self.resolve_definition(&name) {
            Some(Definition::DefExpr(expr)) => Ok(expr),
            Some(_) => Err(ErrorTag::ExpectedExpression),
            None => Err(ErrorTag::UnknownDefinition(name.into())),
        }
    }
}

impl Processor {
    pub fn make_debug(&self) -> Result<ast::Debug, Error> {
        let mut body = Vec::new();
        for line in self.original.body.iter() {
            body.push(match &line.body {
                ast::LineBody::Data(x) => ast::DebugLine {
                    content: x.to_string(),
                    tag: ast::DebugTag::Data,
                },
                ast::LineBody::Simple(x) => ast::DebugLine {
                    content: x.to_string(),
                    tag: ast::DebugTag::Instruction,
                },
                ast::LineBody::Directive(x) => ast::DebugLine {
                    content: x.to_string(),
                    tag: ast::DebugTag::Directive,
                },
                ast::LineBody::DefinitionRef(x) => ast::DebugLine {
                    content: x.to_string(),
                    tag: ast::DebugTag::Macro,
                },
            })
        }
        let mut entries = Vec::new();
        let mut rev: HashMap<usize, Vec<String>> = HashMap::new();
        for (label, address) in &self.labels {
            let names = rev.entry(*address).or_insert(Vec::new());
            names.push(label.clone());
        }
        for item in &self.addresses {
            let empty = &Vec::new();
            let names = rev.get(item.0).unwrap_or(empty);
            entries.push(ast::DebugEntry {
                address: *item.0,
                line: *item.1,
                names: names.to_vec(),
            });
        }
        entries.sort_by_key(|e| e.address);
        return Ok(ast::Debug { entries, body });
    }
}

impl Processor {
    fn align_to(&mut self, boundary: usize) -> usize {
        let padding = boundary - (self.position % boundary);
        if padding != boundary {
            self.position += padding;
        }
        self.position
    }
}

impl Processor {
    pub fn process(program: ast::Program) -> Result<Processor, Error> {
        let mut lines = Vec::new();
        let mut preproc = Processor::default();
        let mut is_error = false;
        let mut errors = Error { tags: Vec::new() };
        preproc.original = program.clone();
        for line in program.body.into_iter() {
            preproc.addresses.insert(preproc.position, line.number);
            match preproc.process_line(line) {
                Err(error) => {
                    errors.tags.push(error);
                    is_error = true;
                }
                Ok(newlines) => lines.extend(newlines),
            }
        }
        // TODO: Aggregate errors.
        if is_error {
            return Err(ErrorTag::Unknown.to_error());
        }
        for processed in lines {
            let newline = preproc.fixup(processed);
            match newline {
                Err(error) => {
                    errors.tags.push(error);
                    is_error = true;
                }
                Ok(line) => {
                    preproc.processed.push(line);
                }
            }
        }
        if is_error {
            return Err(errors);
        }
        return Ok(preproc);
    }

    fn process_line(&mut self, line: ast::Line) -> Result<Vec<ProcessedLine>, ErrorTag> {
        let processed = self.process_line_body(line.body)?;
        if line.mark || 0 < line.labels.len() {
            let position = if processed.len() == 0 {
                self.position
            } else {
                processed[0].address
            };
            if line.mark {
                self.add_mark(position);
            }
            for label in line.labels {
                if self.labels.contains_key(&label) {
                    return Err(ErrorTag::LabelAlreadyDefined(label));
                }
                self.labels.insert(label.to_string(), position);
            }
        }
        return Ok(processed);
    }

    fn process_line_body(&mut self, line: ast::LineBody) -> Result<Vec<ProcessedLine>, ErrorTag> {
        let newlines = match line {
            // Sized strings are word aligned
            ast::LineBody::Data(data @ ast::Data::Str(_, _)) => {
                let position = self.align_to(WORD_SIZE);
                let body = ast::LineBody::Data(self.process_data(data)?);
                vec![ProcessedLine::new(body, position)]
            }
            ast::LineBody::Data(data) => {
                let position = self.position;
                let body = ast::LineBody::Data(self.process_data(data)?);
                vec![ProcessedLine::new(body, position)]
            }
            ast::LineBody::Directive(dir) => self.process_directive(dir)?,
            ast::LineBody::Simple(op) => {
                self.position += 1;
                let body = ast::LineBody::Simple(op);
                vec![ProcessedLine::new(body, self.position - 1)]
            }
            ast::LineBody::DefinitionRef(name) => {
                let mut lines = Vec::new();
                let list = self.expect_definition_list(&name)?;
                for line in list {
                    lines.extend(self.process_line_body(line)?);
                }
                lines
            }
        };
        return Ok(newlines);
    }

    fn process_data(&mut self, data: ast::Data) -> Result<ast::Data, ErrorTag> {
        self.position += data.size_in_bytes();
        Ok(match data {
            ast::Data::D(size, expr) => {
                let expr = self.process_expression(expr)?;
                let expr = self.simplify_expression(expr, self.position)?;
                if let Some(p) = expr.as_primitive() {
                    if size.size_in_bytes() < p.min_bytes() {
                        return Err(ErrorTag::DataSizeMismatch {
                            expected: size.size_in_bytes() as u8,
                            actual: p.min_bytes() as u8,
                        });
                    }
                }
                ast::Data::D(size, expr)
            }
            _ => data,
        })
    }

    fn process_directive(&mut self, dir: ast::Directive) -> Result<Vec<ProcessedLine>, ErrorTag> {
        match dir {
            ast::Directive::At(expr) => {
                let expr = self.simplify_expression(expr, self.position)?;
                let value = expr.as_primitive().unwrap().try_into::<u32>().unwrap() as usize;
                if self.position < value {
                    self.position = value;
                    Ok(vec![])
                } else {
                    Err(ErrorTag::CannotAtToBeforeCurrentPosition)
                }
            }
            ast::Directive::AlignTo(expr) => {
                let expr = self
                    .simplify_expression(expr, self.position)?
                    .as_primitive()
                    .unwrap();
                self.align_to(expr.try_into::<usize>().unwrap());
                Ok(vec![])
            }
            ast::Directive::Include(path) => {
                let mut lines = Vec::new();
                let program = self.includes.include_file(&path)?;
                for line in program.body {
                    lines.extend(self.process_line(line)?);
                }
                return Ok(lines);
            }
            ast::Directive::DefineList(name, list) => {
                if self.definitions.contains_key(&name) {
                    Err(ErrorTag::DefinitionAlreadyDefined(name))
                } else {
                    self.definitions.insert(name, Definition::DefList(list));
                    Ok(vec![])
                }
            }
            ast::Directive::DefineExpression(name, expr) => {
                if self.definitions.contains_key(&name) {
                    Err(ErrorTag::DefinitionAlreadyDefined(name))
                } else {
                    self.definitions.insert(name, Definition::DefExpr(expr));
                    Ok(vec![])
                }
            }
        }
    }

    fn process_expression(&self, expr: ast::Expression) -> Result<ast::Expression, ErrorTag> {
        match expr {
            ast::Expression::Tree(binop, lhs, rhs) => Ok(ast::Expression::Tree(
                binop,
                Box::new(self.process_expression(*lhs)?),
                Box::new(self.process_expression(*rhs)?),
            )),
            ast::Expression::DefinitionRef(name) => self.expect_definition_expression(&name),
            ast::Expression::Quoted(instruction) => {
                Ok(ast::Primitive::from(instruction.into_u8()).to_expr())
            }
            expr => Ok(expr),
        }
    }

    fn simplify_expression(
        &self,
        expr: ast::Expression,
        here: usize,
    ) -> Result<ast::Expression, ErrorTag> {
        Ok(match expr.clone() {
            ast::Expression::Address(addr) => match addr {
                ast::Address::Here => ast::Primitive::from(here as i64).to_expr(),
                ast::Address::Prev => ast::Primitive::from(self.resolve_prev()? as i64).to_expr(),
                ast::Address::Next => match self.resolve_next(here) {
                    None => ast::Expression::ForwardMarkRef(here),
                    Some(address) => ast::Primitive::from(address as i64).to_expr(),
                },
                ast::Address::LabelRef(name) => match self.resolve_label(&name[1..]) {
                    Some(addr) => ast::Primitive::from(addr as i64).to_expr(),
                    None => expr,
                },
            },
            ast::Expression::Tree(op, lhs, rhs) => {
                let lhs = self.simplify_expression(*lhs, here)?;
                let rhs = self.simplify_expression(*rhs, here)?;
                match (lhs.as_primitive(), rhs.as_primitive()) {
                    (Some(lhs), Some(rhs)) => ast::Expression::Primitive(match op {
                        ast::BinOp::Plus => lhs.add(rhs),
                        ast::BinOp::Minus => lhs.sub(rhs),
                        ast::BinOp::Times => lhs.mul(rhs),
                        ast::BinOp::And => lhs.and(rhs),
                        ast::BinOp::Pow => lhs.pow(rhs),
                        ast::BinOp::Div => lhs.div(rhs),
                        ast::BinOp::Or => lhs.or(rhs),
                    }),
                    _ => ast::Expression::Tree(op, Box::new(lhs), Box::new(rhs)),
                }
            }
            ast::Expression::DefinitionRef(name) => self.expect_definition_expression(&name)?,
            ast::Expression::ForwardMarkRef(position) => match self.resolve_next(position) {
                None => {
                    return Err(ErrorTag::ExpressionCannotBeSimplified(expr));
                }
                Some(address) => ast::Primitive::from(address as i64).to_expr(),
            },
            ast::Expression::ForwardLabelRef(name) => {
                ast::Primitive::from(self.resolve_label(&name).unwrap() as i64).to_expr()
            }
            expr => expr,
        })
    }

    // Some expressions cannot be evaluated at the time they are encountered,
    // and so we circle back around and evaluate them once everything else has
    // been accomplished.
    fn fixup(&self, processed: ProcessedLine) -> Result<ProcessedLine, ErrorTag> {
        match processed.body {
            ast::LineBody::Data(ast::Data::D(size, expr)) => {
                let expr = self.simplify_expression(expr, processed.address)?;
                if let Some(p) = expr.as_primitive() {
                    if size.size_in_bytes() < p.min_bytes() {
                        return Err(ErrorTag::DataSizeMismatch {
                            actual: p.min_bytes() as u8,
                            expected: size.size_in_bytes() as u8,
                        });
                    }
                    let body = ast::LineBody::Data(ast::Data::D(size, p.to_expr()));
                    return Ok(ProcessedLine {
                        address: processed.address,
                        body,
                    });
                } else {
                    return Err(ErrorTag::ExpressionCannotBeSimplified(expr));
                }
            }
            _ => return Ok(processed),
        }
    }
}
