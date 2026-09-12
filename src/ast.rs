#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    String,
    Bool,
    Void,
    Class(String),
    Func,
}

#[derive(Debug)]
pub struct Program {
    pub functions: Vec<Function>,
    pub classes: Vec<Class>,
}

#[derive(Debug)]
pub struct Class {
    pub name: String,
    pub fields: Vec<Field>,
    pub methods: Vec<Function>,
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub body: Vec<Statement>,
    pub is_public: bool,
    pub is_static: bool,
    pub is_constructor: bool,
}

#[derive(Debug)]
pub struct Field {
    pub name: String,
    pub type_: Type,
    pub is_public: bool,
}

#[derive(Debug)]
pub enum Statement {
    Let { name: String, type_: Option<Type>, value: Expression },
    Return { value: Option<Expression> },
    Expr(Expression),
    Println { values: Vec<Expression> },
    If {
        condition: Box<Expression>,
        then_body: Vec<Statement>,
        else_body: Option<Vec<Statement>>,
    },
    While {
        condition: Box<Expression>,
        body: Vec<Statement>,
    },
    For {
        init: Box<Statement>,
        condition: Box<Expression>,
        increment: Box<Statement>,
        body: Vec<Statement>,
    },
}

#[derive(Debug)]
pub enum InputType {
    Int,
    String,
    Bool,
}

#[derive(Debug)]
pub enum Expression {
    Number(i128),
    StringLit(String),
    Variable(String),
    Input(InputType),
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    Call {
        func: String,
        args: Vec<Expression>,
    },
    New {
        class: String,
        args: Vec<Expression>,
    },
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    This,
    Assignment { left: Box<Expression>, right: Box<Expression> },
    Rand { max: Option<Box<Expression>> },
    CrCall { func: String, args: Vec<Expression> },
    Not(Box<Expression>),
    Thread {
        obj: Box<Expression>,
        code: String,
    },
    PostInc(Box<Expression>),
    PostDec(Box<Expression>),
    Lambda { body: Vec<Statement> },
}

#[derive(Debug)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge, Rem,
}