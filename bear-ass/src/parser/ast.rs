use std::convert::TryFrom;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use bear_vm::vm;

pub type LineNumber = usize;
pub type LineAddress = usize;

#[derive(Debug, Default, Clone)]
pub struct Program {
    pub body: Vec<Line>,
}

/// Strings come in three flavors.
#[derive(Debug, Clone, Copy)]
pub enum StringTag {
    /// A "raw" string.  That is, a sequence of bytes with no size prefix or null terminator.
    R,
    /// A C-style string with a null-terminator.
    C,
    /// A sized string.  That is, sequence of bytes with a 32-bit prefix encoding the size.
    S,
}

/// Values come in one of three sizes.
#[derive(Debug, Clone, Copy)]
pub enum Size {
    /// 8-bit
    S8,
    /// 16-bit
    S16,
    /// 32-bit
    S32,
}

impl Size {
    pub fn size_in_bits(self) -> usize {
        match self {
            Size::S8 => 8,
            Size::S16 => 16,
            Size::S32 => 32,
        }
    }

    pub fn size_in_bytes(self) -> usize {
        match self {
            Size::S8 => 1,
            Size::S16 => 2,
            Size::S32 => 4,
        }
    }
}

/// Program data is either a string or a value.
#[derive(Clone, Debug)]
pub enum Data {
    D(Size, Expression),
    Str(StringTag, String),
}

impl Data {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            Data::D(size, _) => size.size_in_bytes(),
            Data::Str(StringTag::R, content) => content.bytes().len(),
            Data::Str(StringTag::C, content) => content.bytes().len() + 1,
            Data::Str(StringTag::S, content) => content.bytes().len() + 4,
        }
    }
}

/// Assembler directives.
#[derive(Debug, Clone)]
pub enum Directive {
    /// Add padding until the address given by the expression.
    At(Expression),
    /// Align next value to a multiple of the value given by the expression.
    AlignTo(Expression),
    /// Include the source file located at the given path.
    Include(PathBuf),
    /// Define a macro-block..
    DefineList(String, Vec<LineBody>),
    /// Define a macro-expression.
    DefineExpression(String, Expression),
}

/// A program line.
#[derive(Debug, Clone)]
pub struct Line {
    pub mark: bool,
    /// Labels are guaranteed to be unique.
    pub labels: Vec<String>,
    pub body: LineBody,
    pub number: usize,
}

/// The body of a program line.
#[derive(Debug, Clone)]
pub enum LineBody {
    Data(Data),
    Simple(vm::OpCode),
    Directive(Directive),
    DefinitionRef(String),
    // Comment(String),
}

/// Binary operations which may appear in expressions.
#[derive(Debug, Clone)]
pub enum BinOp {
    Pow,
    Div,
    Plus,
    Minus,
    Times,
    And,
    Or,
}

/// Encodes an address.
#[derive(Debug, Clone)]
pub enum Address {
    /// The "current" address.
    Here,
    /// The address of the next mark.
    Next,
    /// The address of the previous mark.
    Prev,
    /// The address of the given label.
    LabelRef(String),
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expression {
    Tree(BinOp, Box<Expression>, Box<Expression>),
    Address(Address),
    Primitive(Primitive),
    Quoted(vm::OpCode),
    DefinitionRef(String),
    ForwardMarkRef(usize),
    ForwardLabelRef(String),
}

impl Expression {
    pub fn as_primitive(&self) -> Option<Primitive> {
        match self {
            Expression::Primitive(p) => Some(*p),
            Expression::Tree(op, lhs, rhs) => {
                let lhs = lhs.as_primitive()?;
                let rhs = rhs.as_primitive()?;
                Some(match op {
                    BinOp::Or => lhs.or(rhs),
                    BinOp::And => lhs.and(rhs),
                    BinOp::Pow => lhs.pow(rhs),
                    BinOp::Div => lhs.div(rhs),
                    BinOp::Plus => lhs.add(rhs),
                    BinOp::Minus => lhs.sub(rhs),
                    BinOp::Times => lhs.mul(rhs),
                })
            }
            _ => None,
        }
    }
}

/** A primitive value.
 *
 * Although a memory cell can hold at most 32 bits, primitives are allowed to hold 64 bit values
 * so that intermediate expressions can take on larger values.
 */
#[derive(Debug, Clone, Copy)]
pub struct Primitive(i64);

impl Primitive {
    // TODO: what is the best way to do this?
    pub fn min_bytes(self) -> usize {
        let x = self.0.abs();
        if x <= u8::MAX as i64 {
            return 1;
        } else if x <= u16::MAX as i64 {
            return 2;
        } else if x <= u32::MAX as i64 {
            return 4;
        } else {
            return 8;
        }
    }

    pub fn sign(self) -> i64 {
        match self.0.cmp(&0) {
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Less => -1,
            std::cmp::Ordering::Greater => 1,
        }
    }

    pub fn from<T>(value: T) -> Self
    where
        i64: From<T>,
    {
        Primitive(i64::from(value))
    }

    pub fn try_into<T>(self) -> Option<T>
    where
        T: TryFrom<i64>,
    {
        T::try_from(self.0).ok()
    }

