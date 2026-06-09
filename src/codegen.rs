use crate::ast::*;
use std::collections::HashMap;

pub fn generate(program: &Program) -> String {
    let mut ir = String::new();
    let mut string_counter = 0;
    let mut strings = String::new();
    let mut function_body = String::new();

    strings.push_str("@.str.true_val = private unnamed_addr constant [5 x i8] c\"true\\00\", align 1\n");
    strings.push_str("@.str.false_val = private unnamed_addr constant [6 x i8] c\"false\\00\", align 1\n");

    ir.push_str("declare i32 @printf(i8*, ...)\n");
    ir.push_str("declare i8* @malloc(i64)\n");
    ir.push_str("declare i32 @strcmp(i8*, i8*)\n");
    ir.push_str("declare i8* @strcpy(i8*, i8*)\n");
    ir.push_str("declare i8* @strcat(i8*, i8*)\n");
    ir.push_str("declare i64 @strlen(i8*)\n");
    ir.push_str("declare i32 @getchar()\n");
    ir.push_str("declare i32 @scanf(i8*, ...)\n");
    ir.push_str("declare i64 @strtol(i8*, i8**, i32)\n");
    ir.push_str("declare void @exit(i32)\n");
    ir.push_str("declare i32 @sprintf(i8*, i8*, ...)\n\n");
    ir.push_str("@.str.int_fmt = private unnamed_addr constant [3 x i8] c\"%d\\00\", align 1\n");
    ir.push_str("@.str.string_fmt = private unnamed_addr constant [3 x i8] c\"%s\\00\", align 1\n");
    strings.push_str("@.input_error_int = private unnamed_addr constant [25 x i8] c\"Error: expected integer\\0A\\00\", align 1\n");
    strings.push_str("@.input_error_bool = private unnamed_addr constant [31 x i8] c\"Error: expected true or false\\0A\\00\", align 1\n");
    ir.push_str("@.str.char_fmt = private unnamed_addr constant [3 x i8] c\"%c\\00\", align 1\n");
    ir.push_str("@.str.string_fmt_full = private unnamed_addr constant [8 x i8] c\" %[^\\n]\\00\", align 1\n");
    ir.push_str("@.str.string_fmt_skip = private unnamed_addr constant [8 x i8] c\" %[^\\n]\\00\", align 1\n");
    ir.push_str("@.str.num_fmt = private unnamed_addr constant [5 x i8] c\"%lld\\00\", align 1\n\n");
    ir.push_str("@.str.string_fmt_clean = private unnamed_addr constant [8 x i8] c\" %[^\\n]\\00\", align 1\n");
    ir.push_str("@.str.int_fmt_clean = private unnamed_addr constant [4 x i8] c\" %d\\00\", align 1\n");
    ir.push_str("declare i8* @fgets(i8*, i32, i8*)\n");
    ir.push_str("%struct._IO_FILE = type { i32, i8*, i8*, i8*, i8*, i8*, i8*, i8*, i8*, i8*, i8*, i8*, i32, i32, i32, i16, i8, [1 x i8], i8*, i64, i8*, i8*, i8*, i8*, i32, i32, i32, i32, i16, i8, [1 x i8], i8*, i64, i32, i32, i32, i32, i32, i32, i32, i32, i8*, i64, i8*, i64, i32, i32, i32, i32 }\n");
    ir.push_str("@stdin = external global %struct._IO_FILE*\n");

    for class in &program.classes {
        let mut fields = Vec::new();
        for field in &class.fields {
            fields.push(llvm_type(&field.type_));
        }
        ir.push_str(&format!("%{} = type {{ {}}}\n\n", class.name, fields.join(", ")));
    }

    for class in &program.classes {
        for method in &class.methods {
            check_function_returns(method, Some(&class.name));
            function_body.push_str(&generate_method(program, class, method, &mut string_counter, &mut strings));
        }
    }

    for func in &program.functions {
        check_function_returns(func, None);
        function_body.push_str(&generate_function(program, func, &mut string_counter, &mut strings));
    }

    ir.push_str(&strings);
    ir.push_str(&function_body);
    ir
}

fn check_function_returns(func: &Function, class_name: Option<&str>) {
    if func.is_constructor {
        return;
    }
    if func.return_type == Type::Void {
        return;
    }

    fn stmt_has_return(stmt: &Statement) -> bool {
        match stmt {
            Statement::Return { .. } => true,
            Statement::If { condition: _, then_body, else_body } => {
                let then_has = then_body.iter().any(stmt_has_return);
                let else_has = else_body.as_ref().map(|b| b.iter().any(stmt_has_return)).unwrap_or(false);
                else_body.is_some() && then_has && else_has
            }
            Statement::While { .. } | Statement::For { .. } => false,
            Statement::Let { .. } | Statement::Expr(_) | Statement::Println { .. } => false,
        }
    }

    let has_return = func.body.iter().any(stmt_has_return);
    let func_name = match class_name {
        Some(c) => format!("{}.{}", c, func.name),
        None => func.name.clone(),
    };
    if !has_return {
        panic!("Function '{}' must return a value, but may not have a return statement on all paths", func_name);
    }
}

