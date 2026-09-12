mod ast;
mod codegen;
mod cache;
mod pkg;

use crate::ast::*;
use std::env;
use std::fs;
use std::process::Command;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use home::home_dir;
use tar::Archive;
use unicode_xid::UnicodeXID;

const LLVM_MINGW_VERSION: &str = "20260616";

#[derive(Debug, PartialEq, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TokenKind {
    Fn, Let, Int, If, Class, Return,
    Public, Private, This, New, Not,
    Identifier(String), StringType, StringLit(String), Number(i128), Boolean(bool),
    Assign, Plus, Minus, Semicolon,
    LParen, RParen, LBrace, RBrace,
    Star, Slash, Comma, Colon, Arrow, Dot,
    EOF, Else, ColonColon, FnOut, BoolType,
    EqEq, Neq, Lt, Le, Gt, Ge, Rand,
    Input, While, For, Import, Cr, Increment,
    Decrement, Percent,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
    line: usize,
}

#[derive(Debug, Clone)]
struct Import {
    path: Vec<String>,
    line: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
            line: 1,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            if self.input[self.pos] == '\n' {
                self.line += 1;
            }
            self.pos += 1;
        }
    }

    fn read_identifier(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;

        while self.pos < self.input.len() &&
            (UnicodeXID::is_xid_start(self.input[self.pos]) || self.input[self.pos] == '_')
        {
            self.pos += 1;
        }
        while self.pos < self.input.len() &&
            (UnicodeXID::is_xid_continue(self.input[self.pos]) || self.input[self.pos] == '_')
        {
            self.pos += 1;
        }
        let word: String = self.input[start..self.pos].iter().collect();

        let kind = match word.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "int" => TokenKind::Int,
            "if" => TokenKind::If,
            "class" => TokenKind::Class,
            "return" => TokenKind::Return,
            "public" => TokenKind::Public,
            "private" => TokenKind::Private,
            "this" => TokenKind::This,
            "new" => TokenKind::New,
            "string" => TokenKind::StringType,
            "String" => TokenKind::StringType,
            "println" => TokenKind::Identifier("println".to_string()),
            "else" => TokenKind::Else,
            "input" => TokenKind::Input,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "true" => TokenKind::Boolean(true),
            "false" => TokenKind::Boolean(false),
            "import" => TokenKind::Import,
            "fnOut" => TokenKind::FnOut,
            "rand" => TokenKind::Rand,
            "cr" => TokenKind::Cr,
            "bool" => TokenKind::BoolType,
            "boolean" => TokenKind::BoolType,
            "функция" => TokenKind::Fn,
            "вывод" => TokenKind::Identifier("println".to_string()),
            "главная" => TokenKind::Identifier("main".to_string()),
            "если" => TokenKind::If,
            "иначе" => TokenKind::Else,
            "пока" => TokenKind::While,
            "для" => TokenKind::For,
            "класс" => TokenKind::Class,
            "новый" => TokenKind::New,
            "возврат" => TokenKind::Return,
            _ => TokenKind::Identifier(word),
        };
        Token { kind, line }
    }

    fn read_number(&mut self) -> Token {
        let start = self.pos;
        let line = self.line;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let num_str: String = self.input[start..self.pos].iter().collect();
        let num = num_str.parse::<i128>().unwrap_or_else(|_| {
            panic!("Число '{}' слишком большое для i128 на строке {}", num_str, line);
        });
        Token { kind: TokenKind::Number(num), line }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Token { kind: TokenKind::EOF, line: self.line };
        }

        if self.pos + 1 < self.input.len() && self.input[self.pos] == '/' && self.input[self.pos + 1] == '/' {
            while self.pos < self.input.len() && self.input[self.pos] != '\n' {
                self.pos += 1;
            }
            self.skip_whitespace();
            return self.next_token();
        }

        if self.pos + 1 < self.input.len() && self.input[self.pos] == '/' && self.input[self.pos + 1] == '*' {
            self.pos += 2;
            while self.pos + 1 < self.input.len() && !(self.input[self.pos] == '*' && self.input[self.pos + 1] == '/') {
                if self.input[self.pos] == '\n' {
                    self.line += 1;
                }
                self.pos += 1;
            }
            if self.pos + 1 >= self.input.len() {
                panic!("Незакрытый многострочный комментарий на строке {}", self.line);
            }
            self.pos += 2;
            self.skip_whitespace();
            return self.next_token();
        }

        let ch = self.input[self.pos];
        let line = self.line;
        match ch {
            '=' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token { kind: TokenKind::EqEq, line }
                } else {
                    Token { kind: TokenKind::Assign, line }
                }
            }
            '%' => {
                self.pos += 1;
                Token { kind: TokenKind::Percent, line }
            }
            '!' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token { kind: TokenKind::Neq, line }
                } else {
                    Token { kind: TokenKind::Not, line }
                }
            }
            '<' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token { kind: TokenKind::Le, line }
                } else {
                    Token { kind: TokenKind::Lt, line }
                }
            }
            '>' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '=' {
                    self.pos += 1;
                    Token { kind: TokenKind::Ge, line }
                } else {
                    Token { kind: TokenKind::Gt, line }
                }
            }
            '+' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '+' {
                    self.pos += 1;
                    Token { kind: TokenKind::Increment, line }
                } else {
                    Token { kind: TokenKind::Plus, line }
                }
            }
            '-' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == '-' {
                    self.pos += 1;
                    Token { kind: TokenKind::Decrement, line }
                } else if self.pos < self.input.len() && self.input[self.pos] == '>' {
                    self.pos += 1;
                    Token { kind: TokenKind::Arrow, line }
                } else {
                    Token { kind: TokenKind::Minus, line }
                }
            }
            ':' => {
                self.pos += 1;
                if self.pos < self.input.len() && self.input[self.pos] == ':' {
                    self.pos += 1;
                    Token { kind: TokenKind::ColonColon, line }
                } else {
                    Token { kind: TokenKind::Colon, line }
                }
            }
            '.' => { self.pos += 1; Token { kind: TokenKind::Dot, line } }
            ',' => { self.pos += 1; Token { kind: TokenKind::Comma, line } }
            '*' => { self.pos += 1; Token { kind: TokenKind::Star, line } }
            '/' => { self.pos += 1; Token { kind: TokenKind::Slash, line } }
            ';' => { self.pos += 1; Token { kind: TokenKind::Semicolon, line } }
            '(' => { self.pos += 1; Token { kind: TokenKind::LParen, line } }
            ')' => { self.pos += 1; Token { kind: TokenKind::RParen, line } }
            '{' => { self.pos += 1; Token { kind: TokenKind::LBrace, line } }
            '}' => { self.pos += 1; Token { kind: TokenKind::RBrace, line } }
            '"' => {
                self.pos += 1;
                let start = self.pos;
                let line = self.line;
                let mut s = String::new();
                while self.pos < self.input.len() && self.input[self.pos] != '"' {
                    if self.input[self.pos] == '\\' {
                        self.pos += 1;
                        if self.pos >= self.input.len() {
                            panic!("Незакрытая escape-последовательность на строке {}", line);
                        }
                        match self.input[self.pos] {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            _ => panic!("Неизвестная escape-последовательность \\{} на строке {}", self.input[self.pos], line),
                        }
                        self.pos += 1;
                    } else {
                        s.push(self.input[self.pos]);
                        self.pos += 1;
                    }
                    // НЕ проверяем на \n внутри строки — разрешаем многострочные строки
                }
                if self.pos >= self.input.len() {
                    panic!("Незакрытая строка на строке {}", line);
                }
                self.pos += 1;
                Token { kind: TokenKind::StringLit(s), line }
            }
            ch if ch.is_alphabetic() => self.read_identifier(),
            ch if ch.is_ascii_digit() => self.read_number(),
            _ => panic!("Неизвестный символ '{}' на строке {}", ch, line),
        }
    }
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }

    fn current_token(&self) -> TokenKind {
        if self.position >= self.tokens.len() {
            TokenKind::EOF
        } else {
            self.tokens[self.position].kind.clone()
        }
    }

    fn current_line(&self) -> usize {
        if self.position >= self.tokens.len() {
            self.tokens.last().map(|t| t.line).unwrap_or(0)
        } else {
            self.tokens[self.position].line
        }
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn expect(&mut self, expected: TokenKind) {
        if self.current_token() == expected {
            self.advance();
        } else {
            panic!("Ожидался {:?}, получен {:?} на строке {}", expected, self.current_token(), self.current_line());
        }
    }

    fn parse_identifier(&mut self) -> String {
        match self.current_token() {
            TokenKind::Identifier(name) => {
                self.advance();
                name
            }
            TokenKind::StringType => {
                self.advance();
                "String".to_string()
            }
            TokenKind::Int => {
                self.advance();
                "int".to_string()
            }
            _ => panic!("Ожидался идентификатор на строке {}", self.current_line()),
        }
    }

    fn parse_type(&mut self) -> Type {
        match self.current_token() {
            TokenKind::Int => { self.advance(); Type::Int }
            TokenKind::StringType => { self.advance(); Type::String }
            TokenKind::Identifier(ref name) if name == "void" => {
                self.advance();
                Type::Void
            }
            TokenKind::Identifier(ref name) if name == "func" => {
                self.advance();
                Type::Func
            }
            TokenKind::Identifier(name) => { self.advance(); Type::Class(name) }
            _ => panic!("Ожидался тип на строке {}", self.current_line()),
        }
    }

    fn parse_program(&mut self) -> (Program, Vec<Import>) {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        while self.current_token() != TokenKind::EOF {
            match self.current_token() {
                TokenKind::Import => {
                    let import = self.parse_import();
                    imports.push(import);
                }
                TokenKind::Fn => functions.push(self.parse_function()),
                TokenKind::Class => classes.push(self.parse_class()),
                _ => { self.advance(); }
            }
        }
        (Program { functions, classes }, imports)
    }

    fn parse_import(&mut self) -> Import {
        let line = self.current_line();
        self.expect(TokenKind::Import);

        let mut path = Vec::new();

        match self.current_token() {
            TokenKind::Identifier(name) => {
                path.push(name);
                self.advance();
            }
            _ => panic!("Ожидался идентификатор в импорте на строке {}", line),
        }

        while self.current_token() == TokenKind::ColonColon {
            self.advance();

            match self.current_token() {
                TokenKind::Identifier(name) => {
                    path.push(name);
                    self.advance();
                }
                TokenKind::Star => {
                    path.push("*".to_string());
                    self.advance();
                    break;
                }
                _ => panic!("Ожидался идентификатор или * в импорте на строке {}", line),
            }
        }

        self.expect(TokenKind::Semicolon);

        Import { path, line }
    }

    fn parse_class(&mut self) -> Class {
        self.expect(TokenKind::Class);
        let class_name = self.parse_identifier();
        self.expect(TokenKind::LBrace);

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while self.current_token() != TokenKind::RBrace && self.current_token() != TokenKind::EOF {
            let is_public = match self.current_token() {
                TokenKind::Public => { self.advance(); true }
                TokenKind::Private => { self.advance(); false }
                _ => true,
            };

            if self.is_method_declaration() {
                let method = self.parse_method(&class_name, is_public);
                methods.push(method);
            } else {
                let field_type = self.parse_type();
                let field_name = self.parse_identifier();
                if self.current_token() == TokenKind::Assign {
                    self.advance();
                    let _ = self.parse_expression(true, false, &HashSet::new());
                }
                self.expect(TokenKind::Semicolon);
                fields.push(Field { name: field_name, type_: field_type, is_public });
            }
        }
        self.expect(TokenKind::RBrace);
        Class { name: class_name, fields, methods }
    }

    fn is_method_declaration(&mut self) -> bool {
        let save = self.position;
        let mut result = false;
        match self.current_token() {
            TokenKind::Int | TokenKind::StringType | TokenKind::Identifier(_) => {
                let after_type = self.position;
                self.advance();
                match self.current_token() {
                    TokenKind::Identifier(_) => {
                        let after_name = self.position;
                        self.advance();
                        if self.current_token() == TokenKind::LParen { result = true; }
                        self.position = after_name;
                    }
                    TokenKind::LParen => { result = true; }
                    _ => {}
                }
                self.position = after_type;
            }
            _ => {}
        }
        self.position = save;
        result
    }

    fn parse_method(&mut self, class_name: &str, is_public: bool) -> Function {
        let return_type = self.parse_type();
        let name = match self.current_token() {
            TokenKind::Identifier(name) => {
                self.advance();
                name
            }
            _ => class_name.to_string()
        };
        let is_constructor = name == class_name;

        self.expect(TokenKind::LParen);
        let mut params = Vec::new();
        let mut param_names = HashSet::new();

        if self.current_token() != TokenKind::RParen {
            loop {
                let param_type = self.parse_type();
                let param_name = self.parse_identifier();
                param_names.insert(param_name.clone());
                params.push((param_name, param_type));
                match self.current_token() {
                    TokenKind::Comma => { self.advance(); continue; }
                    TokenKind::RParen => break,
                    _ => panic!("..."),
                }
            }
        }
        self.expect(TokenKind::RParen);
        self.expect(TokenKind::LBrace);

        let mut body = Vec::new();
        while self.current_token() != TokenKind::RBrace {
            body.push(self.parse_statement(true, is_constructor, &param_names));
        }
        self.expect(TokenKind::RBrace);

        Function {
            name: name.clone(),
            params,
            return_type,
            body,
            is_public,
            is_static: false,
            is_constructor,
        }
    }

    fn parse_function(&mut self) -> Function {
        self.expect(TokenKind::Fn);
        let name = self.parse_identifier();
        self.expect(TokenKind::LParen);
        let mut params = Vec::new();
        if self.current_token() != TokenKind::RParen {
            loop {
                let param_name = self.parse_identifier();
                self.expect(TokenKind::Colon);
                let param_type = self.parse_type();
                params.push((param_name, param_type));
                match self.current_token() {
                    TokenKind::Comma => { self.advance(); continue; }
                    TokenKind::RParen => break,
                    _ => panic!("Ожидалась запятая или закрывающая скобка на строке {}", self.current_line()),
                }
            }
        }
        self.expect(TokenKind::RParen);
        let return_type = if self.current_token() == TokenKind::Arrow {
            self.advance();
            self.parse_type()
        } else { Type::Void };
        self.expect(TokenKind::LBrace);
        let mut body = Vec::new();
        while self.current_token() != TokenKind::RBrace {
            body.push(self.parse_statement(false, false, &HashSet::new()));
        }
        self.expect(TokenKind::RBrace);
        Function {
            name,
            params,
            return_type,
            body,
            is_public: true,
            is_static: false,
            is_constructor: false,
        }
    }

    fn parse_expression(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Expression {
        self.parse_assignment(in_class, in_constructor, param_names)
    }

    fn parse_assignment(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Expression {
        let left = self.parse_comparison(in_class, in_constructor, param_names);
        if self.current_token() == TokenKind::Assign {
            self.advance();
            let right = self.parse_assignment(in_class, in_constructor, param_names);
            return Expression::Assignment { left: Box::new(left), right: Box::new(right) };
        }
        left
    }

    fn parse_comparison(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Expression {
        let mut left = self.parse_additive(in_class, in_constructor, param_names);
        while let TokenKind::EqEq | TokenKind::Neq | TokenKind::Lt | TokenKind::Le | TokenKind::Gt | TokenKind::Ge = self.current_token() {
            let op = match self.current_token() {
                TokenKind::EqEq => BinaryOperator::Eq,
                TokenKind::Neq => BinaryOperator::Ne,
                TokenKind::Lt => BinaryOperator::Lt,
                TokenKind::Le => BinaryOperator::Le,
                TokenKind::Gt => BinaryOperator::Gt,
                TokenKind::Ge => BinaryOperator::Ge,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_additive(in_class, in_constructor, param_names);
            left = Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }
        left
    }

    fn parse_additive(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Expression {
        let mut left = self.parse_multiplicative(in_class, in_constructor, param_names);
        while let TokenKind::Plus | TokenKind::Minus = self.current_token() {
            let op = match self.current_token() {
                TokenKind::Plus => BinaryOperator::Add,
                TokenKind::Minus => BinaryOperator::Sub,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_multiplicative(in_class, in_constructor, param_names);
            left = Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }
        left
    }

    fn parse_multiplicative(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Expression {
        let mut left = self.parse_primary(in_class, in_constructor, param_names);
        while self.current_token() == TokenKind::Star || self.current_token() == TokenKind::Percent {
            let op = match self.current_token() {
                TokenKind::Star => BinaryOperator::Mul,
                TokenKind::Percent => BinaryOperator::Rem,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_primary(in_class, in_constructor, param_names);
            left = Expression::BinaryOp { left: Box::new(left), op, right: Box::new(right) };
        }
        left
    }

    fn parse_primary(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Expression {
        let line = self.current_line();
        match self.current_token() {
            TokenKind::Number(n) => { self.advance(); Expression::Number(n) }
            TokenKind::Boolean(b) => { self.advance(); Expression::Number(if b { 1 } else { 0 }) }
            TokenKind::StringLit(s) => { self.advance(); Expression::StringLit(s) }
            TokenKind::New => {
                self.advance();
                let class = self.parse_identifier();
                self.expect(TokenKind::LParen);

                if class == "Thread" {
                    let obj = self.parse_expression(in_class, in_constructor, param_names);
                    self.expect(TokenKind::Comma);

                    let code = match self.current_token() {
                        TokenKind::StringLit(s) => {
                            self.advance();
                            s
                        }
                        _ => panic!("Thread: ожидается строка с кодом на строке {}", self.current_line()),
                    };
                    self.expect(TokenKind::RParen);
                    Expression::Thread { obj: Box::new(obj), code }
                } else {
                    let mut args = Vec::new();
                    while self.current_token() != TokenKind::RParen {
                        args.push(self.parse_expression(in_class, in_constructor, param_names));
                        if self.current_token() == TokenKind::Comma { self.advance(); }
                    }
                    self.expect(TokenKind::RParen);
                    Expression::New { class, args }
                }
            }
            TokenKind::Identifier(ref name) if name == "func" => {
                self.advance();
                self.expect(TokenKind::LBrace);
                let mut body = Vec::new();
                while self.current_token() != TokenKind::RBrace {
                    body.push(self.parse_statement(in_class, in_constructor, param_names));
                }
                self.expect(TokenKind::RBrace);
                Expression::Lambda { body }
            }
            TokenKind::Identifier(name) => {
                self.advance();

                if self.current_token() == TokenKind::Increment {
                    self.advance();
                    return Expression::PostInc(Box::new(Expression::Variable(name)));
                }
                if self.current_token() == TokenKind::Decrement {
                    self.advance();
                    return Expression::PostDec(Box::new(Expression::Variable(name)));
                }

                if self.current_token() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while self.current_token() != TokenKind::RParen {
                        args.push(self.parse_expression(in_class, in_constructor, param_names));
                        if self.current_token() == TokenKind::Comma { self.advance(); }
                    }
                    self.expect(TokenKind::RParen);

                    if in_class && !param_names.contains(&name) && !in_constructor {
                        Expression::MethodCall {
                            object: Box::new(Expression::This),
                            method: name,
                            args,
                        }
                    } else {
                        Expression::Call { func: name, args }
                    }
                }
                else if self.current_token() == TokenKind::Dot {
                    self.advance();
                    let member = self.parse_identifier();
                    if self.current_token() == TokenKind::LParen {
                        self.advance();
                        let mut args = Vec::new();
                        while self.current_token() != TokenKind::RParen {
                            args.push(self.parse_expression(in_class, in_constructor, param_names));
                            if self.current_token() == TokenKind::Comma { self.advance(); }
                        }
                        self.expect(TokenKind::RParen);
                        Expression::MethodCall {
                            object: Box::new(Expression::Variable(name)),
                            method: member,
                            args,
                        }
                    } else {
                        panic!("Доступ к полям запрещён. Используйте геттеры на строке {}", line);
                    }
                }
                else {
                    if param_names.contains(&name) {
                        Expression::Variable(name)
                    }
                    else {
                        Expression::Variable(name)
                    }
                }
            }
            TokenKind::This => {
                self.advance();
                let mut expr = Expression::This;
                if self.current_token() == TokenKind::Increment {
                    self.advance();
                    return Expression::PostInc(Box::new(expr));
                }
                if self.current_token() == TokenKind::Decrement {
                    self.advance();
                    return Expression::PostDec(Box::new(expr));
                }
                while self.current_token() == TokenKind::Dot {
                    self.advance();
                    let member = self.parse_identifier();
                    if self.current_token() == TokenKind::LParen {
                        self.advance();
                        let mut args = Vec::new();
                        while self.current_token() != TokenKind::RParen {
                            args.push(self.parse_expression(in_class, in_constructor, param_names));
                            if self.current_token() == TokenKind::Comma { self.advance(); }
                        }
                        self.expect(TokenKind::RParen);
                        expr = Expression::MethodCall {
                            object: Box::new(expr),
                            method: member,
                            args,
                        };
                    } else {
                        expr = Expression::FieldAccess {
                            object: Box::new(expr),
                            field: member,
                        };
                    }
                }
                expr
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression(in_class, in_constructor, param_names);
                self.expect(TokenKind::RParen);
                expr
            }
            TokenKind::Input => {
                self.advance();
                self.expect(TokenKind::LParen);
                let input_type = match self.current_token() {
                    TokenKind::Int => {
                        self.advance();
                        InputType::Int
                    }
                    TokenKind::StringType => {
                        self.advance();
                        InputType::String
                    }
                    TokenKind::Identifier(name) if name == "bool" => {
                        self.advance();
                        InputType::Bool
                    }
                    _ => panic!("Ожидался тип int, string или bool на строке {}", line),
                };
                self.expect(TokenKind::RParen);
                Expression::Input(input_type)
            }
            TokenKind::FnOut => {
                self.advance();
                self.expect(TokenKind::Dot);
                let func = self.parse_identifier();
                self.expect(TokenKind::LParen);
                let mut args = Vec::new();
                while self.current_token() != TokenKind::RParen {
                    args.push(self.parse_expression(in_class, in_constructor, param_names));
                    if self.current_token() == TokenKind::Comma { self.advance(); }
                }
                self.expect(TokenKind::RParen);
                Expression::Call { func, args }
            }
            TokenKind::Rand => {
                self.advance();
                self.expect(TokenKind::LParen);
                let max = if self.current_token() != TokenKind::RParen {
                    Some(Box::new(self.parse_expression(in_class, in_constructor, param_names)))
                } else {
                    None
                };
                self.expect(TokenKind::RParen);
                Expression::Rand { max }
            }
            TokenKind::Cr => {
                self.advance();
                self.expect(TokenKind::Dot);
                let func = self.parse_identifier();
                self.expect(TokenKind::LParen);
                let mut args = Vec::new();
                while self.current_token() != TokenKind::RParen {
                    args.push(self.parse_expression(in_class, in_constructor, param_names));
                    if self.current_token() == TokenKind::Comma { self.advance(); }
                }
                self.expect(TokenKind::RParen);
                Expression::CrCall { func, args }
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_primary(in_class, in_constructor, param_names);
                Expression::Not(Box::new(expr))
            }
            _ => panic!("Ожидалось выражение на строке {}", line),
        }
    }

    fn parse_statement(&mut self, in_class: bool, in_constructor: bool, param_names: &HashSet<String>) -> Statement {
        match self.current_token() {
            TokenKind::If => {
                self.advance();
                let condition = if in_class {
                    self.expect(TokenKind::LParen);
                    let expr = self.parse_expression(in_class, in_constructor, param_names);
                    self.expect(TokenKind::RParen);
                    expr
                } else {
                    self.parse_expression(in_class, in_constructor, param_names)
                };
                self.expect(TokenKind::LBrace);
                let mut then_body = Vec::new();
                while self.current_token() != TokenKind::RBrace {
                    then_body.push(self.parse_statement(in_class, in_constructor, param_names));
                }
                self.expect(TokenKind::RBrace);
                let else_body = if self.current_token() == TokenKind::Else {
                    self.advance();
                    if self.current_token() == TokenKind::If {
                        Some(vec![self.parse_statement(in_class, in_constructor, param_names)])
                    } else {
                        self.expect(TokenKind::LBrace);
                        let mut body = Vec::new();
                        while self.current_token() != TokenKind::RBrace {
                            body.push(self.parse_statement(in_class, in_constructor, param_names));
                        }
                        self.expect(TokenKind::RBrace);
                        Some(body)
                    }
                } else { None };
                Statement::If { condition: Box::new(condition), then_body, else_body }
            }
            TokenKind::Let => {
                self.advance();
                let name = self.parse_identifier();
                let type_ = if self.current_token() == TokenKind::Colon {
                    self.advance();
                    Some(self.parse_type())
                } else { None };
                self.expect(TokenKind::Assign);
                let value = self.parse_expression(in_class, in_constructor, param_names);
                self.expect(TokenKind::Semicolon);
                Statement::Let { name, type_, value }
            }
            TokenKind::Int | TokenKind::StringType if in_class => {
                let type_ = self.parse_type();
                let name = self.parse_identifier();
                self.expect(TokenKind::Assign);
                let value = self.parse_expression(in_class, in_constructor, param_names);
                self.expect(TokenKind::Semicolon);
                Statement::Let { name, type_: Some(type_), value }
            }
            TokenKind::Return => {
                self.advance();
                let value = if self.current_token() != TokenKind::Semicolon {
                    Some(self.parse_expression(in_class, in_constructor, param_names))
                } else { None };
                self.expect(TokenKind::Semicolon);
                Statement::Return { value }
            }
            TokenKind::Identifier(ref name) if name == "println" => {
                self.advance();
                self.expect(TokenKind::LParen);
                let mut values = Vec::new();
                while self.current_token() != TokenKind::RParen {
                    values.push(self.parse_expression(in_class, in_constructor, param_names));
                    if self.current_token() == TokenKind::Comma { self.advance(); }
                }
                self.expect(TokenKind::RParen);
                self.expect(TokenKind::Semicolon);
                Statement::Println { values }
            }
            TokenKind::While => {
                self.advance();
                let condition = if in_class {
                    self.expect(TokenKind::LParen);
                    let expr = self.parse_expression(in_class, in_constructor, param_names);
                    self.expect(TokenKind::RParen);
                    expr
                } else {
                    self.parse_expression(in_class, in_constructor, param_names)
                };
                self.expect(TokenKind::LBrace);
                let mut body = Vec::new();
                while self.current_token() != TokenKind::RBrace {
                    body.push(self.parse_statement(in_class, in_constructor, param_names));
                }
                self.expect(TokenKind::RBrace);
                Statement::While {
                    condition: Box::new(condition),
                    body,
                }
            }
            TokenKind::For => {
                self.advance();
                self.expect(TokenKind::LParen);

                let init = if let TokenKind::Let = self.current_token() {
                    self.advance();
                    let name = self.parse_identifier();
                    self.expect(TokenKind::Assign);
                    let value = self.parse_expression(in_class, in_constructor, param_names);
                    Statement::Let { name, type_: None, value }
                } else {
                    let expr = self.parse_expression(in_class, in_constructor, param_names);
                    Statement::Expr(expr)
                };

                self.expect(TokenKind::Semicolon);

                let condition = self.parse_expression(in_class, in_constructor, param_names);
                self.expect(TokenKind::Semicolon);

                let increment = {
                    let expr = self.parse_expression(in_class, in_constructor, param_names);
                    Statement::Expr(expr)
                };

                self.expect(TokenKind::RParen);
                self.expect(TokenKind::LBrace);

                let mut body = Vec::new();
                while self.current_token() != TokenKind::RBrace {
                    body.push(self.parse_statement(in_class, in_constructor, param_names));
                }
                self.expect(TokenKind::RBrace);

                Statement::For {
                    init: Box::new(init),
                    condition: Box::new(condition),
                    increment: Box::new(increment),
                    body,
                }
            }
            _ => {
                let expr = self.parse_expression(in_class, in_constructor, param_names);
                self.expect(TokenKind::Semicolon);
                Statement::Expr(expr)
            }
        }
    }
}

fn load_file(path: &std::path::Path, root_path: &std::path::Path, classes: &mut Vec<Class>, functions: &mut Vec<Function>, processed: &mut HashSet<std::path::PathBuf>) {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("[ERROR] Failed to canonicalize {:?}: {}", path, e);
            return;
        }
    };
    if processed.contains(&canonical) {
        eprintln!("[DEBUG] Already processed: {:?}", canonical);
        return;
    }
    processed.insert(canonical.clone());

    let source = fs::read_to_string(path).expect(&format!("Не удалось прочитать {}", path.display()));
    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::EOF { break; }
        tokens.push(token);
    }
    let mut parser = Parser::new(tokens);
    let (program, imports) = parser.parse_program();

    for class in program.classes {
        if classes.iter().any(|c| c.name == class.name) {
            panic!("Класс {} уже определён", class.name);
        }
        classes.push(class);
    }
    for func in program.functions {
        if functions.iter().any(|f| f.name == func.name) {
            panic!("Функция {} уже определена", func.name);
        }
        functions.push(func);
    }

    for import in imports {
        load_import_recursive(&import, root_path, classes, functions, processed);
    }
}

fn load_import_recursive(import: &Import, root_path: &std::path::Path, classes: &mut Vec<Class>, functions: &mut Vec<Function>, processed: &mut HashSet<std::path::PathBuf>) {
    // 1. Проверяем: это пакет из зависимостей?
    if let Some(project) = pkg::read_project_toml() {
        if let Some(deps) = project.dependencies {
            if let Some(pkg_name) = import.path.first() {
                if let Some(version) = deps.get(pkg_name) {
                    // Это зависимость! Ищем в ~/.lopy/packages/
                    let pkg_path = pkg::resolve_package(pkg_name, version);

                    // Проверяем что импортируется
                    if import.path.len() == 1 {
                        // Только имя пакета: import llgui;
                        // Загружаем ВСЕ .lp файлы из корня пакета (не рекурсивно!)
                        if let Ok(entries) = fs::read_dir(&pkg_path) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("lp") {
                                    load_file(&path, &pkg_path, classes, functions, processed);
                                }
                            }
                        }
                        return;
                    } else if import.path.len() == 2 && import.path[1] == "*" {
                        // import llgui::*;
                        // Загружаем ВСЕ .lp файлы из корня пакета (не рекурсивно!)
                        if let Ok(entries) = fs::read_dir(&pkg_path) {
                            for entry in entries.flatten() {
                                let path = entry.path();
                                if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("lp") {
                                    load_file(&path, &pkg_path, classes, functions, processed);
                                }
                            }
                        }
                        return;
                    } else {
                        // import llgui::Window; или import llgui::widgets::Button;
                        // Строим путь внутри пакета
                        let mut file_path = pkg_path.clone();
                        for segment in &import.path[1..] {
                            file_path.push(segment);
                        }
                        let lp_file = file_path.with_extension("lp");

                        if lp_file.exists() {
                            load_file(&lp_file, &pkg_path, classes, functions, processed);
                            return;
                        } else {
                            panic!("❌ Файл {} не найден в пакете {}", lp_file.display(), pkg_name);
                        }
                    }
                }
            }
        }
    }

    // 2. Специальная обработка для llstd (встроен в компилятор)
    if import.path.first() == Some(&"llstd".to_string()) {
        let llstd_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/llstd");

        if import.path.len() == 1 || import.path[1] == "*" {
            // import llstd; или import llstd::*;
            if let Ok(entries) = fs::read_dir(&llstd_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("lp") {
                        load_file(&path, &llstd_dir, classes, functions, processed);
                    }
                }
            }
            return;
        } else {
            // import llstd::Something;
            let mut file_path = llstd_dir.clone();
            for segment in &import.path[1..] {
                file_path.push(segment);
            }
            let lp_file = file_path.with_extension("lp");
            if lp_file.exists() {
                load_file(&lp_file, &llstd_dir, classes, functions, processed);
                return;
            } else {
                panic!("❌ llstd class not found: {}", lp_file.display());
            }
        }
    }

    // 3. ВСЁ ОСТАЛЬНОЕ - ищем локально (по старой логике)
    if import.path.is_empty() {
        return;
    }

    let last = import.path.last().unwrap();
    let is_star = last == "*";

    let folder_path = &import.path[0..import.path.len()-1];
    let file_name = if !is_star { Some(last.as_str()) } else { None };

    let mut dir_path = root_path.to_path_buf();
    for segment in folder_path {
        dir_path.push(segment);
    }

    if !dir_path.exists() {
        panic!("❌ Папка не найдена: {}", dir_path.display());
    }

    let mut lp_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("lp") {
                lp_files.push(path);
            }
        }
    }

    if let Some(name) = file_name {
        let target_name = format!("{}.lp", name);
        lp_files.retain(|p| p.file_name().and_then(|n| n.to_str()) == Some(&target_name));
        if lp_files.is_empty() {
            panic!("❌ Файл {}.lp не найден в {}", name, dir_path.display());
        }
    }

    for file_path in lp_files {
        load_file(&file_path, root_path, classes, functions, processed);
    }
}

