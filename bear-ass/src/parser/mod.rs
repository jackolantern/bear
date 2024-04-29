use pest::iterators::{Pair, Pairs};
use pest::Parser as ParserTrait;
use pest_derive::Parser;

use bear_vm::vm;

pub mod ast;

#[derive(Parser)]
#[grammar = "bearasm.pest"] // relative to src
pub struct G;

#[derive(Debug)]
pub struct Error {
    pub message: String,
    pub position: Option<Position>,
}

impl Error {
    fn unknown(e: &dyn std::fmt::Display) -> Error {
        Error {
            message: e.to_string(),
            position: None,
        }
    }

    fn from_message(message: &str) -> Error {
        Error {
            message: message.to_string(),
            position: None,
        }
    }

    fn with_position_from_pair(mut self, pair: &Pair<Rule>) -> Error {
        let span = pair.as_span();
        let start = span.start_pos();
        let (line, column) = start.line_col();
        self.position = Some(Position { line, column });
        self
    }

    fn unsupported(pair: &Pair<Rule>) -> Error {
        let span = pair.as_span();
        let start = span.start_pos();
        let (line, column) = start.line_col();
        let position = Some(Position { line, column });
        Error {
            message: format!("Unsupported {:?} -- {}", pair.as_rule(), pair.as_str()),
            position,
        }
    }
}

#[derive(Debug)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

fn get_line_number(pair: &Pair<Rule>) -> usize {
    let span = pair.as_span();
    let start = span.start_pos();
    let (number, _) = start.line_col();
    number
}

pub struct Parser {}

// The `expect` calls in these methods should never result in a panic,
// unless there is a bug in the pest parser generator (or in `bear.pest`).
impl Parser {
    pub fn parse(mut self, text: &str) -> Result<ast::Program, Error> {
        let start = G::parse(Rule::start, text).map_err(|e| Error::unknown(&e))?;
        self.parse_start(start)
    }

    fn parse_start(&mut self, mut start: Pairs<Rule>) -> Result<ast::Program, Error> {
        let mut body = Vec::new();
        let mut program = start.next().unwrap().into_inner();
        for line in program.next().unwrap().into_inner() {
            if line.as_rule() == Rule::EOI {
                break;
            }
            body.push(self.parse_line(line)?);
        }
        Ok(ast::Program { body })
    }

    fn parse_line(&mut self, line: Pair<Rule>) -> Result<ast::Line, Error> {
        let number = get_line_number(&line);
        let line = line.into_inner().next().unwrap();
        match line.as_rule() {
            Rule::meta => Ok(ast::Line {
                mark: false,
                labels: Vec::new(),
                body: self.parse_meta(line)?,
                number,
            }),
            Rule::normal => self.parse_normal(line),
            _ => Err(Error::unsupported(&line).with_position_from_pair(&line)),
        }
    }

    fn parse_meta(&mut self, line: Pair<Rule>) -> Result<ast::LineBody, Error> {
        let line = line.into_inner().next().unwrap();
        Ok(match line.as_rule() {
            Rule::sep => {
                ast::LineBody::Directive(ast::Directive::AlignTo(ast::Primitive::from(4).to_expr()))
            }
            Rule::directive => ast::LineBody::Directive(self.parse_directive(line)?),
            _ => {
                return Err(Error::unsupported(&line).with_position_from_pair(&line));
            }
        })
    }

    fn parse_normal(&mut self, line: Pair<Rule>) -> Result<ast::Line, Error> {
        let number = get_line_number(&line);
        let mut mark = false;
        let mut labels = Vec::new();
        let mut line = line.into_inner();
        let label_list = line.next().unwrap().into_inner();
        let body = line.next().unwrap();
        for label in label_list {
            let lstr = label.as_str();
            if lstr == "$" {
                mark = true;
            } else {
                labels.push(String::from(&lstr[1..]));
            }
        }
        let body = self.parse_normal_body(body)?;
        Ok(ast::Line {
            mark,
            labels,
            body,
            number,
        })
    }

    fn parse_normal_body(&mut self, line: Pair<Rule>) -> Result<ast::LineBody, Error> {
        Ok(match line.as_rule() {
            Rule::data => ast::LineBody::Data(self.parse_data(line)?),
            Rule::definition_ref => ast::LineBody::DefinitionRef(line.as_str()[1..].to_string()),
            Rule::instruction => ast::LineBody::Simple(self.parse_opcode(line.as_str())?),
            _ => {
                return Err(Error::unsupported(&line).with_position_from_pair(&line));
            }
        })
    }