fn generate_method(program: &Program, class: &Class, method: &Function, string_counter: &mut usize, strings: &mut String) -> String {
    let mut ir = String::new();
    let mut var_types: HashMap<String, (String, String)> = HashMap::new();

    let llvm_name = format!("{}_{}", class.name, method.name);
    let ret_type = if method.is_constructor {
        format!("%{}*", class.name)
    } else {
        llvm_type(&method.return_type)
    };
    let this_type = format!("%{}*", class.name);

    ir.push_str(&format!("define {} @{}({}", ret_type, llvm_name, this_type));
    for (_, param_type) in &method.params {
        ir.push_str(&format!(", {}", llvm_type(param_type)));
    }
    ir.push_str(") {\n");

    let this_alloca = format!("%this_{}", *string_counter);
    *string_counter += 1;
    ir.push_str(&format!("  {} = alloca {}\n", this_alloca, this_type));
    ir.push_str(&format!("  store {} %0, {}* {}\n", this_type, this_type, this_alloca));
    var_types.insert("this".to_string(), (this_alloca, this_type));

    let mut param_counter = 1;
    for (param_name, param_type) in &method.params {
        let llvm_t = llvm_type(param_type);
        let param_alloca = format!("%{}_{}", param_name, *string_counter);
        *string_counter += 1;
        ir.push_str(&format!("  {} = alloca {}\n", param_alloca, llvm_t));
        ir.push_str(&format!("  store {} %{}, {}* {}\n", llvm_t, param_counter, llvm_t, param_alloca));
        var_types.insert(param_name.clone(), (param_alloca, llvm_t));
        param_counter += 1;
    }

    let mut temp_counter = 0;
    for stmt in &method.body {
        ir.push_str(&generate_statement(program, stmt, &mut temp_counter, &mut var_types, string_counter, strings, Some(class)));
    }

    // Если это конструктор и нет return в конце - добавляем
    if method.is_constructor {
        let has_return = method.body.iter().any(|s| matches!(s, Statement::Return { .. }));
        if !has_return {
            let (this_name, this_t) = var_types.get("this").unwrap();
            ir.push_str(&format!("  %ret = load {}, {}* {}\n", this_t, this_t, this_name));
            ir.push_str(&format!("  ret {} %ret\n", this_t));
        }
    } else if method.return_type == Type::Void {
        let has_return = method.body.iter().any(|s| matches!(s, Statement::Return { .. }));
        if !has_return {
            ir.push_str("  ret void\n");
        }
    }

    ir.push_str("}\n\n");
    ir
}

fn generate_function(program: &Program, func: &Function, string_counter: &mut usize, strings: &mut String) -> String {
    let mut ir = String::new();
    let mut var_types: HashMap<String, (String, String)> = HashMap::new();

    let ret_type = llvm_type(&func.return_type);
    ir.push_str(&format!("define {} @{}(", ret_type, func.name));
    for (i, (_, t)) in func.params.iter().enumerate() {
        if i > 0 { ir.push_str(", "); }
        ir.push_str(&llvm_type(t));
    }
    ir.push_str(") {\n");

    let mut param_counter = 0;
    for (name, t) in &func.params {
        let llvm_t = llvm_type(t);
        let param_alloca = format!("%{}_{}", name, *string_counter);
        *string_counter += 1;
        ir.push_str(&format!("  {} = alloca {}\n", param_alloca, llvm_t));
        ir.push_str(&format!("  store {} %{}, {}* {}\n", llvm_t, param_counter, llvm_t, param_alloca));
        var_types.insert(name.clone(), (param_alloca, llvm_t));
        param_counter += 1;
    }

    let mut temp_counter = 0;
    for stmt in &func.body {
        ir.push_str(&generate_statement(program, stmt, &mut temp_counter, &mut var_types, string_counter, strings, None));
    }

    if func.return_type == Type::Void {
        ir.push_str("  ret void\n");
    }
    ir.push_str("}\n\n");
    ir
}