pub fn check_tool_in_system(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn ensure_tools() {
    let lopy_dir = home_dir()
        .expect("Home dir not found")
        .join(".lopy");

    // Склад версий: ~/.lopy/bin/llvm-mingw-<версия>/
    let version_dir_name = format!("llvm-mingw-{}-ucrt-x86_64", LLVM_MINGW_VERSION);
    let bin_dir = lopy_dir.join("bin");
    let llvm_root = bin_dir.join(&version_dir_name);
    let llvm_bin = llvm_root.join("bin");
    let clang_path = llvm_bin.join("clang.exe");

    // 1. Проверяем, есть ли уже полная версия
    if clang_path.exists() {
        let lib_mingw = llvm_root
            .join("x86_64-w64-mingw32")
            .join("lib")
            .join("libmingw32.a");
        if lib_mingw.exists() {
            println!("✅ LLVM {} уже на месте.", LLVM_MINGW_VERSION);
            let _ = onpath::add(&llvm_bin, "lopy-llvm");
            return;
        }
    }

    // 2. Качаем полный архив
    println!("⏳ LLVM не найден. Скачиваю полный llvm-mingw {}...", LLVM_MINGW_VERSION);

    let url = format!(
        "https://github.com/mstorsjo/llvm-mingw/releases/download/{}/llvm-mingw-{}-ucrt-x86_64.zip",
        LLVM_MINGW_VERSION, LLVM_MINGW_VERSION
    );

    fs::create_dir_all(&bin_dir).expect("Не удалось создать ~/.lopy/bin");

    // 3. Временная папка
    let tmp_dir = lopy_dir.join("_tmp_llvm");
    fs::create_dir_all(&tmp_dir).expect("Не удалось создать tmp dir");

    // 4. Скачиваем и распаковываем целиком
    let mut archive = match arkiv::Archive::download(&url) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("❌ Не удалось скачать LLVM: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = archive.unpack(&tmp_dir) {
        eprintln!("❌ Не удалось распаковать архив: {}", e);
        let _ = fs::remove_dir_all(&tmp_dir);
        std::process::exit(1);
    }

    // 5. Находим внутреннюю папку llvm-mingw-*
    let inner = fs::read_dir(&tmp_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.is_dir()
                && p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("llvm-mingw-"))
                .unwrap_or(false)
        })
        .expect("Внутри архива не найдена папка llvm-mingw-*");

    // 6. Перемещаем ЦЕЛИКОМ в ~/.lopy/bin/llvm-mingw-<версия>/
    if llvm_root.exists() {
        let _ = fs::remove_dir_all(&llvm_root);
    }

    fs::rename(&inner, &llvm_root)
        .or_else(|_| copy_dir_recursive(&inner, &llvm_root))
        .expect("Не удалось переместить llvm-mingw");

    // 7. Сносим временную папку
    let _ = fs::remove_dir_all(&tmp_dir);

    // 10. Финальная проверка
    if !clang_path.exists() {
        eprintln!("❌ clang всё ещё не найден после установки.");
        std::process::exit(1);
    }

    println!("🎉 LLVM {} готов к работе!", LLVM_MINGW_VERSION);
}

fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

// Пути для вызова clang/lld
pub fn get_llvm_root() -> PathBuf {
    home_dir()
        .expect("no home dir")
        .join(".lopy")
        .join("bin")
        .join(format!("llvm-mingw-{}-ucrt-x86_64", LLVM_MINGW_VERSION))
}

/// Папка с бинарниками: ~/.lopy/bin/llvm-mingw-<версия>/bin/
pub fn get_llvm_bin_path() -> PathBuf {
    get_llvm_root().join("bin")
}

/// Папка с библиотеками: ~/.lopy/bin/llvm-mingw-<версия>/x86_64-w64-mingw32/lib/
pub fn get_llvm_lib_path() -> PathBuf {
    get_llvm_root()
        .join("x86_64-w64-mingw32")
        .join("lib")
}

/// Полный путь к clang
pub fn clang_path() -> PathBuf {
    get_llvm_bin_path().join("clang.exe")
}

/// Полный путь к lld
pub fn lld_path() -> PathBuf {
    get_llvm_bin_path().join("lld.exe")
}

fn handle_clean() {
    let cache_dir = cache::get_cache_dir();
    if cache_dir.exists() {
        std::fs::remove_dir_all(&cache_dir).expect("Failed to remove cache");
        println!("🧹 Кэш очищен: {}", cache_dir.display());
    } else {
        println!("ℹ️  Кэш не найден: {}", cache_dir.display());
    }
}