    fn parse_directive(&mut self, directive: Pair<Rule>) -> Result<ast::Directive, Error> {
        let mut directive = directive.into_inner();
        let name = directive.next().expect("directive has no name?");
        match name.as_str() {
            "#at" => self.parse_command_at(name, directive),
            "#align" => self.parse_command_align(name, directive),
            "#define" => self.parse_command_define(name, directive),
            "#include" => self.parse_command_include(name, directive),
            // TODO:
            // "#repeat" => self.parse_command_repeat(name, directive),
            _ => Err(Error::unknown(&name.as_str()).with_position_from_pair(&name)),
        }
    }

    fn parse_command_at(
        &mut self,
        directive: Pair<Rule>,
        mut arguments: Pairs<Rule>,
    ) -> Result<ast::Directive, Error> {
        let first = expect_argument(&directive, arguments.next())?;
        expect_no_argument(&directive, arguments, 1)?;
        let expression = self.parse_expression(first)?;
        Ok(ast::Directive::At(expression))
    }

    fn parse_command_align(
        &mut self,
        directive: Pair<Rule>,
        mut arguments: Pairs<Rule>,
    ) -> Result<ast::Directive, Error> {
        let first = expect_argument(&directive, arguments.next())?;
        expect_no_argument(&directive, arguments, 1)?;
        let expression = self.parse_expression(first)?;
        Ok(ast::Directive::AlignTo(expression))
    }

    fn parse_command_define(
        &mut self,
        directive: Pair<Rule>,
        mut arguments: Pairs<Rule>,
    ) -> Result<ast::Directive, Error> {
        let name = expect(directive, Rule::identifier, arguments.next())?;
        let definition = expect_argument(&name, arguments.next())?;
        match definition.as_rule() {
            Rule::argument_list => {
                let list = self.parse_argument_list(definition)?;
                return Ok(ast::Directive::DefineList(name.as_str().to_string(), list));
            }
            _ => {
                let expression = self.parse_expression(definition)?;
                return Ok(ast::Directive::DefineExpression(
                    name.as_str().to_string(),
                    expression,
                ));
            }
        }
    }

    fn parse_command_include(
        &mut self,
        directive: Pair<Rule>,
        mut arguments: Pairs<Rule>,
    ) -> Result<ast::Directive, Error> {
        let first = expect_argument(&directive, arguments.next())?.as_str();
        expect_no_argument(&directive, arguments, 1)?;
        let path = std::path::PathBuf::from(&first[1..first.len() - 1]);
        Ok(ast::Directive::Include(path))
    }

    fn parse_argument_list(&mut self, list: Pair<Rule>) -> Result<Vec<ast::LineBody>, Error> {
        let mut lines = Vec::new();
        for line in list.into_inner() {
            lines.push(self.parse_normal_body(line)?);
        }
        Ok(lines)
    }

    /*
     * TODO
    fn parse_command_repeat(&mut self, directive: Pair<Rule>, arguments: Pairs<Rule>) -> Result<ast::Directive, Error> {
        let first = expect_argument(directive, arguments.next())?;
        expect_no_argument(directive, arguments, 1);
        let count = self.parse_expression(command.next().unwrap())?;
        let data = self.parse_data(command.next().unwrap())?;
        return Ok(ast::Directive::Repeat(count, data));
    }
    */

    fn parse_data(&mut self, data: Pair<Rule>) -> Result<ast::Data, Error> {
        let data = data.into_inner().next().unwrap();
        Ok(match data.as_rule() {
            Rule::string => {
                let string = data.clone().into_inner().next().unwrap();
                let rule = string.as_rule();
                let content = self.parse_string(data)?;
                match rule {
                    Rule::r_string => ast::Data::Str(ast::StringTag::R, content),
                    Rule::c_string => ast::Data::Str(ast::StringTag::C, content),
                    Rule::s_string => ast::Data::Str(ast::StringTag::S, content),
                    rule => panic!("unreachable: {:?}", rule), //unreachable!()
                }
            }
            Rule::value => {
                let mut data = data.clone().into_inner();
                let size = data.next().unwrap();
                let expr = self.parse_expression(data.next().unwrap())?;
                match size.as_str() {
                    "d8" => ast::Data::D(ast::Size::S8, expr),
                    "d16" => ast::Data::D(ast::Size::S16, expr),
                    "d32" => ast::Data::D(ast::Size::S32, expr),
                    rule => panic!("unreachable: {:?}", rule), //unreachable!()
                }
            }
            rule => panic!("unreachable: {:?}", rule), //unreachable!()
        })
    }

