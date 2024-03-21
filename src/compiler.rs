use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

use crate::{chunk::{Chunk, OpCode}, scanner::Scanner, token::{Token, TokenType}, value::Value, vm::{InterpretError, Result}};

#[derive(FromPrimitive, Clone, Copy)]
pub enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

type ParseFn = for<'a> fn(&'a mut Compiler<'_>);

struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

impl ParseRule {
    pub const fn new(prefix: Option<ParseFn>, infix: Option<ParseFn>, precedence: Precedence) -> Self {
        Self {
            prefix,
            infix,
            precedence,
        }
    }

    pub const fn get(token: TokenType) -> &'static Self {
        &RULES[token as usize]
    }
}

macro_rules! parse_handler {
    ($fun:ident) => {
        Some(|compiler: &mut Compiler<'_>| compiler.$fun())
    }
}

const RULES: [ParseRule; 40] = [
    /* LeftParen    */ ParseRule::new(parse_handler!(grouping), None, Precedence::None),
    /* RightParen   */ ParseRule::new(None, None, Precedence::None),
    /* LeftBrace    */ ParseRule::new(None, None, Precedence::None),
    /* RightBrace   */ ParseRule::new(None, None, Precedence::None),
    /* Comma        */ ParseRule::new(None, None, Precedence::None),
    /* Dot          */ ParseRule::new(None, None, Precedence::None),
    /* Minus        */ ParseRule::new(parse_handler!(unary), parse_handler!(binary), Precedence::Term),
    /* Plus         */ ParseRule::new(None, parse_handler!(binary), Precedence::Term),
    /* Semicolon    */ ParseRule::new(None, None, Precedence::None),
    /* Slash        */ ParseRule::new(None, parse_handler!(binary), Precedence::Factor),
    /* Star         */ ParseRule::new(None, parse_handler!(binary), Precedence::Factor),
    /* Bang         */ ParseRule::new(parse_handler!(unary), None, Precedence::None),
    /* BangEqual    */ ParseRule::new(None, parse_handler!(binary), Precedence::Equality),
    /* Equal        */ ParseRule::new(None, None, Precedence::None),
    /* EqualEqual   */ ParseRule::new(None, parse_handler!(binary), Precedence::Equality),
    /* Greater      */ ParseRule::new(None, parse_handler!(binary), Precedence::Comparison),
    /* GreaterEqual */ ParseRule::new(None, parse_handler!(binary), Precedence::Comparison),
    /* Less         */ ParseRule::new(None, parse_handler!(binary), Precedence::Comparison),
    /* LessEqual    */ ParseRule::new(None, parse_handler!(binary), Precedence::Comparison),
    /* Identifier   */ ParseRule::new(None, None, Precedence::None),
    /* String       */ ParseRule::new(parse_handler!(string), None, Precedence::None),
    /* Number       */ ParseRule::new(parse_handler!(number), None, Precedence::None),
    /* And          */ ParseRule::new(None, None, Precedence::None),
    /* Class        */ ParseRule::new(None, None, Precedence::None),
    /* Else         */ ParseRule::new(None, None, Precedence::None),
    /* False        */ ParseRule::new(parse_handler!(literal), None, Precedence::None),
    /* For          */ ParseRule::new(None, None, Precedence::None),
    /* Fun          */ ParseRule::new(None, None, Precedence::None),
    /* If           */ ParseRule::new(None, None, Precedence::None),
    /* Nil          */ ParseRule::new(parse_handler!(literal), None, Precedence::None),
    /* Or           */ ParseRule::new(None, None, Precedence::None),
    /* Print        */ ParseRule::new(None, None, Precedence::None),
    /* Return       */ ParseRule::new(None, None, Precedence::None),
    /* Super        */ ParseRule::new(None, None, Precedence::None),
    /* This         */ ParseRule::new(None, None, Precedence::None),
    /* True         */ ParseRule::new(parse_handler!(literal), None, Precedence::None),
    /* Var          */ ParseRule::new(None, None, Precedence::None),
    /* While        */ ParseRule::new(None, None, Precedence::None),
    /* Error        */ ParseRule::new(None, None, Precedence::None),
    /* EOF          */ ParseRule::new(None, None, Precedence::None),
];

pub struct Compiler<'a> {
    scanner: Scanner<'a>,
    current: Option<Token>,
    previous: Option<Token>,
    current_chunk: Option<Chunk>,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        let scanner = Scanner::new(source);

        Self {
            scanner,
            current: None,
            previous: None,
            current_chunk: None,
            had_error: false,
            panic_mode: false,
        }
    }

    fn add_instruction(&mut self, instruction: OpCode) {
        if let Some(chunk) = self.current_chunk.as_mut() {
            chunk.write_chunk(instruction, self.previous.as_ref().unwrap().line());
        }
    }

    fn add_return(&mut self) {
        self.add_instruction(OpCode::Return);
    }

    fn add_constant(&mut self, value: Value) {
        let index = self.current_chunk.as_mut().unwrap().set_constant(value);
        self.add_instruction(OpCode::Constant(index));
    }

    pub fn compile(&mut self) -> Result<Chunk> {
        self.current_chunk = Some(Chunk::new());

        self.advance();
        self.expression();
        self.consume(TokenType::EOF, "Expect end of expression.");
        self.end_compiler();

        if self.had_error {
            Err(InterpretError::CompileError)
        } else {
            Ok(self.current_chunk.take().unwrap())
        }
    }

    fn string(&mut self) {
        let string = self.previous.as_ref().unwrap().lexeme();
        self.add_constant(Value::String(string[1..string.len() - 1].to_string()));
    }

    fn literal(&mut self) {
        match self.previous.as_ref().unwrap().token_type() {
            TokenType::False => self.add_instruction(OpCode::False),
            TokenType::True => self.add_instruction(OpCode::True),
            TokenType::Nil => self.add_instruction(OpCode::Nil),
            _ => (),
        }
    }

    fn binary(&mut self) {
        let operator = self.previous.as_ref().unwrap().token_type();
        let rule = ParseRule::get(operator);

        let index = rule.precedence as usize + 1;
        self.parse_precedence(Precedence::from_usize(index).unwrap());

        match operator {
            TokenType::BangEqual => {
                self.add_instruction(OpCode::Equal);
                self.add_instruction(OpCode::Not);
            }
            TokenType::EqualEqual => self.add_instruction(OpCode::Equal),
            TokenType::Greater => self.add_instruction(OpCode::Greater),
            TokenType::GreaterEqual => {
                self.add_instruction(OpCode::Less);
                self.add_instruction(OpCode::Not);
            }
            TokenType::Less => self.add_instruction(OpCode::Less),
            TokenType::LessEqual => {
                self.add_instruction(OpCode::Greater);
                self.add_instruction(OpCode::Not);
            }
            TokenType::Plus => self.add_instruction(OpCode::Add),
            TokenType::Minus => self.add_instruction(OpCode::Subtract),
            TokenType::Star => self.add_instruction(OpCode::Multiply),
            TokenType::Slash => self.add_instruction(OpCode::Divide),
            _ => (),
        }
    }

    fn unary(&mut self) {
        let operator_type = self.previous.as_ref().unwrap().token_type();

        self.parse_precedence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.add_instruction(OpCode::Negate),
            TokenType::Bang => self.add_instruction(OpCode::Not),
            _ => (),
        }
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn number(&mut self) {
        let number = self.previous.as_ref().unwrap().lexeme().parse().unwrap();
        self.add_constant(Value::Number(number));
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let prefix_rule = match ParseRule::get(self.previous.as_ref().unwrap().token_type()).prefix {
            Some(prefix) => prefix,
            None => {
                self.error("Expect expression.");
                return;
            }
        };

        prefix_rule(self);

        while precedence as usize <= ParseRule::get(self.current.as_ref().unwrap().token_type()).precedence as usize {
            self.advance();
            if let Some(infix_rule) = ParseRule::get(self.previous.as_ref().unwrap().token_type()).infix {
                infix_rule(self);
            }
        }
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn end_compiler(&mut self) {
        self.add_return();
        #[cfg(debug_print_code)] {
            if !self.had_error {
                if let Some(chunk) = self.current_chunk.as_ref() {
                    println!("{:?}", chunk);
                }
            }
        }
    }

    fn advance(&mut self) {
        self.previous = self.current.take();

        loop {
            self.current = Some(self.scanner.scan_token());
            if self.current.as_ref().unwrap().token_type() != TokenType::Error {
                break;
            }

            let message = self.current.as_ref().unwrap().lexeme().clone();
            self.error_at_current(message.as_str());
        }
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current.as_ref().unwrap().token_type() == token_type {
            self.advance();
            return;
        }

        self.error_at_current(message);
    }

    fn error_at_current(&mut self, message: &str) {
        if let Some(current) = self.current.clone() {
            self.error_at(&current, message);
        }
    }

    fn error(&mut self, message: &str) {
        let token = self.previous.as_ref().unwrap().clone();
        self.error_at(&token, message);
    }

    fn error_at(&mut self, token: &Token, message: &str) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;

        eprint!("[line {}] Error", token.line());

        match token.token_type() {
            TokenType::EOF => eprint!(" at EOF"),
            TokenType::Error => (),
            _ => eprint!(" at '{}'", token.lexeme()),
        }

        eprintln!(": {}", message);
        self.had_error = true;
    }
}

pub fn compile(source: &str) -> Result<Chunk> {
    let mut compiler = Compiler::new(source);

    compiler.compile()
}