fn handle_pkg(args: &[String]) {
    pkg::ensure_dirs();

    match args[0].as_str() {
        "add" => {
            if args.len() < 2 {
                eprintln!("Использование: lopy pkg add <name> [version] [url]");
                std::process::exit(1);
            }
            let name = &args[1];
            let version = if args.len() >= 3 { &args[2] } else { "latest" };
            let url = if args.len() >= 4 { Some(args[3].as_str()) } else { None };
            pkg::add_package(name, version, url);
        }
        "remove" => {
            if args.len() < 2 {
                eprintln!("Использование: lopy pkg remove <name>");
                std::process::exit(1);
            }
            pkg::remove_package(&args[1]);
        }
        "list" => {
            pkg::list_packages();
        }
        _ => {
            eprintln!("Неизвестная команда pkg: {}", args[0]);
            eprintln!("Доступные: add, remove, list");
            std::process::exit(1);
        }
    }
}

fn compile_file(
    filename: &str,
    out_name: &str,
    should_run: bool,
    link_objects: Vec<PathBuf>,
    link_flags: Vec<String>,
) -> Result<(), String> {
    let source = fs::read_to_string(filename)
        .map_err(|e| format!("Не удалось прочитать файл: {}", e))?;
    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::EOF {
            break;
        }
        tokens.push(token);
    }

    let mut parser = Parser::new(tokens);
    let (program, imports) = parser.parse_program();

    let root_path = std::path::Path::new(filename)
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap();
    let main_abs_path = std::fs::canonicalize(filename).unwrap();

    let mut processed = HashSet::new();
    processed.insert(main_abs_path);

    let mut all_classes = program.classes;
    let mut all_functions = program.functions;
    let mut has_llgui = false;

    for import in imports {
        if import.path.first() == Some(&"llgui".to_string()) {
            has_llgui = true;
        }
        load_import_recursive(&import, &root_path, &mut all_classes, &mut all_functions, &mut processed);
    }

    let mut final_program = Program {
        classes: all_classes,
        functions: all_functions,
    };

    let mut all_signatures = HashMap::new();

    if let Some(project) = pkg::read_project_toml() {
        if let Some(deps) = project.dependencies {
            for (name, version) in deps {
                let pkg_dir = pkg::get_packages_dir().join(format!("{}@{}", name, version));
                let lib_dir = pkg_dir.join("lib");

                if let Ok(entries) = fs::read_dir(&lib_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|e| e.to_str()) == Some("c") {
                            let sigs = pkg::parse_c_signatures(&path);
                            all_signatures.extend(sigs);
                        }
                    }
                }
            }
        }
    }

    codegen::set_c_signatures(all_signatures);

    // 1. Генерируем IR
    let ir = codegen::generate(&mut final_program);

    let source_hash = cache::hash_string(&source);
    let cached_ll = cache::get_cache_dir().join(format!("{}.ll", source_hash));
    fs::write(&cached_ll, &ir)
        .map_err(|e| format!("Не удалось записать IR: {}", e))?;

    // 2. .ll -> .o через clang (без ассемблера)
    let cached_o = cache::get_cache_dir().join(format!("{}.o", source_hash));
    let status = Command::new(clang_path())
        .arg("-c")           // компилировать в объектник
        .arg(&cached_ll)
        .arg("-o")
        .arg(&cached_o)
        .status()
        .map_err(|_| "clang не найден".to_string())?;
    if !status.success() {
        return Err("Ошибка компиляции IR в объектник".to_string());
    }

    // 3. .o -> exe через clang + lld
    let out_name_final = out_name.to_string();
    let cached_bin = cache::get_cache_dir().join(format!("{}_{}", source_hash, out_name_final));

    let mut link_cmd = Command::new(clang_path());
    link_cmd.arg(&cached_o);       // объектник вместо .s
    link_cmd.arg("-o");
    link_cmd.arg(&cached_bin);
    link_cmd.arg("-fuse-ld=lld");
    link_cmd.arg(format!("-Wl,-L{}", get_llvm_lib_path().display()));
    link_cmd.arg("-lwinpthread");
    link_cmd.arg("-lucrt");
    link_cmd.arg("-lmsvcrt");

    for obj in link_objects {
        link_cmd.arg(obj);
    }

    for flag in link_flags {
        link_cmd.arg(flag);
    }

    let llstd_o = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/llstd/lib")
        .join(match std::env::consts::OS {
            "linux" => "llstd_linux.o",
            "windows" => "llstd_windows.o",
            "macos" => "llstd_macos.o",
            _ => panic!("❌ Неподдерживаемая платформа: {}", std::env::consts::OS),
        });

    if llstd_o.exists() {
        link_cmd.arg(&llstd_o);
    }

    // -lm только на Linux, на MinGW не нужно
    #[cfg(target_os = "linux")]
    link_cmd.arg("-lm");

    let status = link_cmd
        .status()
        .map_err(|_| "clang не найден".to_string())?;
    if !status.success() {
        return Err("Ошибка линковки бинарника".to_string());
    }

    // 4. Запуск или копирование
    if should_run {
        let status = Command::new(&cached_bin)
            .status()
            .map_err(|_| "Ошибка запуска программы".to_string())?;
        if !status.success() {
            return Err(format!(
                "Программа завершилась с кодом {}",
                status.code().unwrap_or(-1)
            ));
        }
    } else {
        let dest = Path::new(&out_name_final);
        fs::copy(&cached_bin, dest)
            .map_err(|e| format!("Не удалось скопировать бинарник: {}", e))?;
    }

    Ok(())
}

