use std::mem::discriminant;

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

type ParseFn = for<'a> fn(&'a mut Compiler<'_>, bool);

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
        Some(|compiler: &mut Compiler<'_>, can_assign: bool| compiler.$fun(can_assign))
    }
}

macro_rules! patch_jump {
    ($compiler:ident, $offset:expr, $discriminant:ident) => {
        {
            let jump = $compiler.current_chunk.as_ref().unwrap().code.len() - 1;
            $compiler.patch_instruction($offset, OpCode::$discriminant(jump - $offset));
        }
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
    /* Identifier   */ ParseRule::new(parse_handler!(variable), None, Precedence::None),
    /* String       */ ParseRule::new(parse_handler!(string), None, Precedence::None),
    /* Number       */ ParseRule::new(parse_handler!(number), None, Precedence::None),
    /* And          */ ParseRule::new(None, parse_handler!(and), Precedence::None),
    /* Class        */ ParseRule::new(None, None, Precedence::None),
    /* Else         */ ParseRule::new(None, None, Precedence::None),
    /* False        */ ParseRule::new(parse_handler!(literal), None, Precedence::None),
    /* For          */ ParseRule::new(None, None, Precedence::None),
    /* Fun          */ ParseRule::new(None, None, Precedence::None),
    /* If           */ ParseRule::new(None, None, Precedence::None),
    /* Nil          */ ParseRule::new(parse_handler!(literal), None, Precedence::None),
    /* Or           */ ParseRule::new(None, parse_handler!(or), Precedence::None),
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

struct Local {
    name: Token,
    depth: usize,
}

pub struct Compiler<'a> {
    locals: Vec<Local>,
    scope_depth: usize,
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
            locals: Vec::new(),
            scope_depth: 0,
            scanner,
            current: None,
            previous: None,
            current_chunk: None,
            had_error: false,
            panic_mode: false,
        }
    }

    fn add_instruction(&mut self, instruction: OpCode) {
        let chunk = self.current_chunk.as_mut().unwrap();
        chunk.write_chunk(instruction, self.previous.as_ref().unwrap().line());
    }

    fn add_jump(&mut self, instruction: OpCode) -> usize {
        self.add_instruction(instruction);

        self.current_chunk.as_ref().unwrap().code.len() - 1
    }

    fn add_loop(&mut self, loop_start: usize) {
        let offset = self.current_chunk.as_ref().unwrap().code.len() - loop_start + 1;
        self.add_instruction(OpCode::Loop(offset))
    }

    fn patch_instruction(&mut self, address: usize, instruction: OpCode) {
        let chunk = self.current_chunk.as_mut().unwrap();
        chunk.code[address] = instruction;
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
        
        while !self.match_token(TokenType::EOF) {
            self.declaration();
        }

        self.end_compiler();

        if self.had_error {
            Err(InterpretError::CompileError)
        } else {
            Ok(self.current_chunk.take().unwrap())
        }
    }

    fn string(&mut self, _: bool) {
        let string = self.previous.as_ref().unwrap().lexeme();
        self.add_constant(Value::String(string[1..string.len() - 1].to_string()));
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) {
        let mut arg = self.resolve_local(&name);
        let (set_op, get_op) = if arg < usize::MAX {
            (OpCode::SetLocal(arg), OpCode::GetLocal(arg))
        } else {
            arg = self.current_chunk.as_mut().unwrap().set_constant(Value::String(name.lexeme().clone()));
            (OpCode::SetGlobal(arg), OpCode::GetGlobal(arg))
        };
        
        if can_assign && self.match_token(TokenType::Equal) {
            self.expression();
            self.add_instruction(set_op);
        } else {
            self.add_instruction(get_op);
        }
    }

    fn variable(&mut self, can_assign: bool) {
        let name = self.previous.as_ref().unwrap().clone();
        self.named_variable(name, can_assign);
    }

    fn literal(&mut self, _: bool) {
        match self.previous.as_ref().unwrap().token_type() {
            TokenType::False => self.add_instruction(OpCode::False),
            TokenType::True => self.add_instruction(OpCode::True),
            TokenType::Nil => self.add_instruction(OpCode::Nil),
            _ => (),
        }
    }

    fn binary(&mut self, _: bool) {
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

    fn unary(&mut self, _: bool) {
        let operator_type = self.previous.as_ref().unwrap().token_type();

        self.parse_precedence(Precedence::Unary);

        match operator_type {
            TokenType::Minus => self.add_instruction(OpCode::Negate),
            TokenType::Bang => self.add_instruction(OpCode::Not),
            _ => (),
        }
    }

    fn grouping(&mut self, _: bool) {
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after expression.");
    }

    fn number(&mut self, _: bool) {
        let number = self.previous.as_ref().unwrap().lexeme().parse().unwrap();
        self.add_constant(Value::Number(number));
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after value.");
        self.add_instruction(OpCode::Print);
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk.as_ref().unwrap().code.len();
        self.consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.add_jump(OpCode::Nil);
        self.add_instruction(OpCode::Pop);
        self.statement();
        self.add_loop(loop_start);

        patch_jump!(self, exit_jump, JumpIfFalse);
        self.add_instruction(OpCode::Pop);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.add_instruction(OpCode::Pop);
    }

    fn for_statement(&mut self) {
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        if self.match_token(TokenType::Semicolon) {}
        else if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk.as_ref().unwrap().code.len();
        let exit_jump = if !self.match_token(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expect ';' after loop condition.");
            let jump = self.add_jump(OpCode::Nil);
            self.add_instruction(OpCode::Pop);

            Some(jump)
        } else {
            None
        };

        if !self.match_token(TokenType::RightParen) {
            let body_jump = self.add_jump(OpCode::Nil);
            let increment_start = self.current_chunk.as_ref().unwrap().code.len();
            self.expression();
            self.add_instruction(OpCode::Pop);
            self.consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.add_loop(loop_start);
            loop_start = increment_start;
            patch_jump!(self, body_jump, Jump);
        }

        self.statement();
        self.add_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            patch_jump!(self, exit_jump, JumpIfFalse);
            self.add_instruction(OpCode::Pop);
        }

        self.end_scope();
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.add_jump(OpCode::Nil);
        self.add_instruction(OpCode::Pop);
        self.statement();
        let else_jump = self.add_jump(OpCode::Nil);
        patch_jump!(self, then_jump, JumpIfFalse);
        self.add_instruction(OpCode::Pop);

        if self.match_token(TokenType::Else) {
            self.statement();
        }
        patch_jump!(self, else_jump, Jump);
    }

    fn statement(&mut self) {
        if self.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.match_token(TokenType::For) {
            self.for_statement();
        } else if self.match_token(TokenType::If) {
            self.if_statement();
        } else if self.match_token(TokenType::While) {
            self.while_statement();
        } else if self.match_token(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.match_token(TokenType::Equal) {
            self.expression();
        } else {
            self.add_instruction(OpCode::Nil);
        }
        self.consume(TokenType::Semicolon, "Expect ';' after variable declaration.");

        self.define_variable(global);
    }

    fn declaration(&mut self) {
        if self.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.check(TokenType::EOF) {
            match self.previous.as_ref().unwrap().token_type() {
                TokenType::Class | 
                TokenType::Fun | 
                TokenType::Var | 
                TokenType::For | 
                TokenType::If | 
                TokenType::While | 
                TokenType::Print | 
                TokenType::Return => return,
                _ => (),
            }
            self.advance();
        }
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

        let can_assign = precedence as usize <= Precedence::Assignment as usize;
        prefix_rule(self, can_assign);

        while precedence as usize <= ParseRule::get(self.current.as_ref().unwrap().token_type()).precedence as usize {
            self.advance();
            if let Some(infix_rule) = ParseRule::get(self.previous.as_ref().unwrap().token_type()).infix {
                infix_rule(self, can_assign);
            }

            if can_assign && self.match_token(TokenType::Equal) {
                self.error("Invalid assignment target.");
            }
        }
    }

    fn parse_variable(&mut self, error_message: &str) -> usize {
        self.consume(TokenType::Identifier, error_message);

        self.declare_variable();

        if self.scope_depth > 0 {
            0
        } else {
            self.current_chunk.as_mut().unwrap().set_constant(Value::String(self.previous.as_ref().unwrap().lexeme().clone()))
        }
    }

    fn mark_initialized(&mut self) {
        self.locals.last_mut().unwrap().depth = self.scope_depth;
    }

    fn define_variable(&mut self, global: usize) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }

        self.add_instruction(OpCode::DefineGlobal(global));
    }

    fn and(&mut self, _: bool) {
        let end_jump = self.add_jump(OpCode::Nil);

        self.add_instruction(OpCode::Pop);
        self.parse_precedence(Precedence::And);

        patch_jump!(self, end_jump, JumpIfFalse);
    }

    fn or(&mut self, _: bool) {
        let else_jump = self.add_jump(OpCode::Nil);
        let end_jump = self.add_jump(OpCode::Nil);

        patch_jump!(self, else_jump, JumpIfFalse);
        self.add_instruction(OpCode::Pop);

        self.parse_precedence(Precedence::Or);
        patch_jump!(self, end_jump, JumpIfFalse);
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

    fn resolve_local(&mut self, name: &Token) -> usize {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if name.lexeme() == local.name.lexeme() {
                if local.depth == usize::MAX {
                    self.error("Cannot read local variable in its own initializer.");
                }
                return i;
            }
        }

        usize::MAX
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while self.locals.len() > 0 && self.locals.last().unwrap().depth > self.scope_depth {
            self.add_instruction(OpCode::Pop);
            self.locals.pop();
        }
    }

    fn add_local(&mut self, name: Token) {
        self.locals.push(Local {
            name,
            depth: usize::MAX,
        })
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self.previous.as_ref().unwrap().clone();
        let mut err = false;
        for local in self.locals.iter().rev() {
            if local.depth != usize::MAX && local.depth < self.scope_depth {
                break;
            }

            if name.lexeme() == local.name.lexeme() {
                err = true;
                break;
            }
        }
        if err {
            self.error("Variable with this name already declared in this scope.");
        }
        self.add_local(name);
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

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            return false;
        }

        self.advance();
        true
    }

    fn check(&self, token_type: TokenType) -> bool {
        discriminant(&self.current.as_ref().unwrap().token_type()) == discriminant(&token_type)
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