fn generate_statement(program: &Program, stmt: &Statement, temp_counter: &mut usize, var_types: &mut HashMap<String, (String, String)>, string_counter: &mut usize, strings: &mut String, current_class: Option<&Class>) -> String {
    match stmt {
        Statement::Let { name, type_, value } => {
            let (val_ir, val_reg, val_type) = generate_expression(program, value, temp_counter, string_counter, strings, var_types, current_class);

            let llvm_t = match &type_ {
                Some(t) => llvm_type(t),
                None => val_type.clone(),
            };

            let var_alloca = format!("%{}_{}", name, *string_counter);
            *string_counter += 1;
            let alloca = format!("  {} = alloca {}\n", var_alloca, llvm_t);
            let store = format!("  store {} {}, {}* {}\n", val_type, val_reg, llvm_t, var_alloca);
            var_types.insert(name.clone(), (var_alloca, llvm_t));
            format!("{}{}{}", val_ir, alloca, store)
        }
        Statement::Return { value } => {
            match value {
                Some(expr) => {
                    let (val_ir, val_reg, val_type) = generate_expression(program, expr, temp_counter, string_counter, strings, var_types, current_class);
                    format!("{}  ret {} {}\n", val_ir, val_type, val_reg)
                }
                None => {
                    "  ret void\n".to_string()
                }
            }
        }
        Statement::Expr(expr) => {
            let (ir, _, _) = generate_expression(program, expr, temp_counter, string_counter, strings, var_types, current_class);
            ir
        }
        Statement::Println { values } => {
            let mut ir = String::new();
            let mut regs = Vec::new();
            let mut types = Vec::new();
            for value in values {
                let (val_ir, val_reg, val_type) = generate_expression(program, value, temp_counter, string_counter, strings, var_types, current_class);
                ir.push_str(&val_ir);
                regs.push(val_reg);
                types.push(val_type);
            }
            let fmt_parts: Vec<String> = types.iter().map(|t| if t == "i8*" { "%s".to_string() } else { "%lld".to_string() }).collect();
            let fmt_str = fmt_parts.join(" ");
            let fmt_len = fmt_str.len() + 2;

            let fmt_num = *temp_counter;
            *temp_counter += 1;
            let fmt_name = format!("%fmt{}", fmt_num);
            let fmt_ptr_name = format!("%fmt_ptr{}", fmt_num);

            let fmt_alloca = format!("  {} = alloca [{} x i8]\n", fmt_name, fmt_len);
            let fmt_store = format!("  store [{} x i8] c\"{}\\0A\\00\", [{} x i8]* {}\n", fmt_len, fmt_str, fmt_len, fmt_name);
            let fmt_ptr = format!("  {} = getelementptr [{} x i8], [{} x i8]* {}, i32 0, i32 0\n", fmt_ptr_name, fmt_len, fmt_len, fmt_name);
            let args: Vec<String> = regs.iter().zip(types.iter()).map(|(r, t)| format!("{} {}", t, r)).collect();
            let call_ir = format!("  call i32 (i8*, ...) @printf(i8* {}, {})\n", fmt_ptr_name, args.join(", "));

            ir.push_str(&fmt_alloca);
            ir.push_str(&fmt_store);
            ir.push_str(&fmt_ptr);
            ir.push_str(&call_ir);
            ir
        }
        Statement::If { condition, then_body, else_body } => {
            let (cond_ir, cond_reg, _) = generate_expression(program, condition, temp_counter, string_counter, strings, var_types, current_class);
            let mut ir = cond_ir;
            let then_label = format!(".then{}", *temp_counter);
            let else_label = format!(".else{}", *temp_counter);
            let end_label = format!(".endif{}", *temp_counter);
            *temp_counter += 1;
            let cmp_reg = format!("%cmp{}", *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = icmp ne i128 {}, 0\n", cmp_reg, cond_reg));
            ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_reg, then_label, else_label));

            ir.push_str(&format!("{}:\n", then_label));
            let then_always_returns = block_always_returns(then_body);
            for stmt in then_body {
                ir.push_str(&generate_statement(program, stmt, temp_counter, var_types, string_counter, strings, current_class));
            }
            if !then_always_returns {
                ir.push_str(&format!("  br label %{}\n", end_label));
            }

            ir.push_str(&format!("{}:\n", else_label));
            let else_always_returns = else_body.as_ref().map(|b| block_always_returns(b)).unwrap_or(false);
            if let Some(else_body) = else_body {
                for stmt in else_body {
                    ir.push_str(&generate_statement(program, stmt, temp_counter, var_types, string_counter, strings, current_class));
                }
            }
            if !else_always_returns {
                ir.push_str(&format!("  br label %{}\n", end_label));
            }

            if !then_always_returns || !else_always_returns {
                ir.push_str(&format!("{}:\n", end_label));
            }

            ir
        }
        Statement::While { condition, body } => {
            let mut ir = String::new();

            let start_label = format!(".while_cond{}", *temp_counter);
            let body_label = format!(".while_body{}", *temp_counter);
            let end_label = format!(".while_end{}", *temp_counter);
            *temp_counter += 1;

            ir.push_str(&format!("  br label %{}\n", start_label));
            ir.push_str(&format!("{}:\n", start_label));

            let (cond_ir, cond_reg, _) = generate_expression(program, condition, temp_counter, string_counter, strings, var_types, current_class);
            ir.push_str(&cond_ir);

            let cmp_reg = format!("%cmp{}", *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = icmp ne i128 {}, 0\n", cmp_reg, cond_reg));
            ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_reg, body_label, end_label));

            ir.push_str(&format!("{}:\n", body_label));
            for stmt in body {
                ir.push_str(&generate_statement(program, stmt, temp_counter, var_types, string_counter, strings, current_class));
            }
            ir.push_str(&format!("  br label %{}\n", start_label));

            ir.push_str(&format!("{}:\n", end_label));

            ir
        }
        Statement::For { init, condition, increment, body } => {
            let mut ir = String::new();

            ir.push_str(&generate_statement(program, init, temp_counter, var_types, string_counter, strings, current_class));

            let start_label = format!(".for_cond{}", *temp_counter);
            let body_label = format!(".for_body{}", *temp_counter);
            let end_label = format!(".for_end{}", *temp_counter);
            *temp_counter += 1;

            ir.push_str(&format!("  br label %{}\n", start_label));
            ir.push_str(&format!("{}:\n", start_label));

            let (cond_ir, cond_reg, _) = generate_expression(program, condition, temp_counter, string_counter, strings, var_types, current_class);
            ir.push_str(&cond_ir);

            let cmp_reg = format!("%cmp{}", *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = icmp ne i128 {}, 0\n", cmp_reg, cond_reg));
            ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_reg, body_label, end_label));

            ir.push_str(&format!("{}:\n", body_label));
            for stmt in body {
                ir.push_str(&generate_statement(program, stmt, temp_counter, var_types, string_counter, strings, current_class));
            }

            ir.push_str(&generate_statement(program, increment, temp_counter, var_types, string_counter, strings, current_class));
            ir.push_str(&format!("  br label %{}\n", start_label));
            ir.push_str(&format!("{}:\n", end_label));

            ir
        }
    }
}