fn handle_build() {
    let args: Vec<String> = env::args().collect();
    let mut project_path = PathBuf::from(".");
    let mut i = 2; // пропускаем "lopy build"
    while i < args.len() {
        if args[i] == "--project" || args[i] == "-p" {
            if i + 1 < args.len() {
                project_path = PathBuf::from(&args[i + 1]);
                i += 2;
                continue;
            }
        }
        i += 1;
    }

    // Сохраняем текущую директорию
    let original_dir = env::current_dir().unwrap();

    // Переходим в папку проекта
    if project_path != PathBuf::from(".") {
        std::env::set_current_dir(&project_path).expect("Не удалось перейти в папку проекта");
        println!("📁 Перешли в: {}", project_path.display());
    }

    let project = pkg::read_project_toml();
    if project.is_none() {
        eprintln!("⚠️  lopy.toml не найден в {}", project_path.display());
        std::process::exit(1);
    }
    let project = project.unwrap();

    let main_path = Path::new("src/main.lp");
    if !main_path.exists() {
        eprintln!("⚠️  src/main.lp не найден в {}", project_path.display());
        std::process::exit(1);
    }

    println!("🔨 Сборка проекта {} v{}", project.package.name, project.package.version);

    let mut link_objects = Vec::new();
    let mut link_flags = Vec::new();

    if let Some(deps) = project.dependencies {
        for (name, version) in deps {
            let pkg_path = pkg::resolve_package(&name, &version);
            println!("📦 Используем пакет {} из {}", name, pkg_path.display());

            if let Ok((obj_path, flags)) = pkg::compile_package(&name, &version) {
                if obj_path.exists() {
                    link_objects.push(obj_path);
                    link_flags.extend(flags);
                }
            }
        }
    }

    if let Err(e) = compile_file(main_path.to_str().unwrap(), &project.package.name, false, link_objects, link_flags) {
        eprintln!("❌ Ошибка сборки: {}", e);
        std::process::exit(1);
    }

    let build_dir = Path::new("build");
    fs::create_dir_all(build_dir).expect("Не удалось создать папку build");

    let binary_name = &project.package.name;
    let source_bin = Path::new(&binary_name);
    let dest_bin = build_dir.join(binary_name);

    if source_bin.exists() {
        fs::copy(source_bin, &dest_bin).expect("Не удалось скопировать бинарник");
        println!("✅ Бинарник скопирован в {}", dest_bin.display());
        fs::remove_file(source_bin).unwrap_or_default();
    } else {
        eprintln!("❌ Бинарник не найден");
        std::process::exit(1);
    }

    // Возвращаемся обратно
    std::env::set_current_dir(&original_dir).unwrap();
}