    pub fn assemble_8(self) -> Option<u8> {
        if self.min_bytes() <= 1 {
            if self.sign() == -1 {
                let v = self.0 as i8;
                let v: u8 = unsafe { std::mem::transmute_copy(&v) };
                return Some(v);
            } else {
                let v = self.0 as u8;
                return Some(v);
            }
        }
        None
    }

    pub fn assemble_16(self) -> Option<u16> {
        if self.min_bytes() <= 2 {
            if self.sign() == -1 {
                let v = self.0 as i16;
                let v: u16 = unsafe { std::mem::transmute_copy(&v) };
                return Some(v);
            } else {
                let v = self.0 as u16;
                return Some(v);
            }
        }
        None
    }

    pub fn assemble_32(self) -> Option<u32> {
        if self.sign() == -1 {
            let v = self.0 as i32;
            let v: u32 = unsafe { std::mem::transmute_copy(&v) };
            return Some(v);
        } else {
            let v = self.0 as u32;
            return Some(v);
        }
    }
}

impl Primitive {
    pub fn add(self, other: Self) -> Self {
        return Primitive(self.0 + other.0);
    }

    pub fn sub(self, other: Self) -> Self {
        return Primitive(self.0 - other.0);
    }

    pub fn mul(self, other: Self) -> Self {
        return Primitive(self.0 * other.0);
    }

    pub fn div(self, other: Self) -> Self {
        return Primitive(self.0 / other.0);
    }

    pub fn and(self, other: Self) -> Self {
        return Primitive(self.0 & other.0);
    }

    pub fn or(self, other: Self) -> Self {
        return Primitive(self.0 | other.0);
    }

    pub fn pow(self, other: Self) -> Self {
        return Primitive(self.0.pow(other.0 as u32));
    }

    pub fn to_expr(self) -> Expression {
        Expression::Primitive(self)
    }
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Or => write!(f, "|"),
            BinOp::And => write!(f, "&"),
            BinOp::Pow => write!(f, "^"),
            BinOp::Div => write!(f, "/"),
            BinOp::Plus => write!(f, "+"),
            BinOp::Minus => write!(f, "-"),
            BinOp::Times => write!(f, "*"),
        }
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Address::Here => write!(f, "@"),
            Address::Next => write!(f, "$>"),
            Address::Prev => write!(f, "<$"),
            Address::LabelRef(name) => write!(f, "{}", name),
        }
    }
}

impl std::fmt::Display for Directive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Directive::At(expr) => write!(f, "#at {};", expr),
            // TODO: Directive::Repeat(expr, data) => write!(f, "{} {}", data, expr),
            Directive::AlignTo(expr) => write!(f, "#align \"{}\";", expr),
            Directive::Include(path) => write!(f, "#include \"{}\";", path.display()),
            Directive::DefineList(name, lines) => {
                write!(f, "#define {} [", name)?;
                for line in lines.iter() {
                    line.fmt(f)?;
                    write!(f, ", ")?
                }
                write!(f, "];")
            }
            Directive::DefineExpression(name, expr) => write!(f, "#define {} {};", name, expr),
        }
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Address(address) => address.fmt(f),
            Expression::DefinitionRef(name) => write!(f, "!{}", name),
            Expression::Primitive(Primitive(n)) => n.fmt(f),
            Expression::Quoted(opcode) => opcode.fmt(f),
            Expression::Tree(bop, lhs, rhs) => write!(f, "({} {} {})", lhs, bop, rhs),
            Expression::ForwardMarkRef(_) => write!(f, "$"),
            Expression::ForwardLabelRef(name) => write!(f, "{}", name),
        }
    }
}

impl std::fmt::Display for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Data::D(size, expr) => write!(f, "d{} {}", size.size_in_bits(), expr),
            Data::Str(StringTag::C, content) => write!(f, "c\"{}\"", content),
            Data::Str(StringTag::R, content) => write!(f, "r\"{}\"", content),
            Data::Str(StringTag::S, content) => write!(f, "s\"{}\"", content),
        }
    }
}

impl std::fmt::Display for LineBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LineBody::Data(data) => data.fmt(f)?,
            LineBody::Simple(opcode) => opcode.fmt(f)?,
            LineBody::Directive(directive) => directive.fmt(f)?,
            LineBody::DefinitionRef(name) => write!(f, "!{}", name)?,
        };
        Ok(())
    }
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.mark {
            writeln!(f, "$")?;
        }
        for label in &self.labels {
            writeln!(f, ":{}", label)?;
        }
        self.body.fmt(f)?;
        Ok(())
    }
}

impl std::fmt::Display for Program {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for line in self.body.iter() {
            line.fmt(f)?
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Debug {
    pub body: Vec<DebugLine>,
    pub entries: Vec<DebugEntry>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub enum DebugTag {
    Data,
    Macro,
    Directive,
    Instruction,
}

#[derive(Serialize, Deserialize)]
pub struct DebugLine {
    pub tag: DebugTag,
    pub content: String,
}

#[derive(Serialize, Deserialize)]
pub struct DebugEntry {
    pub line: LineNumber,
    pub address: LineAddress,
    pub names: Vec<String>,
}
