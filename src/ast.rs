use std::fmt;

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Program {{")?;
        writeln!(f, "  classes: [")?;
        for class in &self.classes {
            let class_str = format!("{}", class);
            for line in class_str.lines() {
                writeln!(f, "    {}", line)?;
            }
        }
        writeln!(f, "  ],")?;
        writeln!(f, "  functions: [")?;
        for func in &self.functions {
            let func_str = format!("{}", func);
            for line in func_str.lines() {
                writeln!(f, "    {}", line)?;
            }
        }
        writeln!(f, "  ]")?;
        writeln!(f, "}}")
    }
}

impl fmt::Display for Class {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Class {{")?;
        writeln!(f, "    name: \"{}\",", self.name)?;
        writeln!(f, "    fields: [")?;
        for field in &self.fields {
            writeln!(f, "      {},", field)?;
        }
        writeln!(f, "    ],")?;
        writeln!(f, "    methods: [")?;
        for method in &self.methods {
            let method_str = format!("{}", method);
            for line in method_str.lines() {
                writeln!(f, "      {}", line)?;
            }
        }
        writeln!(f, "    ]")?;
        writeln!(f, "    }}")
    }
}

impl fmt::Display for Field {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Field {{ name: \"{}\", type_: {:?}, is_public: {} }}",
               self.name, self.type_, self.is_public)
    }
}

impl fmt::Display for Function {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Function {{")?;
        writeln!(f, "    name: \"{}\",", self.name)?;
        writeln!(f, "    params: [")?;
        for (name, t) in &self.params {
            writeln!(f, "      (\"{}\", {:?}),", name, t)?;
        }
        writeln!(f, "    ],")?;
        writeln!(f, "    return_type: {:?},", self.return_type)?;
        writeln!(f, "    is_public: {},", self.is_public)?;
        writeln!(f, "    is_static: {},", self.is_static)?;
        writeln!(f, "    is_constructor: {},", self.is_constructor)?;
        writeln!(f, "    body: [")?;
        for stmt in &self.body {
            let stmt_str = format!("{}", stmt);
            for line in stmt_str.lines() {
                writeln!(f, "      {}", line)?;
            }
        }
        writeln!(f, "    ]")?;
        writeln!(f, "  }}")
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Statement::Let { name, type_, value } => {
                writeln!(f, "Let {{ name: \"{}\", type_: {:?}, value: {} }}", name, type_, value)
            }
            Statement::Return { value } => {
                writeln!(f, "Return {{ value: {:?} }}", value)
            }
            Statement::Expr(expr) => {
                writeln!(f, "Expr({})", expr)
            }
            Statement::Println { values } => {
                write!(f, "Println {{ values: [")?;
                for (i, v) in values.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", v)?;
                }
                writeln!(f, "] }}")
            }
            Statement::If { condition, then_body, else_body } => {
                writeln!(f, "If {{")?;
                writeln!(f, "    condition: {},", condition)?;
                writeln!(f, "    then_body: [")?;
                for stmt in then_body {
                    let stmt_str = format!("{}", stmt);
                    for line in stmt_str.lines() {
                        writeln!(f, "        {}", line)?;
                    }
                }
                writeln!(f, "    ],")?;
                if let Some(else_body) = else_body {
                    writeln!(f, "    else_body: [")?;
                    for stmt in else_body {
                        let stmt_str = format!("{}", stmt);
                        for line in stmt_str.lines() {
                            writeln!(f, "        {}", line)?;
                        }
                    }
                    writeln!(f, "    ],")?;
                }
                writeln!(f, "  }}")
            }
            Statement::While { condition, body } => {
                writeln!(f, "While {{")?;
                writeln!(f, "    condition: {},", condition)?;
                writeln!(f, "    body: [")?;
                for stmt in body {
                    let stmt_str = format!("{}", stmt);
                    for line in stmt_str.lines() {
                        writeln!(f, "        {}", line)?;
                    }
                }
                writeln!(f, "    ]")?;
                writeln!(f, "  }}")
            }
            Statement::For { init, condition, increment, body } => {
                writeln!(f, "For {{")?;
                writeln!(f, "    init: {},", init)?;
                writeln!(f, "    condition: {},", condition)?;
                writeln!(f, "    increment: {},", increment)?;
                writeln!(f, "    body: [")?;
                for stmt in body {
                    let stmt_str = format!("{}", stmt);
                    for line in stmt_str.lines() {
                        writeln!(f, "        {}", line)?;
                    }
                }
                writeln!(f, "    ]")?;
                writeln!(f, "  }}")
            }
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Number(n) => write!(f, "Number({})", n),
            Expression::StringLit(s) => write!(f, "StringLit(\"{}\")", s),
            Expression::Variable(name) => write!(f, "Variable(\"{}\")", name),
            Expression::Input(t) => write!(f, "Input({:?})", t),
            Expression::BinaryOp { left, op, right } => {
                write!(f, "BinaryOp({}, {:?}, {})", left, op, right)
            }
            Expression::Call { func, args } => {
                write!(f, "Call(\"{}\", [", func)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, "])")
            }
            Expression::New { class, args } => {
                write!(f, "New(\"{}\", [", class)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, "])")
            }
            Expression::FieldAccess { object, field } => {
                write!(f, "FieldAccess({}, \"{}\")", object, field)
            }
            Expression::MethodCall { object, method, args } => {
                write!(f, "MethodCall({}, \"{}\", [", object, method)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, "])")
            }
            Expression::This => write!(f, "This"),
            Expression::Assignment { left, right } => {
                write!(f, "Assignment({}, {})", left, right)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    String,
    Void,
    Class(String),
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
}

#[derive(Debug)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
}