fn handle_run() {
    let args: Vec<String> = env::args().collect();
    let mut project_path = PathBuf::from(".");
    let mut i = 2; // пропускаем "lopy run"
    while i < args.len() {
        if args[i] == "--project" || args[i] == "-p" {
            if i + 1 < args.len() {
                project_path = PathBuf::from(&args[i + 1]);
                i += 2;
                continue;
            }
        }
        i += 1;
    }

    // Сохраняем текущую директорию
    let original_dir = env::current_dir().unwrap();

    // Переходим в папку проекта
    if project_path != PathBuf::from(".") {
        std::env::set_current_dir(&project_path).expect("Не удалось перейти в папку проекта");
        println!("📁 Перешли в: {}", project_path.display());
    }

    let project = pkg::read_project_toml();
    if project.is_none() {
        eprintln!("⚠️  lopy.toml не найден в {}", project_path.display());
        std::process::exit(1);
    }
    let project = project.unwrap();

    let main_path = Path::new("src/main.lp");
    if !main_path.exists() {
        eprintln!("⚠️  src/main.lp не найден в {}", project_path.display());
        std::process::exit(1);
    }

    println!("🚀 Запуск проекта {} v{}", project.package.name, project.package.version);

    let mut link_objects = Vec::new();
    let mut link_flags = Vec::new();

    if let Some(deps) = project.dependencies {
        for (name, version) in deps {
            let pkg_path = pkg::resolve_package(&name, &version);
            println!("📦 Используем пакет {} из {}", name, pkg_path.display());

            if let Ok((obj_path, flags)) = pkg::compile_package(&name, &version) {
                if obj_path.exists() {
                    link_objects.push(obj_path);
                    link_flags.extend(flags);
                }
            }
        }
    }

    if let Err(e) = compile_file(main_path.to_str().unwrap(), &project.package.name, true, link_objects, link_flags) {
        eprintln!("❌ Ошибка выполнения: {}", e);
        std::process::exit(1);
    }

    // Возвращаемся обратно
    std::env::set_current_dir(&original_dir).unwrap();
}

