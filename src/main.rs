mod codegen;

use std::fs;
use std::path::PathBuf;

// ---- Токены ----
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Ключевые слова
    Fn,
    Let,
    Int,
    If,
    Class,

    // Идентификаторы и литералы
    Identifier(String),
    Number(i64),

    // Символы
    Assign,     // =
    Plus,       // +
    Minus,      // -
    Semicolon,  // ;
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }

    // Специальные
    EOF,
}

// ---- Лексер ----
pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    // Пропуск пробелов и символов новой строки
    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() && self.input[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    // Чтение идентификатора или ключевого слова
    fn read_identifier(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_alphanumeric() {
            self.pos += 1;
        }
        let word: String = self.input[start..self.pos].iter().collect();
        match word.as_str() {
            "fn" => Token::Fn,
            "let" => Token::Let,
            "int" => Token::Int,
            "if" => Token::If,
            "class" => Token::Class,
            _ => Token::Identifier(word),
        }
    }

    // Чтение числа
    fn read_number(&mut self) -> Token {
        let start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        let num: i64 = self.input[start..self.pos].iter().collect::<String>().parse().unwrap();
        Token::Number(num)
    }

    // Получить следующий токен
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return Token::EOF;
        }

        let ch = self.input[self.pos];
        match ch {
            '=' => {
                self.pos += 1;
                Token::Assign
            }
            '+' => {
                self.pos += 1;
                Token::Plus
            }
            '-' => {
                self.pos += 1;
                Token::Minus
            }
            ';' => {
                self.pos += 1;
                Token::Semicolon
            }
            '(' => {
                self.pos += 1;
                Token::LParen
            }
            ')' => {
                self.pos += 1;
                Token::RParen
            }
            '{' => {
                self.pos += 1;
                Token::LBrace
            }
            '}' => {
                self.pos += 1;
                Token::RBrace
            }
            ch if ch.is_alphabetic() => self.read_identifier(),
            ch if ch.is_ascii_digit() => self.read_number(),
            _ => panic!("Неизвестный символ: {}", ch),
        }
    }
}

// ---- Точка входа для теста ----
fn main() {
    let source = r#"
fn main() {
    let x = 5;
    let y = 10;
}
"#;
    let mut lexer = Lexer::new(source);
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token();
        tokens.push(token.clone());
        if token == Token::EOF {
            break;
        }
    }

    let mut parser = Parser::new(tokens);
    let program = parser.parse_program();
    println!("AST:\n{:#?}", program);
}

// AST узлы
#[derive(Debug)]
struct Program {
    functions: Vec<Function>,
}

#[derive(Debug)]
struct Function {
    name: String,
    body: Vec<Statement>,
}

#[derive(Debug)]
enum Statement {
    Let { name: String, value: Expression },
}

#[derive(Debug)]
enum Expression {
    Number(i64),
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, position: 0 }
    }

    fn parse_program(&mut self) -> Program {
        let mut functions = Vec::new();
        while self.position < self.tokens.len() {
            if let Token::Fn = self.current_token() {
                functions.push(self.parse_function());
            } else {
                self.advance();
            }
        }
        Program { functions }
    }

    fn parse_function(&mut self) -> Function {
        self.expect(Token::Fn);
        let name = match self.current_token() {
            Token::Identifier(name) => name.clone(),
            _ => panic!("Ожидалось имя функции"),
        };
        self.advance(); // eat identifier
        self.expect(Token::LParen);
        self.expect(Token::RParen);
        self.expect(Token::LBrace);

        let mut body = Vec::new();
        while self.current_token() != Token::RBrace {
            body.push(self.parse_statement());
        }
        self.expect(Token::RBrace);

        Function { name, body }
    }

    fn parse_statement(&mut self) -> Statement {
        match self.current_token() {
            Token::Let => {
                self.advance();
                let name = match self.current_token() {
                    Token::Identifier(name) => name.clone(),
                    _ => panic!("Ожидалось имя переменной"),
                };
                self.advance();
                self.expect(Token::Assign);
                let value = self.parse_expression();
                self.expect(Token::Semicolon);
                Statement::Let { name, value }
            }
            _ => panic!("Неизвестное выражение"),
        }
    }

    fn parse_expression(&mut self) -> Expression {
        match self.current_token() {
            Token::Number(n) => {
                let num = n;
                self.advance();
                Expression::Number(num)
            }
            _ => panic!("Ожидалось число"),
        }
    }

    fn current_token(&self) -> Token {
        self.tokens[self.position].clone()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn expect(&mut self, expected: Token) {
        if self.current_token() == expected {
            self.advance();
        } else {
            panic!("Ожидался {:?}, получен {:?}", expected, self.current_token());
        }
    }
}