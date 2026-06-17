mod ast;
mod codegen;
mod cache;
mod libs;

use crate::ast::*;
use std::env;
use std::fs;
use std::process::Command;
use std::collections::HashSet;
use std::path::Path;
use unicode_xid::UnicodeXID;

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
    Decrement,
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
        while self.current_token() == TokenKind::Star {
            self.advance();
            let right = self.parse_primary(in_class, in_constructor, param_names);
            left = Expression::BinaryOp { left: Box::new(left), op: BinaryOperator::Mul, right: Box::new(right) };
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
    if import.path.first() == Some(&"llgui".to_string()) {
        let last = import.path.last().unwrap();
        if last == "*" {
            let lp_dir = libs::get_lib_path().join("llgui");
            if let Ok(entries) = fs::read_dir(&lp_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("lp") {
                        load_file(&path, root_path, classes, functions, processed);
                    }
                }
            }
        } else {
            let lp_path = libs::get_lib_path().join(format!("llgui/{}.lp", last));
            if lp_path.exists() {
                load_file(&lp_path, root_path, classes, functions, processed);
            } else {
                panic!("LLGUI class '{}' not found in builtin library", last);
            }
        }
        return;
    }
    if import.path.first() == Some(&"llstd".to_string()) {
        let last = import.path.last().unwrap();
        if last == "*" {
            let lp_dir = libs::get_lib_path().join("llstd");
            if let Ok(entries) = fs::read_dir(&lp_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("lp") {
                        load_file(&path, root_path, classes, functions, processed);
                    }
                }
            }
        } else {
            let lp_path = libs::get_lib_path().join(format!("llstd/{}.lp", last));
            if lp_path.exists() {
                load_file(&lp_path, root_path, classes, functions, processed);
            } else {
                panic!("LLSTD class '{}' not found in builtin library", last);
            }
        }
        return;
    }

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
        panic!("Папка не найдена: {}", dir_path.display());
    }

    let mut lp_files = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("lp") {
                lp_files.push(path);
            }
        }
    }

    if let Some(name) = file_name {
        let target_name = format!("{}.lp", name);
        lp_files.retain(|p| p.file_name().and_then(|n| n.to_str()) == Some(&target_name));
        if lp_files.is_empty() {
            panic!("Файл {}.lp не найден в {}", name, dir_path.display());
        }
    }

    for file_path in lp_files {
        load_file(&file_path, root_path, classes, functions, processed);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() >= 2 && args[1] == "clean" {
        let cache_dir = cache::get_cache_dir();
        if cache_dir.exists() {
            std::fs::remove_dir_all(&cache_dir).expect("Failed to remove cache");
            println!("🧹 Кэш очищен: {}", cache_dir.display());
        } else {
            println!("ℹ️  Кэш не найден: {}", cache_dir.display());
        }
        return;
    }

    if args.len() < 2 {
        eprintln!("Использование: lopy <файл.lp> [--run]");
        eprintln!("           lopy clean");
        std::process::exit(1);
    }

    let filename = &args[1];
    let should_run = args.contains(&"--run".to_string());

    cache::ensure_dirs();
    libs::ensure_libs();

    let source = fs::read_to_string(filename).expect("Не удалось прочитать файл");
    let mut lexer = Lexer::new(&source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        if token.kind == TokenKind::EOF { break; }
        tokens.push(token);
    }

    let mut parser = Parser::new(tokens);
    let (program, imports) = parser.parse_program();

    let root_path = std::path::Path::new(filename).parent().unwrap().canonicalize().unwrap();
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

    let ir = codegen::generate(&mut final_program);

    let source_hash = cache::hash_string(&source);
    let cached_ll = cache::get_cache_dir().join(format!("{}.ll", source_hash));
    fs::write(&cached_ll, &ir).expect("Не удалось записать IR в кэш");

    let cached_s = cache::get_cache_dir().join(format!("{}.s", source_hash));
    let status = Command::new("llc")
        .arg(&cached_ll)
        .arg("-o")
        .arg(&cached_s)
        .status()
        .expect("llc не найден");
    if !status.success() {
        eprintln!("Ошибка компиляции IR в ассемблер");
        std::process::exit(1);
    }

    let out_name = if has_llgui { "gui" } else { "out" };
    let cached_bin = cache::get_cache_dir().join(format!("{}_{}", source_hash, out_name));

    let mut link_cmd = Command::new("clang");
    link_cmd.arg(&cached_s);
    link_cmd.arg("-o");
    link_cmd.arg(&cached_bin);
    link_cmd.arg("-no-pie");

    if has_llgui {
        let llgui_o = libs::get_gui_lib_path().join("lib/llgui.o");
        if llgui_o.exists() {
            link_cmd.arg(llgui_o);
        } else {
            eprintln!("Ошибка: llgui.o не найден");
            std::process::exit(1);
        }

        #[cfg(target_os = "linux")]
        link_cmd.args(&["-lX11", "-lXrandr", "-lGL"]);

        #[cfg(target_os = "windows")]
        link_cmd.args(&["-lgdi32", "-lopengl32"]);

        #[cfg(target_os = "macos")]
        link_cmd.args(&["-framework Cocoa", "-framework OpenGL"]);
    }

    let llstd_o_path = libs::get_std_lib_path().join("lib/llstd.o");
    if llstd_o_path.exists() {
        link_cmd.arg(llstd_o_path);
    }

    link_cmd.arg("-lm");

    let status = link_cmd.status().expect("clang не найден");
    if !status.success() {
        eprintln!("Ошибка линковки бинарника");
        std::process::exit(1);
    }

    if should_run {
        let output = Command::new(&cached_bin)
            .output()
            .expect("Ошибка запуска программы");
        print!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
    } else {
        let dest = Path::new(&out_name);
        fs::copy(&cached_bin, dest).expect("Не удалось скопировать бинарник");
        println!("✅ Скомпилировано в ./{}", out_name);
        if has_llgui {
            println!("   Запустите: ./gui");
        } else {
            println!("   Запустите: ./out");
        }
    }
}