fn handle_create(args: &[String]) {
    if args.is_empty() {
        eprintln!("Использование: lopy create <project_name>");
        std::process::exit(1);
    }

    let name = &args[0];
    let project_dir = Path::new(name);

    if project_dir.exists() {
        eprintln!("❌ Папка {} уже существует", name);
        std::process::exit(1);
    }

    // Создаём структуру
    fs::create_dir_all(project_dir.join("src")).expect("Failed to create src dir");

    // lopy.toml
    let toml_content = format!(
        r#"[package]
name = "{}"
version = "0.1.0"
authors = ["Your Name"]

[dependencies]
"#,
        name
    );
    fs::write(project_dir.join("lopy.toml"), toml_content).expect("Failed to write lopy.toml");

    // src/main.lp
    let main_content = r#"функция главная() {
    вывод("Hello, LopyLang! 🦎");
}
"#;
    fs::write(project_dir.join("src/main.lp"), main_content).expect("Failed to write main.lp");

    println!("✅ Проект {} создан!", name);
    println!("📁 cd {}", name);
    println!("🚀 lopy run");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    cache::ensure_dirs();
    ensure_tools();

    if args.len() >= 2 {
        match args[1].as_str() {
            "clean" => { handle_clean(); return; }
            "build" => { handle_build(); return; }
            "run" => { handle_run(); return; }
            "create" => {
                handle_create(&args[2..]);
                return;
            }
            "pkg" => {
                if args.len() < 3 {
                    eprintln!("Использование: lopy pkg <add|remove|list> [name] [url]");
                    std::process::exit(1);
                }
                handle_pkg(&args[2..]);
                return;
            }
            _ => {}
        }
    }

    if args.len() < 2 {
        eprintln!("Использование: lopy <файл.lp> [--run]");
        eprintln!("           lopy clean");
        eprintln!("           lopy build");
        eprintln!("           lopy run");
        eprintln!("           lopy pkg <add|remove|list>");
        std::process::exit(1);
    }

    let filename = &args[1];
    let should_run = args.contains(&"--run".to_string());

    let out_name = Path::new(filename)
        .file_stem()
        .unwrap()
        .to_string_lossy()
        .to_string();

    if let Err(e) = compile_file(filename, &out_name, should_run, Vec::new(), Vec::new()) {
        eprintln!("❌ Ошибка компиляции: {}", e);
        std::process::exit(1);
    }
}