    fn parse_string(&mut self, string: Pair<Rule>) -> Result<String, Error> {
        // let (start, finish) = string.as_span().split();
        // let hint = finish.pos() - start.pos();
        let nl_regex = regex::Regex::new(r"[^\\]\\n").unwrap();
        let slash_regex = regex::Regex::new(r"\\\\").unwrap();
        let s = string.into_inner().next().unwrap().as_str();
        let s = nl_regex.replace_all(s, "\n").to_string();
        let s = slash_regex.replace_all(&s, "\\").to_string();
        let len = s.len();
        Ok(s[2..len - 1].to_string())
    }

    fn parse_expression(&mut self, expr: Pair<Rule>) -> Result<ast::Expression, Error> {
        match expr.as_rule() {
            Rule::expression_leaf => self.parse_expression_leaf(expr.into_inner().next().unwrap()),
            Rule::expression_tree => self.parse_expression_tree(expr), //.into_inner().next().unwrap()),
            rule => panic!("unreachable: {:?}", rule),                 //unreachable!()
        }
    }

    fn parse_expression_tree(&mut self, tree: Pair<Rule>) -> Result<ast::Expression, Error> {
        let mut tree = tree.into_inner();
        let lhs = Box::new(self.parse_expression(tree.next().unwrap())?);
        let bop = tree.next().unwrap();
        let rhs = Box::new(self.parse_expression(tree.next().unwrap())?);
        Ok(match bop.as_str() {
            "+" => ast::Expression::Tree(ast::BinOp::Plus, lhs, rhs),
            "-" => ast::Expression::Tree(ast::BinOp::Minus, lhs, rhs),
            "*" => ast::Expression::Tree(ast::BinOp::Times, lhs, rhs),
            "&" => ast::Expression::Tree(ast::BinOp::And, lhs, rhs),
            "|" => ast::Expression::Tree(ast::BinOp::Or, lhs, rhs),
            "^" => ast::Expression::Tree(ast::BinOp::Pow, lhs, rhs),
            rule => panic!("unreachable: {:?}", rule), //unreachable!()
        })
    }

    fn parse_expression_leaf(&mut self, leaf: Pair<Rule>) -> Result<ast::Expression, Error> {
        Ok(match leaf.as_rule() {
            Rule::r#char => ast::Primitive::from(self.parse_char(leaf)? as u32).to_expr(),
            Rule::number => ast::Expression::Primitive(self.parse_number(leaf)?),
            Rule::address => ast::Expression::Address(self.parse_address(leaf)?),
            Rule::quoted => {
                let mut inner = leaf.into_inner();
                let op = inner.next().unwrap();
                match self.parse_opcode(op.as_str()) {
                    Ok(value) => ast::Expression::Quoted(value),
                    Err(err) => Err(err.with_position_from_pair(&op))?,
                }
            }
            Rule::definition_ref => {
                let name = (leaf.as_str()[1..]).to_string();
                ast::Expression::DefinitionRef(name)
            }
            // TODO: This is a bit of a hack.  Can we avoid the recursive call?
            Rule::expression_leaf => {
                let mut inner = leaf.into_inner();
                self.parse_expression_leaf(inner.next().unwrap())?
            }
            rule => panic!("unreachable {:?}", rule),
        })
    }

    fn parse_number(&mut self, number: Pair<Rule>) -> Result<ast::Primitive, Error> {
        let number = number.into_inner().next().unwrap();
        match number.as_rule() {
            Rule::number_dec => {
                Ok(ast::Primitive::from::<i64>(
                    number.as_str().parse().map_err(|e| Error::unknown(&e))?,
                ))
            }
            Rule::number_hex => {
                let strip = number.as_str().trim_start_matches("0x");
                let n = i64::from_str_radix(strip, 16).map_err(|e| Error::unknown(&e))?;
                Ok(ast::Primitive::from(n))
            }
            rule => panic!("unreachable: {:?}", rule),
        }
    }

