#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Let { name: String, value: Expression },
    Println { values: Vec<Expression> },
}

#[derive(Debug)]
pub enum Expression {
    Number(i64),
    Variable(String),
    StringLit(String),
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
}

#[derive(Debug)]
pub enum BinaryOperator {
    Add,   // +
    Sub,   // -
    Mul,   // *
    Div,   // /
}