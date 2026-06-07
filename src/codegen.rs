use crate::ast::*;
use std::collections::HashMap;

pub fn generate(program: &Program) -> String {
    let mut ir = String::new();
    let mut string_counter = 0;
    let mut strings = String::new();
    let mut body = String::new();
    let mut var_types = HashMap::new();

    for func in &program.functions {
        body.push_str(&generate_function(func, &mut string_counter, &mut strings, &mut var_types));
    }

    ir.push_str("declare i32 @printf(i8*, ...)\n\n");
    ir.push_str(&strings);
    ir.push_str(&body);
    ir
}

fn generate_function(func: &Function, string_counter: &mut usize, strings: &mut String, var_types: &mut HashMap<String, String>) -> String {
    let mut ir = format!("define i32 @{}() {{\n", func.name);
    let mut temp_counter = 0;
    for stmt in &func.body {
        ir.push_str(&generate_statement(stmt, &mut temp_counter, string_counter, strings, var_types));
    }
    ir.push_str("  ret i32 0\n");
    ir.push_str("}\n");
    ir
}

fn generate_statement(stmt: &Statement, temp_counter: &mut usize, string_counter: &mut usize, strings: &mut String, var_types: &mut HashMap<String, String>) -> String {
    match stmt {
        Statement::Let { name, value } => {
            let (val_ir, val_reg, val_type) = generate_expression(value, temp_counter, string_counter, strings, var_types);
            var_types.insert(name.clone(), val_type.clone());
            let alloca = if val_type == "i8*" {
                format!("  %{} = alloca i8*\n", name)
            } else {
                format!("  %{} = alloca i32\n", name)
            };
            let store = if val_type == "i8*" {
                format!("  store i8* {}, i8** %{}\n", val_reg, name)
            } else {
                format!("  store i32 {}, i32* %{}\n", val_reg, name)
            };
            format!("{}{}{}", val_ir, alloca, store)
        }
        Statement::Println { values } => {
            let mut ir = String::new();
            let mut regs = Vec::new();
            let mut types = Vec::new();
            for value in values {
                let (val_ir, val_reg, val_type) = generate_expression(value, temp_counter, string_counter, strings, var_types);
                ir.push_str(&val_ir);
                regs.push(val_reg);
                types.push(val_type);
            }
            let fmt_parts: Vec<String> = types.iter().map(|t| if t == "i8*" { "%s".to_string() } else { "%d".to_string() }).collect();
            let fmt_str = fmt_parts.join(" ");
            let fmt_len = fmt_str.len() + 2;
            let fmt_alloca = format!("  %fmt = alloca [{} x i8]\n", fmt_len);
            let fmt_store = format!("  store [{} x i8] c\"{}\\0A\\00\", [{} x i8]* %fmt\n", fmt_len, fmt_str, fmt_len);
            let fmt_ptr = format!("  %fmt_ptr = getelementptr [{} x i8], [{} x i8]* %fmt, i32 0, i32 0\n", fmt_len, fmt_len);
            let args: Vec<String> = regs.iter().zip(types.iter()).map(|(r, t)| format!("{} {}", t, r)).collect();
            let call_ir = format!("  call i32 (i8*, ...) @printf(i8* %fmt_ptr, {})\n", args.join(", "));
            ir.push_str(&fmt_alloca);
            ir.push_str(&fmt_store);
            ir.push_str(&fmt_ptr);
            ir.push_str(&call_ir);
            ir
        }
    }
}

fn generate_expression(expr: &Expression, temp_counter: &mut usize, string_counter: &mut usize, strings: &mut String, var_types: &HashMap<String, String>) -> (String, String, String) {
    match expr {
        Expression::Number(n) => (String::new(), format!("{}", n), "i32".to_string()),
        Expression::Variable(name) => {
            let reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;
            // Определяем тип переменной из хранилища
            let var_type = var_types.get(name).cloned().unwrap_or_else(|| "i32".to_string());
            let ir = if var_type == "i8*" {
                format!("  {} = load i8*, i8** %{}\n", reg, name)
            } else {
                format!("  {} = load i32, i32* %{}\n", reg, name)
            };
            (ir, reg, var_type)
        }
        Expression::StringLit(s) => {
            let label = format!(".str{}", *string_counter);
            *string_counter += 1;
            let escaped = s.replace("\\", "\\\\").replace("\"", "\\\"");
            let len = escaped.len() + 1;
            strings.push_str(&format!("@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\", align 1\n", label, len, escaped));
            let ptr = format!("getelementptr inbounds ([{} x i8], [{} x i8]* @{}, i32 0, i32 0)", len, len, label);
            (String::new(), ptr, "i8*".to_string())
        }
        Expression::BinaryOp { left, op, right } => {
            let (left_ir, left_reg, left_type) = generate_expression(left, temp_counter, string_counter, strings, var_types);
            let (right_ir, right_reg, right_type) = generate_expression(right, temp_counter, string_counter, strings, var_types);
            assert_eq!(left_type, "i32");
            assert_eq!(right_type, "i32");
            let op_str = match op {
                BinaryOperator::Add => "add",
                BinaryOperator::Sub => "sub",
                BinaryOperator::Mul => "mul",
                BinaryOperator::Div => "div",
            };
            let result_reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;
            let op_ir = format!("  {} = {} i32 {}, {}\n", result_reg, op_str, left_reg, right_reg);
            let ir = format!("{}{}{}", left_ir, right_ir, op_ir);
            (ir, result_reg, "i32".to_string())
        }
    }
}