    fn parse_address(&mut self, address: Pair<Rule>) -> Result<ast::Address, Error> {
        assert!(address.as_rule() == Rule::address);
        match address.as_str() {
            "@" => Ok(ast::Address::Here),
            "$>" => Ok(ast::Address::Next),
            "<$" => Ok(ast::Address::Prev),
            name => Ok(ast::Address::LabelRef(name.to_string())),
        }
    }
}

impl Parser {
    fn parse_char(&mut self, c: Pair<Rule>) -> Result<char, Error> {
        assert!(c.as_rule() == Rule::r#char);
        match c.as_str() {
            "'\\''" => Ok('\''),
            "'\\n'" => Ok('\n'),
            "'\\r'" => Ok('\r'),
            "'\\t'" => Ok('\t'),
            string => Ok(string.chars().nth(1).unwrap()),
        }
    }

    fn parse_opcode(&mut self, text: &str) -> Result<vm::OpCode, Error> {
        Ok(match text {
            "halt" => vm::OpCode::Halt,
            "lit" => vm::OpCode::Lit,

            "add" => vm::OpCode::Add,
            "sub" => vm::OpCode::Sub,
            "mul" => vm::OpCode::Mul,
            "shift" => vm::OpCode::Shift,
            "div" => vm::OpCode::Div,
            "mod" => vm::OpCode::Mod,

            "dup" => vm::OpCode::Dup,
            "drop" => vm::OpCode::Drop,
            "swap" => vm::OpCode::Swap,

            "call" => vm::OpCode::Call,
            "jump" => vm::OpCode::Jump,
            "ret" => vm::OpCode::Return,
            "ifz:call" => vm::OpCode::CallIfZ,
            "ifz:jump" => vm::OpCode::JumpIfZ,
            "ifz:ret" => vm::OpCode::ReturnIfZ,
            "io" => vm::OpCode::Io,

            "pop" => vm::OpCode::MoveAddrToData,
            "push" => vm::OpCode::MoveDataToAddr,

            "load" => vm::OpCode::Load,
            "store" => vm::OpCode::Store,
            "load.8" => vm::OpCode::Load8,
            "store.8" => vm::OpCode::Store8,
            "sext.8" => vm::OpCode::Sext8,
            "sext.16" => vm::OpCode::Sext16,

            "eq" => vm::OpCode::Equal,
            "lt" => vm::OpCode::LessThan,
            "gt" => vm::OpCode::GreaterThan,

            "and" => vm::OpCode::And,
            "or" => vm::OpCode::Or,
            "not" => vm::OpCode::Not,

            "nop" => vm::OpCode::Nop,

            _ => {
                return Err(Error::unknown(&text));
            }
        })
    }
}

fn expect<'a>(
    pair: Pair<Rule>,
    rule: Rule,
    on: Option<Pair<'a, Rule>>,
) -> Result<Pair<'a, Rule>, Error> {
    if on.is_none() {
        let message = format!("Expected '{:?}'.", rule);
        Err(Error::from_message(&message).with_position_from_pair(&pair))
    } else {
        let on = on.unwrap();
        if on.as_rule() != rule {
            let message = format!("Expected identifier, fonud '{:?}'.", on.as_rule());
            Err(Error::from_message(&message).with_position_from_pair(&pair))
        } else {
            Ok(on)
        }
    }
}

fn expect_argument<'a>(
    pair: &Pair<Rule>,
    on: Option<Pair<'a, Rule>>,
) -> Result<Pair<'a, Rule>, Error> {
    if on.is_none() {
        Err(Error::from_message("Expected argument.").with_position_from_pair(pair))
    } else {
        Ok(on.unwrap())
    }
}

fn expect_no_argument(
    pair: &Pair<Rule>,
    arguments: Pairs<'_, Rule>,
    n: usize,
) -> Result<(), Error> {
    let count = arguments.count();
    if count == 0 {
        Ok(())
    } else {
        let message = format!("Expected exactly {} arguments, but found {}.", n, n + count);
        Err(Error::from_message(&message).with_position_from_pair(pair))
    }
}