fn get_struct_size(class_name: &str, program: &Program) -> usize {
    let class_def = program.classes.iter().find(|c| c.name == class_name).unwrap();
    let mut offset = 0;
    let mut max_align = 8;

    for field in &class_def.fields {
        let size = match field.type_ {
            Type::Int => 16,
            Type::String => 8,
            Type::Class(_) => 8,
            Type::Void => 0,
        };
        let align = match field.type_ {
            Type::Int => 16,
            _ => 8,
        };
        max_align = max_align.max(align);

        if offset % align != 0 {
            offset += align - (offset % align);
        }
        offset += size;
    }

    if offset % max_align != 0 {
        offset += max_align - (offset % max_align);
    }

    offset
}

fn block_always_returns(body: &[Statement]) -> bool {
    if let Some(last) = body.last() {
        match last {
            Statement::Return { .. } => true,
            Statement::If { then_body, else_body, .. } => {
                let then_always = block_always_returns(then_body);
                let else_always = else_body.as_ref().map(|b| block_always_returns(b)).unwrap_or(false);
                else_body.is_some() && then_always && else_always
            }
            _ => false,
        }
    } else {
        false
    }
}

fn generate_expression(program: &Program, expr: &Expression, temp_counter: &mut usize, string_counter: &mut usize, strings: &mut String, var_types: &HashMap<String, (String, String)>, current_class: Option<&Class>) -> (String, String, String) {
    match expr {
        Expression::Number(n) => {
            let num_str = n.to_string();
            (String::new(), num_str, "i128".to_string())
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
        Expression::Variable(name) => {
            let (var_name, var_type) = var_types.get(name).cloned().unwrap_or_else(|| {
                panic!("Variable '{}' not found", name);
            });
            let reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;
            let ir = format!("  {} = load {}, {}* {}\n", reg, var_type, var_type, var_name);
            (ir, reg, var_type)
        }
        Expression::This => {
            let (this_name, this_type) = var_types.get("this").cloned().unwrap_or_else(|| {
                panic!("'this' used outside of method");
            });
            let reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;
            let ir = format!("  {} = load {}, {}* {}\n", reg, this_type, this_type, this_name);
            (ir, reg, this_type)
        }
        Expression::BinaryOp { left, op, right } => {
            let (left_ir, left_reg, left_type) = generate_expression(program, left, temp_counter, string_counter, strings, var_types, current_class);
            let (right_ir, right_reg, right_type) = generate_expression(program, right, temp_counter, string_counter, strings, var_types, current_class);
            let mut ir = format!("{}{}", left_ir, right_ir);

            if matches!(op, BinaryOperator::Add) {
                if left_type == "i8*" && right_type == "i8*" {
                    let len_left = format!("%len_left{}", *temp_counter);
                    *temp_counter += 1;
                    let len_right = format!("%len_right{}", *temp_counter);
                    *temp_counter += 1;
                    let total_len = format!("%total_len{}", *temp_counter);
                    *temp_counter += 1;
                    let total_len_with_null = format!("%total_len_with_null{}", *temp_counter);
                    *temp_counter += 1;
                    let ptr = format!("%concat_ptr{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len_left, left_reg));
                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len_right, right_reg));
                    ir.push_str(&format!("  {} = add i64 {}, {}\n", total_len, len_left, len_right));
                    ir.push_str(&format!("  {} = add i64 {}, 1\n", total_len_with_null, total_len));
                    ir.push_str(&format!("  {} = call i8* @malloc(i64 {})\n", ptr, total_len_with_null));
                    ir.push_str(&format!("  call i8* @strcpy(i8* {}, i8* {})\n", ptr, left_reg));
                    ir.push_str(&format!("  call i8* @strcat(i8* {}, i8* {})\n", ptr, right_reg));
                    return (ir, ptr, "i8*".to_string());
                }

                if left_type == "i128" && right_type == "i8*" {
                    let num_str = format!("%num_str{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  {} = alloca i8, i64 39\n", num_str));
                    ir.push_str(&format!("  call i32 (i8*, i8*, ...) @sprintf(i8* {}, i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str.num_fmt, i32 0, i32 0), i128 {})\n", num_str, left_reg));

                    let len_left = format!("%len_left{}", *temp_counter);
                    *temp_counter += 1;
                    let len_right = format!("%len_right{}", *temp_counter);
                    *temp_counter += 1;
                    let total_len = format!("%total_len{}", *temp_counter);
                    *temp_counter += 1;
                    let total_len_with_null = format!("%total_len_with_null{}", *temp_counter);
                    *temp_counter += 1;
                    let ptr = format!("%concat_ptr{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len_left, num_str));
                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len_right, right_reg));
                    ir.push_str(&format!("  {} = add i64 {}, {}\n", total_len, len_left, len_right));
                    ir.push_str(&format!("  {} = add i64 {}, 1\n", total_len_with_null, total_len));
                    ir.push_str(&format!("  {} = call i8* @malloc(i64 {})\n", ptr, total_len_with_null));
                    ir.push_str(&format!("  call i8* @strcpy(i8* {}, i8* {})\n", ptr, num_str));
                    ir.push_str(&format!("  call i8* @strcat(i8* {}, i8* {})\n", ptr, right_reg));
                    return (ir, ptr, "i8*".to_string());
                }

                if left_type == "i8*" && right_type == "i128" {
                    let num_str = format!("%num_str{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  {} = alloca i8, i64 39\n", num_str));
                    ir.push_str(&format!("  call i32 (i8*, i8*, ...) @sprintf(i8* {}, i8* getelementptr inbounds ([4 x i8], [4 x i8]* @.str.num_fmt, i32 0, i32 0), i128 {})\n", num_str, right_reg));

                    let len_left = format!("%len_left{}", *temp_counter);
                    *temp_counter += 1;
                    let len_right = format!("%len_right{}", *temp_counter);
                    *temp_counter += 1;
                    let total_len = format!("%total_len{}", *temp_counter);
                    *temp_counter += 1;
                    let total_len_with_null = format!("%total_len_with_null{}", *temp_counter);
                    *temp_counter += 1;
                    let ptr = format!("%concat_ptr{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len_left, left_reg));
                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len_right, num_str));
                    ir.push_str(&format!("  {} = add i64 {}, {}\n", total_len, len_left, len_right));
                    ir.push_str(&format!("  {} = add i64 {}, 1\n", total_len_with_null, total_len));
                    ir.push_str(&format!("  {} = call i8* @malloc(i64 {})\n", ptr, total_len_with_null));
                    ir.push_str(&format!("  call i8* @strcpy(i8* {}, i8* {})\n", ptr, left_reg));
                    ir.push_str(&format!("  call i8* @strcat(i8* {}, i8* {})\n", ptr, num_str));
                    return (ir, ptr, "i8*".to_string());
                }
            }

            if (left_type == "i8*" && right_type == "i8*") && matches!(op, BinaryOperator::Eq | BinaryOperator::Ne) {
                let cmp_reg = format!("%cmp{}", *temp_counter);
                *temp_counter += 1;
                ir.push_str(&format!("  {} = call i32 @strcmp(i8* {}, i8* {})\n", cmp_reg, left_reg, right_reg));
                let result_reg = format!("%t{}", *temp_counter);
                *temp_counter += 1;
                match op {
                    BinaryOperator::Eq => {
                        let eq_reg = format!("%eq{}", *temp_counter);
                        *temp_counter += 1;
                        ir.push_str(&format!("  {} = icmp eq i32 {}, 0\n", eq_reg, cmp_reg));
                        ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, eq_reg));
                    }
                    BinaryOperator::Ne => {
                        let ne_reg = format!("%ne{}", *temp_counter);
                        *temp_counter += 1;
                        ir.push_str(&format!("  {} = icmp ne i32 {}, 0\n", ne_reg, cmp_reg));
                        ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, ne_reg));
                    }
                    _ => {}
                }
                return (ir, result_reg, "i128".to_string());
            }

            let result_reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;
            match op {
                BinaryOperator::Add => ir.push_str(&format!("  {} = add i128 {}, {}\n", result_reg, left_reg, right_reg)),
                BinaryOperator::Sub => ir.push_str(&format!("  {} = sub i128 {}, {}\n", result_reg, left_reg, right_reg)),
                BinaryOperator::Mul => ir.push_str(&format!("  {} = mul i128 {}, {}\n", result_reg, left_reg, right_reg)),
                BinaryOperator::Div => ir.push_str(&format!("  {} = sdiv i128 {}, {}\n", result_reg, left_reg, right_reg)),
                BinaryOperator::Eq => {
                    let cmp = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp eq i128 {}, {}\n", cmp, left_reg, right_reg));
                    ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, cmp));
                }
                BinaryOperator::Ne => {
                    let cmp = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp ne i128 {}, {}\n", cmp, left_reg, right_reg));
                    ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, cmp));
                }
                BinaryOperator::Lt => {
                    let cmp = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp slt i128 {}, {}\n", cmp, left_reg, right_reg));
                    ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, cmp));
                }
                BinaryOperator::Le => {
                    let cmp = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp sle i128 {}, {}\n", cmp, left_reg, right_reg));
                    ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, cmp));
                }
                BinaryOperator::Gt => {
                    let cmp = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp sgt i128 {}, {}\n", cmp, left_reg, right_reg));
                    ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, cmp));
                }
                BinaryOperator::Ge => {
                    let cmp = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp sge i128 {}, {}\n", cmp, left_reg, right_reg));
                    ir.push_str(&format!("  {} = zext i1 {} to i128\n", result_reg, cmp));
                }
            }
            (ir, result_reg, "i128".to_string())
        }
        Expression::Call { func, args } => {
            let mut ir = String::new();
            let mut arg_regs = Vec::new();
            let mut arg_types = Vec::new();

            let func_def = program.functions.iter().find(|f| f.name == *func);

            for arg in args {
                let (arg_ir, arg_reg, arg_type) = generate_expression(program, arg, temp_counter, string_counter, strings, var_types, current_class);
                ir.push_str(&arg_ir);
                arg_regs.push(arg_reg);
                arg_types.push(arg_type);
            }
            let args_str: Vec<String> = arg_regs.iter().zip(arg_types.iter()).map(|(r, t)| format!("{} {}", t, r)).collect();

            if let Some(f) = func_def {
                let ret_type = llvm_type(&f.return_type);
                if f.return_type == Type::Void {
                    ir.push_str(&format!("  call {} @{}({})\n", ret_type, func, args_str.join(", ")));
                    (ir, "".to_string(), ret_type)
                } else {
                    let call_reg = format!("%t{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = call {} @{}({})\n", call_reg, ret_type, func, args_str.join(", ")));
                    (ir, call_reg, ret_type)
                }
            } else {
                let call_reg = format!("%t{}", *temp_counter);
                *temp_counter += 1;
                ir.push_str(&format!("  {} = call i128 @{}({})\n", call_reg, func, args_str.join(", ")));
                (ir, call_reg, "i128".to_string())
            }
        }
        Expression::MethodCall { object, method, args } => {
            let (obj_ir, obj_reg, obj_type) = generate_expression(program, object, temp_counter, string_counter, strings, var_types, current_class);
            let mut ir = obj_ir;
            let mut arg_regs = Vec::new();
            let mut arg_types = Vec::new();
            arg_regs.push(obj_reg.clone());
            arg_types.push(obj_type.clone());
            for arg in args {
                let (arg_ir, arg_reg, arg_type) = generate_expression(program, arg, temp_counter, string_counter, strings, var_types, current_class);
                ir.push_str(&arg_ir);
                arg_regs.push(arg_reg);
                arg_types.push(arg_type);
            }
            let class_name = obj_type.trim_start_matches('%').trim_end_matches('*').to_string();
            let class_def = program.classes.iter().find(|c| c.name == class_name).expect("Class not found");
            let method_def = class_def.methods.iter().find(|m| m.name == *method).expect("Method not found");
            if !method_def.is_public {
                let is_same_class = match current_class {
                    Some(cur) => cur.name == class_name,
                    None => false,
                };
                if !is_same_class {
                    panic!("Method '{}.{}' is private and cannot be called from outside the class", class_name, method);
                }
            }
            let llvm_method = format!("{}_{}", class_name, method);
            let args_str: Vec<String> = arg_regs.iter().zip(arg_types.iter()).map(|(r, t)| format!("{} {}", t, r)).collect();
            let ret_type = llvm_type(&method_def.return_type);

            if method_def.return_type == Type::Void {
                ir.push_str(&format!("  call {} @{}({})\n", ret_type, llvm_method, args_str.join(", ")));
                (ir, "".to_string(), ret_type)
            } else {
                let call_reg = format!("%t{}", *temp_counter);
                *temp_counter += 1;
                ir.push_str(&format!("  {} = call {} @{}({})\n", call_reg, ret_type, llvm_method, args_str.join(", ")));
                (ir, call_reg, ret_type)
            }
        }
        Expression::New { class, args } => {
            let mut ir = String::new();
            let struct_size = get_struct_size(class, program);

            let size_reg = format!("%{}_size_{}", class, *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = mul i64 {}, 1\n", size_reg, struct_size));

            let ptr_reg = format!("%{}_ptr_{}", class, *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = call i8* @malloc(i64 {})\n", ptr_reg, size_reg));

            let obj_ptr = format!("%{}_obj_{}", class, *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = bitcast i8* {} to %{}*\n", obj_ptr, ptr_reg, class));

            // Вызываем конструктор
            let constructor_name = format!("{}_{}", class, class);
            let init_reg = format!("%{}_init_{}", class, *temp_counter);
            *temp_counter += 1;

            // Формируем аргументы для конструктора (первый аргумент - this, затем поля)
            let mut constructor_args = vec![format!("%{}* {}", class, obj_ptr)];
            for arg in args {
                let (arg_ir, arg_reg, arg_type) = generate_expression(program, arg, temp_counter, string_counter, strings, var_types, current_class);
                ir.push_str(&arg_ir);
                constructor_args.push(format!("{} {}", arg_type, arg_reg));
            }

            ir.push_str(&format!("  {} = call %{}* @{}({})\n",
                                 init_reg, class, constructor_name, constructor_args.join(", ")));

            (ir, init_reg, format!("%{}*", class))
        }
        Expression::FieldAccess { object, field } => {
            let (obj_ir, obj_reg, obj_type) = generate_expression(program, object, temp_counter, string_counter, strings, var_types, current_class);
            let mut ir = obj_ir;

            eprintln!("[DEBUG] FieldAccess: object_type={}, field={}", obj_type, field);

            let class_name = obj_type.trim_start_matches('%').trim_end_matches('*').to_string();
            eprintln!("[DEBUG] Class name from type: {}", class_name);

            let class_def = program.classes.iter().find(|c| c.name == class_name).expect(&format!("Class '{}' not found", class_name));

            // Находим индекс поля
            let index = match class_def.fields.iter().position(|f| f.name == *field) {
                Some(idx) => idx,
                None => {
                    eprintln!("[DEBUG] Available fields in class '{}':", class_name);
                    for (i, f) in class_def.fields.iter().enumerate() {
                        eprintln!("  [{}] {} ({:?})", i, f.name, f.type_);
                    }
                    panic!("Field '{}' not found in class '{}'", field, class_name);
                }
            };

            let field_type = llvm_type(&class_def.fields[index].type_);
            eprintln!("[DEBUG] Field index: {}, type: {}", index, field_type);

            let field_ptr = format!("%field_ptr_{}_{}_{}", class_name, field, *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = getelementptr %{}, %{}* {}, i32 0, i32 {}\n",
                                 field_ptr, class_name, class_name, obj_reg, index));

            let val_reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;
            ir.push_str(&format!("  {} = load {}, {}* {}\n", val_reg, field_type, field_type, field_ptr));
            (ir, val_reg, field_type)
        }
        Expression::Assignment { left, right } => {
            let (right_ir, right_reg, right_type) = generate_expression(program, right, temp_counter, string_counter, strings, var_types, current_class);
            let mut ir = right_ir;
            match left.as_ref() {
                Expression::Variable(name) => {
                    let (var_name, var_type) = var_types.get(name).cloned().unwrap_or_else(|| {
                        panic!("Variable '{}' not found", name);
                    });
                    ir.push_str(&format!("  store {} {}, {}* {}\n", right_type, right_reg, var_type, var_name));
                    (ir, right_reg, right_type)
                }
                Expression::FieldAccess { object, field } => {
                    let (obj_ir, obj_reg, obj_type) = generate_expression(program, object, temp_counter, string_counter, strings, var_types, current_class);
                    ir.push_str(&obj_ir);

                    eprintln!("[DEBUG] Assignment FieldAccess: object_type={}, field={}", obj_type, field);

                    let class_name = obj_type.trim_start_matches('%').trim_end_matches('*').to_string();
                    let class_def = program.classes.iter().find(|c| c.name == class_name).expect(&format!("Class '{}' not found", class_name));

                    let index = match class_def.fields.iter().position(|f| f.name == *field) {
                        Some(idx) => idx,
                        None => {
                            eprintln!("[DEBUG] Available fields in class '{}':", class_name);
                            for (i, f) in class_def.fields.iter().enumerate() {
                                eprintln!("  [{}] {} ({:?})", i, f.name, f.type_);
                            }
                            panic!("Field '{}' not found in class '{}'", field, class_name);
                        }
                    };
                    let field_type = llvm_type(&class_def.fields[index].type_);

                    let field_ptr = format!("%field_ptr_{}_{}_{}", class_name, field, *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = getelementptr %{}, %{}* {}, i32 0, i32 {}\n",
                                         field_ptr, class_name, class_name, obj_reg, index));
                    ir.push_str(&format!("  store {} {}, {}* {}\n", right_type, right_reg, field_type, field_ptr));
                    (ir, right_reg, right_type)
                }
                _ => panic!("Left side of assignment must be a variable or field access"),
            }
        }
        Expression::Input(input_type) => {
            let mut ir = String::new();
            let result_reg = format!("%t{}", *temp_counter);
            *temp_counter += 1;

            match input_type {
                InputType::String => {
                    let buf_reg = format!("%buf{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = alloca i8, i64 256\n", buf_reg));

                    // Уникальное имя для stdin_ptr
                    let stdin_ptr = format!("%stdin_ptr_{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = load %struct._IO_FILE*, %struct._IO_FILE** @stdin\n", stdin_ptr));
                    ir.push_str(&format!("  call i8* @fgets(i8* {}, i32 256, i8* {})\n", buf_reg, stdin_ptr));

                    // Убираем newline
                    let len = format!("%len{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len, buf_reg));

                    let cond = format!("%cond{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp ne i64 {}, 0\n", cond, len));

                    let if_label = format!(".if{}", *temp_counter);
                    let end_label = format!(".end{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cond, if_label, end_label));

                    ir.push_str(&format!("{}:\n", if_label));
                    let idx = format!("%idx{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = sub i64 {}, 1\n", idx, len));
                    let ptr = format!("%ptr{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = getelementptr i8, i8* {}, i64 {}\n", ptr, buf_reg, idx));
                    ir.push_str(&format!("  store i8 0, i8* {}\n", ptr));
                    ir.push_str(&format!("  br label %{}\n", end_label));

                    ir.push_str(&format!("{}:\n", end_label));
                    (ir, buf_reg, "i8*".to_string())
                }

                InputType::Int => {
                    let buf_reg = format!("%buf{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = alloca i8, i64 256\n", buf_reg));

                    // Уникальное имя для stdin_ptr
                    let stdin_ptr = format!("%stdin_ptr_{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = load %struct._IO_FILE*, %struct._IO_FILE** @stdin\n", stdin_ptr));
                    ir.push_str(&format!("  call i8* @fgets(i8* {}, i32 256, i8* {})\n", buf_reg, stdin_ptr));

                    // Убираем newline
                    let len = format!("%len{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = call i64 @strlen(i8* {})\n", len, buf_reg));

                    let cond = format!("%cond{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp ne i64 {}, 0\n", cond, len));

                    let if_label = format!(".if{}", *temp_counter);
                    let end_label = format!(".end{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cond, if_label, end_label));

                    ir.push_str(&format!("{}:\n", if_label));
                    let idx = format!("%idx{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = sub i64 {}, 1\n", idx, len));
                    let ptr = format!("%ptr{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = getelementptr i8, i8* {}, i64 {}\n", ptr, buf_reg, idx));
                    ir.push_str(&format!("  store i8 0, i8* {}\n", ptr));
                    ir.push_str(&format!("  br label %{}\n", end_label));

                    ir.push_str(&format!("{}:\n", end_label));

                    // Конвертируем строку в число
                    let end_ptr = format!("%end_ptr{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = alloca i8*\n", end_ptr));
                    ir.push_str(&format!("  store i8* null, i8** {}\n", end_ptr));

                    let num = format!("%num{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = call i64 @strtol(i8* {}, i8** {}, i32 10)\n", num, buf_reg, end_ptr));

                    // Проверка на ошибку
                    let end_val = format!("%end_val{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = load i8*, i8** {}\n", end_val, end_ptr));

                    let valid = format!("%valid{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = icmp eq i8* {}, {}\n", valid, end_val, buf_reg));

                    let error_label = format!(".err{}", *temp_counter);
                    let ok_label = format!(".ok{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", valid, error_label, ok_label));

                    ir.push_str(&format!("{}:\n", error_label));
                    ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([25 x i8], [25 x i8]* @.input_error_int, i32 0, i32 0))\n"));
                    ir.push_str(&format!("  call void @exit(i32 1)\n"));
                    ir.push_str(&format!("  unreachable\n"));

                    ir.push_str(&format!("{}:\n", ok_label));
                    let conv = format!("%conv{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = sext i64 {} to i128\n", conv, num));
                    (ir, conv, "i128".to_string())
                }

                InputType::Bool => {
                    let buf_reg = format!("%buf{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = alloca i8, i64 256\n", buf_reg));

                    ir.push_str(&format!("  call i32 (i8*, ...) @scanf(i8* getelementptr inbounds ([8 x i8], [8 x i8]* @.str.string_fmt_clean, i32 0, i32 0), i8* {})\n", buf_reg));

                    let clear_ch = format!("%clear_ch{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = call i32 @getchar()\n", clear_ch));

                    let cmp_true = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    let cmp_false = format!("%cmp{}", *temp_counter);
                    *temp_counter += 1;
                    let true_label = format!(".bool_true{}", *temp_counter);
                    let false_label = format!(".bool_false{}", *temp_counter);
                    let false_ok_label = format!(".bool_false_ok{}", *temp_counter);
                    let error_label = format!(".bool_error{}", *temp_counter);
                    let end_label = format!(".bool_end{}", *temp_counter);
                    *temp_counter += 1;
                    let alloca_reg = format!("%bool_storage{}", *temp_counter);
                    *temp_counter += 1;

                    ir.push_str(&format!("  {} = alloca i32\n", alloca_reg));
                    ir.push_str(&format!("  {} = call i32 @strcmp(i8* {}, i8* getelementptr inbounds ([5 x i8], [5 x i8]* @.str.true_val, i32 0, i32 0))\n", cmp_true, buf_reg));
                    ir.push_str(&format!("  {} = icmp eq i32 {}, 0\n", cmp_true, cmp_true));
                    ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_true, true_label, false_label));

                    ir.push_str(&format!("{}:\n", true_label));
                    ir.push_str(&format!("  store i32 1, i32* {}\n", alloca_reg));
                    ir.push_str(&format!("  br label %{}\n", end_label));

                    ir.push_str(&format!("{}:\n", false_label));
                    ir.push_str(&format!("  {} = call i32 @strcmp(i8* {}, i8* getelementptr inbounds ([6 x i8], [6 x i8]* @.str.false_val, i32 0, i32 0))\n", cmp_false, buf_reg));
                    ir.push_str(&format!("  {} = icmp eq i32 {}, 0\n", cmp_false, cmp_false));
                    ir.push_str(&format!("  br i1 {}, label %{}, label %{}\n", cmp_false, false_ok_label, error_label));

                    ir.push_str(&format!("{}:\n", false_ok_label));
                    ir.push_str(&format!("  store i32 0, i32* {}\n", alloca_reg));
                    ir.push_str(&format!("  br label %{}\n", end_label));

                    ir.push_str(&format!("{}:\n", error_label));
                    ir.push_str(&format!("  call i32 (i8*, ...) @printf(i8* getelementptr inbounds ([31 x i8], [31 x i8]* @.input_error_bool, i32 0, i32 0))\n"));
                    ir.push_str(&format!("  call void @exit(i32 1)\n"));
                    ir.push_str(&format!("  unreachable\n"));

                    ir.push_str(&format!("{}:\n", end_label));
                    ir.push_str(&format!("  {} = load i32, i32* {}\n", buf_reg, alloca_reg));
                    let conv_reg = format!("%t{}", *temp_counter);
                    *temp_counter += 1;
                    ir.push_str(&format!("  {} = sext i32 {} to i128\n", conv_reg, buf_reg));
                    (ir, conv_reg, "i128".to_string())
                }
            }
        }
    }
}

fn llvm_type(t: &Type) -> String {
    match t {
        Type::Int => "i128".to_string(),
        Type::String => "i8*".to_string(),
        Type::Void => "void".to_string(),
        Type::Class(name) => format!("%{}*", name),
    }
}