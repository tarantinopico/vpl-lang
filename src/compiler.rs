#![allow(warnings)]
use crate::parser::{Stmt, Expr};
use std::collections::HashSet;

pub fn compile(stmts: &[Stmt]) -> (String, Vec<String>) {
    let mut code = String::new();
    let mut used_builtins = HashSet::new();
    
    analyze_usage(stmts, &mut used_builtins);

    code.push_str("#![allow(warnings)]\n");
    let (runtime_code, modules) = generate_runtime(&used_builtins);
    code.push_str(runtime_code.as_str());

    let mut main_stmts = Vec::new();
    let mut functions = Vec::new();

    for stmt in stmts {
        if let Stmt::Func(..) = stmt { functions.push(stmt.clone()); }
        else { main_stmts.push(stmt.clone()); }
    }

    let mut has_vpl_main = false;
    for func in &functions {
        if let Stmt::Func(name, ..) = func {
            if name == "main" { has_vpl_main = true; }
        }
    }

    code.push_str("fn main() {\n    vpl_main();\n}\n\n");
    code.push_str("fn vpl_main() -> Value {\n");
    compile_scope_with_params(&main_stmts, &mut code, HashSet::new());
    if has_vpl_main { code.push_str("    vpl_fn_main()\n"); }
    else { code.push_str("    Value::None\n"); }
    code.push_str("}\n\n");

    for func in functions {
        if let Stmt::Func(name, params, body) = func {
            code.push_str(&format!("fn vpl_fn_{}(", name));
            for (i, p) in params.iter().enumerate() {
                if i > 0 { code.push_str(", "); }
                code.push_str(&format!("mut v_{}: Value", p));
            }
            code.push_str(") -> Value {\n");
            let param_set: HashSet<_> = params.iter().cloned().collect();
            compile_scope_with_params(&body, &mut code, param_set);
            code.push_str("    Value::None\n}\n\n");
        }
    }
    (code, modules)
}

fn analyze_usage(stmts: &[Stmt], used: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Set(_, expr) => analyze_expr(expr, used),
            Stmt::SetIndex(e1, e2, e3) => { analyze_expr(e1, used); analyze_expr(e2, used); analyze_expr(e3, used); }
            Stmt::Say(expr) => analyze_expr(expr, used),
            Stmt::Expr(expr) => analyze_expr(expr, used),
            Stmt::If(cond, then_b, else_b) => {
                analyze_expr(cond, used);
                analyze_usage(then_b, used);
                if let Some(eb) = else_b { analyze_usage(eb, used); }
            }
            Stmt::While(cond, body) => { analyze_expr(cond, used); analyze_usage(body, used); }
            Stmt::For(_, expr, body) => { analyze_expr(expr, used); analyze_usage(body, used); }
            Stmt::Func(_, _, body) => analyze_usage(body, used),
            Stmt::Return(expr_opt) => { if let Some(e) = expr_opt { analyze_expr(e, used); } }
        }
    }
}

fn analyze_expr(expr: &Expr, used: &mut HashSet<String>) {
    match expr {
        Expr::Binary(l, _, r) => { analyze_expr(l, used); analyze_expr(r, used); }
        Expr::Call(name, args) => {
            used.insert(name.clone());
            for a in args { analyze_expr(a, used); }
        }
        Expr::Array(items) => { 
            used.insert("arr_literal".to_string());
            for i in items { analyze_expr(i, used); } 
        }
        Expr::Index(a, i) => { 
            used.insert("arr_index".to_string());
            analyze_expr(a, used); analyze_expr(i, used); 
        }
        _ => {}
    }
}

fn compile_scope_with_params(stmts: &[Stmt], code: &mut String, params: HashSet<String>) {
    let mut vars = HashSet::new();
    find_vars(stmts, &mut vars);
    for v in vars {
        if !params.contains(&v) { code.push_str(&format!("    let mut v_{} = Value::None;\n", v)); }
    }
    for stmt in stmts { compile_stmt(stmt, code); }
}

fn find_vars(stmts: &[Stmt], vars: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::Set(name, _) => { vars.insert(name.clone()); }
            Stmt::If(_, then_b, else_b) => {
                find_vars(then_b, vars);
                if let Some(eb) = else_b { find_vars(eb, vars); }
            }
            Stmt::While(_, body) => { find_vars(body, vars); }
            Stmt::For(var, _, body) => { vars.insert(var.clone()); find_vars(body, vars); }
            _ => {}
        }
    }
}

fn compile_stmt(stmt: &Stmt, code: &mut String) {
    match stmt {
        Stmt::Set(name, expr) => {
            let expr_str = compile_expr(expr);
            code.push_str(&format!("    v_{} = {};\n", name, expr_str));
        }
        Stmt::SetIndex(arr_expr, idx_expr, val_expr) => {
            let a = compile_expr(arr_expr);
            let i = compile_expr(idx_expr);
            let v = compile_expr(val_expr);
            code.push_str(&format!("    {}.set_index({}, {});\n", a, i, v));
        }
        Stmt::Say(expr) => {
            let expr_str = compile_expr(expr);
            code.push_str(&format!("    println!(\"{{}}\", {});\n", expr_str));
            code.push_str("    std::io::Write::flush(&mut std::io::stdout()).ok();\n");
        }
        Stmt::Expr(expr) => {
            let expr_str = compile_expr(expr);
            code.push_str(&format!("    {};\n", expr_str));
        }
        Stmt::If(cond, then_b, else_b) => {
            let cond_str = compile_expr(cond);
            code.push_str(&format!("    if {}.is_truthy() {{\n", cond_str));
            for s in then_b { compile_stmt(s, code); }
            if let Some(eb) = else_b {
                code.push_str("    } else {\n");
                for s in eb { compile_stmt(s, code); }
            }
            code.push_str("    }\n");
        }
        Stmt::While(cond, body) => {
            let cond_str = compile_expr(cond);
            code.push_str(&format!("    while {}.is_truthy() {{\n", cond_str));
            for s in body { compile_stmt(s, code); }
            code.push_str("    }\n");
        }
        Stmt::For(var, expr, body) => {
            let expr_str = compile_expr(expr);
            code.push_str(&format!("    if let Value::Array(a_vpl) = {} {{\n", expr_str));
            code.push_str("        for item_vpl in a_vpl.borrow().iter() {\n");
            code.push_str(&format!("            v_{} = item_vpl.clone();\n", var));
            for s in body { compile_stmt(s, code); }
            code.push_str("        }\n");
            code.push_str("    }\n");
        }
        Stmt::Return(expr_opt) => {
            if let Some(expr) = expr_opt {
                let expr_str = compile_expr(expr);
                code.push_str(&format!("    return {};\n", expr_str));
            } else {
                code.push_str("    return Value::None;\n");
            }
        }
        _ => {}
    }
}

fn compile_expr(expr: &Expr) -> String {
    match expr {
        Expr::Num(n) => format!("Value::Num({})", n),
        Expr::Str(s) => format!("Value::Str(\"{}\".to_string())", s.replace("\"", "\\\"").replace("\n", "\\n")),
        Expr::Ident(name) => format!("v_{}.clone()", name),
        Expr::Array(items) => {
            let mut s = "Value::Array(std::rc::Rc::new(std::cell::RefCell::new(vec![".to_string();
            for (i, item) in items.iter().enumerate() {
                if i > 0 { s.push_str(", "); }
                s.push_str(&compile_expr(item));
            }
            s.push_str("])))");
            s
        }
        Expr::Index(arr, idx) => {
            let a = compile_expr(arr);
            let i = compile_expr(idx);
            format!("{}.get_index({})", a, i)
        }
        Expr::Binary(l, op, r) => {
            let ls = compile_expr(l);
            let rs = compile_expr(r);
            match op.as_str() {
                "+" => format!("({}.add(&{}))", ls, rs),
                "-" => format!("({}.sub(&{}))", ls, rs),
                "*" => format!("({}.mul(&{}))", ls, rs),
                "/" => format!("({}.div(&{}))", ls, rs),
                "==" => format!("Value::Num(if {} == {} {{ 1 }} else {{ 0 }})", ls, rs),
                "!=" => format!("Value::Num(if {} != {} {{ 1 }} else {{ 0 }})", ls, rs),
                "<" => format!("Value::Num(if {} < {} {{ 1 }} else {{ 0 }})", ls, rs),
                ">" => format!("Value::Num(if {} > {} {{ 1 }} else {{ 0 }})", ls, rs),
                "<=" => format!("Value::Num(if {} <= {} {{ 1 }} else {{ 0 }})", ls, rs),
                ">=" => format!("Value::Num(if {} >= {} {{ 1 }} else {{ 0 }})", ls, rs),
                _ => unreachable!(),
            }
        }
        Expr::Call(name, args) => {
            let mut arg_strs = Vec::new();
            for a in args { arg_strs.push(compile_expr(a)); }
            match name.as_str() {
                // TUI
                "tui_init" => "builtin_tui_init()".to_string(),
                "tui_restore" => "builtin_tui_restore()".to_string(),
                "tui_clear" => "builtin_tui_clear()".to_string(),
                "tui_hide_cursor" => "builtin_tui_hide_cursor()".to_string(),
                "tui_show_cursor" => "builtin_tui_show_cursor()".to_string(),
                "tui_set_color" => format!("builtin_tui_set_color({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "tui_reset_color" => "builtin_tui_reset_color()".to_string(),
                "tui_print" => format!("builtin_tui_print({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "tui_read_key" => "builtin_tui_read_key()".to_string(),
                "tui_delay" => format!("builtin_tui_delay({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "tui_width" => "builtin_tui_width()".to_string(),
                "tui_height" => "builtin_tui_height()".to_string(),
                "tui_read_line" => format!("builtin_tui_read_line({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "tui_set_title" => format!("builtin_tui_set_title({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "tui_bold" => format!("builtin_tui_bold({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "tui_underline" => format!("builtin_tui_underline({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "tui_beep" => "builtin_tui_beep()".to_string(),
                "tui_draw_box" => format!("builtin_tui_draw_box({}, {}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string()), arg_strs.get(3).unwrap_or(&"Value::None".to_string())),
                "tui_draw_window" => format!("builtin_tui_draw_window({}, {}, {}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string()), arg_strs.get(3).unwrap_or(&"Value::None".to_string()), arg_strs.get(4).unwrap_or(&"Value::None".to_string())),
                "tui_fill" => format!("builtin_tui_fill({}, {}, {}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string()), arg_strs.get(3).unwrap_or(&"Value::None".to_string()), arg_strs.get(4).unwrap_or(&"Value::None".to_string())),
                "tui_draw_button" => format!("builtin_tui_draw_button({}, {}, {}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string()), arg_strs.get(3).unwrap_or(&"Value::None".to_string()), arg_strs.get(4).unwrap_or(&"Value::None".to_string())),
                "tui_menu" => format!("builtin_tui_menu({}, {}, {}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string()), arg_strs.get(3).unwrap_or(&"Value::None".to_string()), arg_strs.get(4).unwrap_or(&"Value::None".to_string())),
                "tui_cursor_pos" => format!("builtin_tui_cursor_pos({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                
                // Utils
                "input" => "builtin_input()".to_string(),
                "num_to_str" => format!("builtin_num_to_str({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "str_to_num" => format!("builtin_str_to_num({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "num_to_hex" => format!("builtin_num_to_hex({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                
                // Strings
                "str_len" => format!("builtin_str_len({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "str_at" => format!("builtin_str_at({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_upper" => format!("builtin_str_upper({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "str_lower" => format!("builtin_str_lower({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "str_split" => format!("builtin_str_split({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_join" => format!("builtin_str_join({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_replace" => format!("builtin_str_replace({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "str_replace_all" => format!("builtin_str_replace_all({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "str_contains" => format!("builtin_str_contains({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_find" => format!("builtin_str_find({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_trim" => format!("builtin_str_trim({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "str_slice" => format!("builtin_str_slice({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "str_starts_with" => format!("builtin_str_starts_with({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_ends_with" => format!("builtin_str_ends_with({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_repeat" => format!("builtin_str_repeat({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "str_is_num" => format!("builtin_str_is_num({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                
                // FS
                "fs_read" => format!("builtin_fs_read({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_write" => format!("builtin_fs_write({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "fs_append" => format!("builtin_fs_append({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "fs_list" => format!("builtin_fs_list({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_is_dir" => format!("builtin_fs_is_dir({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_size" => format!("builtin_fs_size({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_exists" => format!("builtin_fs_exists({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_delete" => format!("builtin_fs_delete({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_mkdir" => format!("builtin_fs_mkdir({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "fs_move" => format!("builtin_fs_move({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "fs_copy" => format!("builtin_fs_copy({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "fs_chmod" => format!("builtin_fs_chmod({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                
                // Arrays
                "arr_push" => format!("builtin_arr_push({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "arr_pop" => format!("builtin_arr_pop({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "arr_len" => format!("builtin_arr_len({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "arr_remove" => format!("builtin_arr_remove({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "arr_insert" => format!("builtin_arr_insert({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                
                // Maps
                "map_new" => "builtin_map_new()".to_string(),
                "map_get" => format!("builtin_map_get({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "map_set" => format!("builtin_map_set({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "map_has" => format!("builtin_map_has({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "map_keys" => format!("builtin_map_keys({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "map_remove" => format!("builtin_map_remove({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                
                // Math
                "math_abs" => format!("builtin_math_abs({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_max" => format!("builtin_math_max({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "math_min" => format!("builtin_math_min({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "math_rand" => format!("builtin_math_rand({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "math_pow" => format!("builtin_math_pow({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "math_sqrt" => format!("builtin_math_sqrt({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_log" => format!("builtin_math_log({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_sin" => format!("builtin_math_sin({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_cos" => format!("builtin_math_cos({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_floor" => format!("builtin_math_floor({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_ceil" => format!("builtin_math_ceil({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "math_round" => format!("builtin_math_round({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                
                // System
                "sys_exec" => format!("builtin_sys_exec({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "sys_env" => format!("builtin_sys_env({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "sys_hostname" => "builtin_sys_hostname()".to_string(),
                "sys_username" => "builtin_sys_username()".to_string(),
                "sys_user_home" => "builtin_sys_user_home()".to_string(),
                "sys_os" => "builtin_sys_os()".to_string(),
                "sys_pid" => "builtin_sys_pid()".to_string(),
                "sys_time" => "builtin_sys_time()".to_string(),
                "sys_exit" => format!("builtin_sys_exit({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "sys_args" => "builtin_sys_args()".to_string(),
                "sys_cwd" => "builtin_sys_cwd()".to_string(),
                "sys_cd" => format!("builtin_sys_cd({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "sys_shell" => format!("builtin_sys_shell({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "sys_sleep" => format!("builtin_sys_sleep({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                
                // Network
                "net_get" => format!("builtin_net_get({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "net_post" => format!("builtin_net_post({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "net_ping" => format!("builtin_net_ping({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "net_download" => format!("builtin_net_download({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "net_serve" => format!("builtin_net_serve({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                
                // Data
                "type_of" => format!("builtin_type_of({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "json_stringify" => format!("builtin_json_stringify({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "json_parse" => format!("builtin_json_parse({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                
                // GUI
                "gui_msg" => format!("builtin_gui_msg({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "gui_error" => format!("builtin_gui_error({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "gui_warning" => format!("builtin_gui_warning({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "gui_question" => format!("builtin_gui_question({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "gui_input" => format!("builtin_gui_input({}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string())),
                "gui_password" => format!("builtin_gui_password({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "gui_file" => format!("builtin_gui_file({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "gui_calendar" => format!("builtin_gui_calendar({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "gui_color" => format!("builtin_gui_color({})", arg_strs.get(0).unwrap_or(&"Value::None".to_string())),
                "gui_list" => format!("builtin_gui_list({}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string())),
                "gui_scale" => format!("builtin_gui_scale({}, {}, {}, {}, {}, {})", arg_strs.get(0).unwrap_or(&"Value::None".to_string()), arg_strs.get(1).unwrap_or(&"Value::None".to_string()), arg_strs.get(2).unwrap_or(&"Value::None".to_string()), arg_strs.get(3).unwrap_or(&"Value::None".to_string()), arg_strs.get(4).unwrap_or(&"Value::None".to_string()), arg_strs.get(5).unwrap_or(&"Value::None".to_string())),
                _ => format!("vpl_fn_{}({})", name, arg_strs.join(", "))
            }
        }
    }
}

fn generate_runtime(used: &HashSet<String>) -> (String, Vec<String>) {
    let mut r = String::new();
    let mut mods = Vec::new();
    r.push_str(CORE_RUNTIME);
    mods.push("CORE".to_string());
    
    let mut has_tui = false;
    for f in used { if f.starts_with("tui_") { has_tui = true; break; } }
    
    if has_tui { r.push_str(TUI_RUNTIME); mods.push("TUI".to_string()); }
    if used.contains("input") { r.push_str(INPUT_RUNTIME); mods.push("INPUT".to_string()); }
    if used.iter().any(|x| x.starts_with("fs_")) { r.push_str(FS_RUNTIME); mods.push("FILESYSTEM".to_string()); }
    if used.iter().any(|x| x.starts_with("str_") || x.starts_with("num_to_") || x == "type_of" || x == "json_stringify") { r.push_str(STR_RUNTIME); mods.push("STRING/DATA".to_string()); }
    if used.iter().any(|x| x.starts_with("math_")) { r.push_str(MATH_RUNTIME); mods.push("MATH".to_string()); }
    if used.iter().any(|x| x.starts_with("sys_")) { r.push_str(SYS_RUNTIME); mods.push("SYSTEM".to_string()); }
    if used.contains("net_get") || used.contains("net_serve") || used.contains("net_post") || used.contains("net_ping") || used.contains("net_download") { r.push_str(NET_RUNTIME); mods.push("NETWORK".to_string()); }
    if used.iter().any(|x| x.starts_with("arr_") || x == "arr_literal" || x == "arr_index") { r.push_str(ARR_RUNTIME); mods.push("ARRAY".to_string()); }
    if used.iter().any(|x| x.starts_with("map_")) { r.push_str(MAP_RUNTIME); mods.push("MAP".to_string()); }
    if used.iter().any(|x| x.starts_with("gui_")) { r.push_str(GUI_RUNTIME); mods.push("GUI".to_string()); }
    if used.contains("json_parse") { r.push_str(JSON_RUNTIME); mods.push("JSON".to_string()); }
    
    (r, mods)
}

const CORE_RUNTIME: &str = r#"
use std::fmt;
use std::io::{Read, Write};
use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::collections::HashMap;
use std::process::Command;

#[derive(Clone, Debug, PartialEq)]
pub enum Value { Num(i64), Str(String), Array(Rc<RefCell<Vec<Value>>>), Map(Rc<RefCell<HashMap<String, Value>>>), None }

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Num(a), Value::Num(b)) => a.partial_cmp(b),
            (Value::Str(a), Value::Str(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool { match self { Value::Num(n) => *n != 0, Value::Str(s) => !s.is_empty(), Value::Array(a) => !a.borrow().is_empty(), Value::Map(m) => !m.borrow().is_empty(), Value::None => false } }
    pub fn add(&self, other: &Value) -> Value { match (self, other) { (Value::Num(a), Value::Num(b)) => Value::Num(a + b), (Value::Str(a), Value::Str(b)) => Value::Str(format!("{}{}", a, b)), (Value::Num(a), Value::Str(b)) => Value::Str(format!("{}{}", a, b)), (Value::Str(a), Value::Num(b)) => Value::Str(format!("{}{}", a, b)), _ => Value::None } }
    pub fn sub(&self, other: &Value) -> Value { if let (Value::Num(a), Value::Num(b)) = (self, other) { Value::Num(a - b) } else { Value::None } }
    pub fn mul(&self, other: &Value) -> Value { if let (Value::Num(a), Value::Num(b)) = (self, other) { Value::Num(a * b) } else { Value::None } }
    pub fn div(&self, other: &Value) -> Value { if let (Value::Num(a), Value::Num(b)) = (self, other) { if *b != 0 { Value::Num(a / b) } else { Value::None } } else { Value::None } }
    pub fn as_i64(&self) -> i64 { match self { Value::Num(n) => *n, Value::Str(s) => s.parse().unwrap_or(0), _ => 0 } }
    pub fn get_index(&self, idx: Value) -> Value { 
        if let Value::Array(a) = self { 
            let i = idx.as_i64(); if i >= 0 && (i as usize) < a.borrow().len() { return a.borrow()[i as usize].clone(); } 
        } else if let Value::Map(m) = self {
            if let Value::Str(k) = idx { return m.borrow().get(&k).cloned().unwrap_or(Value::None); }
        }
        Value::None 
    }
    pub fn set_index(&self, idx: Value, val: Value) { 
        if let Value::Array(a) = self { 
            let i = idx.as_i64(); if i >= 0 && (i as usize) < a.borrow().len() { a.borrow_mut()[i as usize] = val; } 
        } else if let Value::Map(m) = self {
            if let Value::Str(k) = idx { m.borrow_mut().insert(k, val); }
        }
    }
}
impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { 
        match self { 
            Value::Num(n) => write!(f, "{}", n), 
            Value::Str(s) => write!(f, "{}", s), 
            Value::Array(a) => { write!(f, "[")?; for (i, it) in a.borrow().iter().enumerate() { if i > 0 { write!(f, ", ")?; } write!(f, "{}", it)?; } write!(f, "]") } 
            Value::Map(m) => { write!(f, "{{")?; let mut first = true; for (k, v) in m.borrow().iter() { if !first { write!(f, ", ")?; } first = false; write!(f, "\"{}\": {}", k, v)?; } write!(f, "}}") }
            Value::None => write!(f, "none") 
        } 
    }
}

pub fn builtin_fs_exists(path: Value) -> Value { if let Value::Str(p) = path { return Value::Num(if fs::metadata(p).is_ok() { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_fs_delete(path: Value) -> Value { if let Value::Str(p) = path { return Value::Num(if fs::remove_file(&p).is_ok() || fs::remove_dir_all(&p).is_ok() { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_fs_mkdir(path: Value) -> Value { if let Value::Str(p) = path { return Value::Num(if fs::create_dir_all(p).is_ok() { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_fs_move(old: Value, new: Value) -> Value { if let (Value::Str(o), Value::Str(n)) = (old, new) { return Value::Num(if fs::rename(o, n).is_ok() { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_fs_copy(src: Value, dst: Value) -> Value { if let (Value::Str(s), Value::Str(d)) = (src, dst) { return Value::Num(if fs::copy(s, d).is_ok() { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_fs_chmod(path: Value, mode: Value) -> Value { #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; if let (Value::Str(p), Value::Num(m)) = (path, mode) { if fs::set_permissions(p, fs::Permissions::from_mode(m as u32)).is_ok() { return Value::Num(1); } } } Value::Num(0) }
"#;

const TUI_RUNTIME: &str = r#"
#[repr(C)] struct termios { c_iflag: u32, c_oflag: u32, c_cflag: u32, c_lflag: u32, c_line: u8, c_cc: [u8; 32], c_ispeed: u32, c_ospeed: u32 }
#[repr(C)] struct winsize { ws_row: u16, ws_col: u16, ws_xpixel: u16, ws_ypixel: u16 }
extern "C" { fn tcgetattr(fd: i32, termios_p: *mut termios) -> i32; fn tcsetattr(fd: i32, optional_actions: i32, termios_p: *const termios) -> i32; fn ioctl(fd: i32, request: u64, ...) -> i32; }
static mut ORIG_TERMIOS: Option<termios> = None;
pub fn builtin_tui_init() -> Value { unsafe { let mut raw: termios = std::mem::zeroed(); if tcgetattr(0, &mut raw) != 0 { return Value::None; } ORIG_TERMIOS = Some(std::ptr::read(&raw)); raw.c_lflag &= !0x0000000A; raw.c_iflag &= !0x00000500; tcsetattr(0, 0, &raw); } Value::None }
pub fn builtin_tui_restore() -> Value { unsafe { if let Some(ref raw) = ORIG_TERMIOS { tcsetattr(0, 0, raw); } } Value::None }
pub fn builtin_tui_width() -> Value { unsafe { let mut w: winsize = std::mem::zeroed(); if ioctl(1, 0x5413, &mut w) == 0 { return Value::Num(w.ws_col as i64); } } Value::Num(80) }
pub fn builtin_tui_height() -> Value { unsafe { let mut w: winsize = std::mem::zeroed(); if ioctl(1, 0x5413, &mut w) == 0 { return Value::Num(w.ws_row as i64); } } Value::Num(24) }
pub fn builtin_tui_clear() -> Value { print!("\x1b[2J\x1b[H"); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_hide_cursor() -> Value { print!("\x1b[?25l"); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_show_cursor() -> Value { print!("\x1b[?25h"); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_set_color(fg: Value, bg: Value) -> Value { let f = fg.as_i64(); let b = bg.as_i64(); if f >= 0 { print!("\x1b[{}m", 30 + (f % 8)); } if b >= 0 { print!("\x1b[{}m", 40 + (b % 8)); } std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_reset_color() -> Value { print!("\x1b[0m"); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_print(x: Value, y: Value, text: Value) -> Value { print!("\x1b[{};{}H{}", y.as_i64(), x.as_i64(), text); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_delay(ms: Value) -> Value { std::thread::sleep(std::time::Duration::from_millis(ms.as_i64() as u64)); Value::None }
pub fn builtin_tui_set_title(t: Value) -> Value { print!("\x1b]2;{}\x07", t); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_bold(e: Value) -> Value { if e.is_truthy() { print!("\x1b[1m"); } else { print!("\x1b[22m"); } std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_underline(e: Value) -> Value { if e.is_truthy() { print!("\x1b[4m"); } else { print!("\x1b[24m"); } std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_beep() -> Value { print!("\x07"); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_fill(x_v: Value, y_v: Value, w_v: Value, h_v: Value, ch_v: Value) -> Value { let x = x_v.as_i64(); let y = y_v.as_i64(); let w = w_v.as_i64(); let h = h_v.as_i64(); let ch = ch_v.to_string().chars().next().unwrap_or(' '); let line = ch.to_string().repeat(w as usize); for i in 0..h { print!("\x1b[{};{}H{}", y + i, x, line); } std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_draw_box(x_v: Value, y_v: Value, w_v: Value, h_v: Value) -> Value { let x = x_v.as_i64(); let y = y_v.as_i64(); let w = w_v.as_i64(); let h = h_v.as_i64(); if w < 2 || h < 2 { return Value::None; } print!("\x1b[{};{}H┌", y, x); for _ in 1..(w-1) { print!("─"); } print!("┐"); print!("\x1b[{};{}H└", y + h - 1, x); for _ in 1..(w-1) { print!("─"); } print!("┘"); for i in 1..(h-1) { print!("\x1b[{};{}H│", y + i, x); print!("\x1b[{};{}H│", y + i, x + w - 1); } std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_draw_window(x_v: Value, y_v: Value, w_v: Value, h_v: Value, title_v: Value) -> Value { let x = x_v.as_i64(); let y = y_v.as_i64(); let w = w_v.as_i64(); let h = h_v.as_i64(); builtin_tui_fill(x_v.clone(), y_v.clone(), w_v.clone(), h_v.clone(), Value::Str(" ".to_string())); builtin_tui_draw_box(x_v.clone(), y_v.clone(), w_v.clone(), h_v.clone()); let title = title_v.to_string(); let title_x = x + (w - title.len() as i64) / 2; print!("\x1b[{};{}H\x1b[1m {} \x1b[0m", y, title_x, title); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_read_key() -> Value { let mut buf = [0; 3]; let mut stdin = std::io::stdin(); let bytes_read = stdin.read(&mut buf[..1]).unwrap_or(0); if bytes_read == 0 { return Value::None; } if buf[0] == 27 { let mut nbuf = [0; 2]; let bytes_next = stdin.read(&mut nbuf).unwrap_or(0); if bytes_next == 2 && nbuf[0] == 91 { match nbuf[1] { 65 => return Value::Str("up".to_string()), 66 => return Value::Str("down".to_string()), 67 => return Value::Str("right".to_string()), 68 => return Value::Str("left".to_string()), _ => {} } } if bytes_next == 2 && nbuf[0] == 79 { match nbuf[1] { 80 => return Value::Str("f1".to_string()), 81 => return Value::Str("f2".to_string()), 82 => return Value::Str("f3".to_string()), 83 => return Value::Str("f4".to_string()), _ => {} } } return Value::Str("esc".to_string()); } if buf[0] == 127 || buf[0] == 8 { return Value::Str("backspace".to_string()); } if buf[0] == 10 || buf[0] == 13 { return Value::Str("enter".to_string()); } if buf[0] == 9 { return Value::Str("tab".to_string()); } Value::Str((buf[0] as char).to_string()) }
pub fn builtin_tui_read_line(x_v: Value, y_v: Value, w_v: Value) -> Value { let mut input = String::new(); let x = x_v.as_i64(); let y = y_v.as_i64(); let w = w_v.as_i64(); loop { print!("\x1b[{};{}H\x1b[K{}", y, x, input); if (input.len() as i64) < w { print!("\x1b[7m \x1b[0m"); } std::io::stdout().flush().ok(); let key = builtin_tui_read_key(); match key.to_string().as_str() { "enter" => break, "backspace" => { input.pop(); } "esc" => { input.clear(); break; } k if k.len() == 1 => { if (input.len() as i64) < w { input.push_str(k); } } _ => {} } } Value::Str(input) }
pub fn builtin_tui_draw_button(x_v: Value, y_v: Value, w_v: Value, h_v: Value, text_v: Value) -> Value { let x = x_v.as_i64(); let y = y_v.as_i64(); let w = w_v.as_i64(); let h = h_v.as_i64(); let text = text_v.to_string(); builtin_tui_fill(x_v.clone(), y_v.clone(), w_v.clone(), h_v.clone(), Value::Str(" ".to_string())); builtin_tui_draw_box(x_v.clone(), y_v.clone(), w_v.clone(), h_v.clone()); let text_x = x + (w - text.len() as i64) / 2; let text_y = y + h / 2; print!("\x1b[{};{}H{}", text_y, text_x, text); std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_menu(x_v: Value, y_v: Value, w_v: Value, items_v: Value, selected_v: Value) -> Value { let x = x_v.as_i64(); let y = y_v.as_i64(); let w = w_v.as_i64(); let selected = selected_v.as_i64(); if let Value::Array(items) = items_v { let b = items.borrow(); for (i, item) in b.iter().enumerate() { if i as i64 == selected { print!("\x1b[{};{}H\x1b[7m {:<width$} \x1b[0m", y + i as i64, x, item.to_string(), width = (w as usize - 2)); } else { print!("\x1b[{};{}H  {:<width$} ", y + i as i64, x, item.to_string(), width = (w as usize - 2)); } } } std::io::stdout().flush().ok(); Value::None }
pub fn builtin_tui_cursor_pos(x: Value, y: Value) -> Value { print!("\x1b[{};{}H", y.as_i64(), x.as_i64()); std::io::stdout().flush().ok(); Value::None }
"#;

const INPUT_RUNTIME: &str = r#"
pub fn builtin_input() -> Value { let mut input = String::new(); std::io::stdin().read_line(&mut input).ok(); Value::Str(input.trim().to_string()) }
"#;

const FS_RUNTIME: &str = r#"
pub fn builtin_fs_read(path: Value) -> Value { if let Value::Str(p) = path { fs::read_to_string(p).map(Value::Str).unwrap_or(Value::None) } else { Value::None } }
pub fn builtin_fs_write(path: Value, content: Value) -> Value { if let (Value::Str(p), Value::Str(c)) = (path, content) { fs::write(p, c).map(|_| Value::Num(1)).unwrap_or(Value::Num(0)) } else { Value::Num(0) } }
pub fn builtin_fs_append(path: Value, content: Value) -> Value { if let (Value::Str(p), Value::Str(c)) = (path, content) { if let Ok(mut f) = fs::OpenOptions::new().append(true).create(true).open(p) { if f.write_all(c.as_bytes()).is_ok() { return Value::Num(1); } } } Value::Num(0) }
pub fn builtin_fs_list(path: Value) -> Value { if let Value::Str(p) = path { let mut items = Vec::new(); if let Ok(entries) = fs::read_dir(p) { for entry in entries.flatten() { if let Ok(name) = entry.file_name().into_string() { items.push(Value::Str(name)); } } } items.sort_by(|a, b| if let (Value::Str(s1), Value::Str(s2)) = (a, b) { s1.cmp(s2) } else { std::cmp::Ordering::Equal }); Value::Array(Rc::new(RefCell::new(items))) } else { Value::Array(Rc::new(RefCell::new(Vec::new()))) } }
pub fn builtin_fs_is_dir(path: Value) -> Value { if let Value::Str(p) = path { if let Ok(m) = fs::metadata(p) { return Value::Num(if m.is_dir() { 1 } else { 0 }); } } Value::Num(0) }
pub fn builtin_fs_size(path: Value) -> Value { if let Value::Str(p) = path { if let Ok(m) = fs::metadata(p) { return Value::Num(m.len() as i64); } } Value::Num(-1) }
"#;

const STR_RUNTIME: &str = r#"
pub fn builtin_str_len(v: Value) -> Value { if let Value::Str(s) = v { Value::Num(s.chars().count() as i64) } else { Value::Num(0) } }
pub fn builtin_str_at(v: Value, idx: Value) -> Value { if let Value::Str(s) = v { let i = idx.as_i64(); if i >= 0 { if let Some(c) = s.chars().nth(i as usize) { return Value::Str(c.to_string()); } } } Value::None }
pub fn builtin_num_to_str(v: Value) -> Value { Value::Str(v.to_string()) }
pub fn builtin_str_to_num(v: Value) -> Value { Value::Num(v.as_i64()) }
pub fn builtin_num_to_hex(v: Value) -> Value { Value::Str(format!("{:x}", v.as_i64())) }
pub fn builtin_str_upper(v: Value) -> Value { if let Value::Str(s) = v { Value::Str(s.to_uppercase()) } else { Value::None } }
pub fn builtin_str_lower(v: Value) -> Value { if let Value::Str(s) = v { Value::Str(s.to_lowercase()) } else { Value::None } }
pub fn builtin_str_split(v: Value, sep: Value) -> Value { if let (Value::Str(s), Value::Str(sep)) = (v, sep) { let parts: Vec<Value> = s.split(&sep).map(|p| Value::Str(p.to_string())).collect(); return Value::Array(Rc::new(RefCell::new(parts))); } Value::Array(Rc::new(RefCell::new(Vec::new()))) }
pub fn builtin_str_join(arr: Value, sep: Value) -> Value { if let (Value::Array(a), Value::Str(sep)) = (arr, sep) { let res: Vec<String> = a.borrow().iter().map(|v| v.to_string()).collect(); return Value::Str(res.join(&sep)); } Value::Str("".to_string()) }
pub fn builtin_str_replace(v: Value, old: Value, new: Value) -> Value { if let (Value::Str(s), Value::Str(o), Value::Str(n)) = (v, old, new) { return Value::Str(s.replace(&o, &n)); } Value::None }
pub fn builtin_str_replace_all(v: Value, old: Value, new: Value) -> Value { if let (Value::Str(s), Value::Str(o), Value::Str(n)) = (v, old, new) { return Value::Str(s.replace(&o, &n)); } Value::None }
pub fn builtin_str_contains(v: Value, sub: Value) -> Value { if let (Value::Str(s), Value::Str(sub)) = (v, sub) { return Value::Num(if s.contains(&sub) { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_str_find(v: Value, sub: Value) -> Value { if let (Value::Str(s), Value::Str(sub)) = (v, sub) { if let Some(pos) = s.find(&sub) { return Value::Num(pos as i64); } } Value::Num(-1) }
pub fn builtin_str_trim(v: Value) -> Value { if let Value::Str(s) = v { Value::Str(s.trim().to_string()) } else { Value::None } }
pub fn builtin_str_slice(v: Value, start: Value, len: Value) -> Value { if let Value::Str(s) = v { let st = start.as_i64() as usize; let l = len.as_i64() as usize; let chars: Vec<char> = s.chars().collect(); if st < chars.len() { let end = std::cmp::min(st + l, chars.len()); return Value::Str(chars[st..end].iter().collect()); } } Value::Str("".to_string()) }
pub fn builtin_str_starts_with(v: Value, p: Value) -> Value { if let (Value::Str(s), Value::Str(prefix)) = (v, p) { Value::Num(if s.starts_with(&prefix) { 1 } else { 0 }) } else { Value::Num(0) } }
pub fn builtin_str_ends_with(v: Value, p: Value) -> Value { if let (Value::Str(s), Value::Str(suffix)) = (v, p) { Value::Num(if s.ends_with(&suffix) { 1 } else { 0 }) } else { Value::Num(0) } }
pub fn builtin_str_repeat(v: Value, n: Value) -> Value { if let Value::Str(s) = v { Value::Str(s.repeat(n.as_i64() as usize)) } else { Value::None } }
pub fn builtin_str_is_num(v: Value) -> Value { if let Value::Str(s) = v { Value::Num(if s.parse::<i64>().is_ok() { 1 } else { 0 }) } else { Value::Num(0) } }
pub fn builtin_type_of(v: Value) -> Value { match v { Value::Num(_) => Value::Str("num".to_string()), Value::Str(_) => Value::Str("str".to_string()), Value::Array(_) => Value::Str("arr".to_string()), Value::Map(_) => Value::Str("map".to_string()), Value::None => Value::Str("none".to_string()) } }
pub fn builtin_json_stringify(v: Value) -> Value { fn stringify(val: &Value) -> String { match val { Value::Num(n) => n.to_string(), Value::Str(s) => format!("\"{}\"", s.replace("\"", "\\\"").replace("\n", "\\n")), Value::Array(a) => { let items: Vec<String> = a.borrow().iter().map(|x| stringify(x)).collect(); format!("[{}]", items.join(",")) } Value::Map(m) => { let items: Vec<String> = m.borrow().iter().map(|(k, v)| format!("\"{}\":{}", k, stringify(v))).collect(); format!("{{{}}}", items.join(",")) } Value::None => "null".to_string(), } } Value::Str(stringify(&v)) }
"#;

const MAP_RUNTIME: &str = r#"
pub fn builtin_map_new() -> Value { Value::Map(Rc::new(RefCell::new(HashMap::new()))) }
pub fn builtin_map_get(map: Value, key: Value) -> Value { if let (Value::Map(m), Value::Str(k)) = (map, key) { m.borrow().get(&k).cloned().unwrap_or(Value::None) } else { Value::None } }
pub fn builtin_map_set(map: Value, key: Value, val: Value) -> Value { if let (Value::Map(m), Value::Str(k)) = (map, key) { m.borrow_mut().insert(k, val); return Value::Num(1); } Value::Num(0) }
pub fn builtin_map_has(map: Value, key: Value) -> Value { if let (Value::Map(m), Value::Str(k)) = (map, key) { return Value::Num(if m.borrow().contains_key(&k) { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_map_keys(map: Value) -> Value { if let Value::Map(m) = map { let keys: Vec<Value> = m.borrow().keys().map(|k| Value::Str(k.clone())).collect(); return Value::Array(Rc::new(RefCell::new(keys))); } Value::Array(Rc::new(RefCell::new(Vec::new()))) }
pub fn builtin_map_remove(map: Value, key: Value) -> Value { if let (Value::Map(m), Value::Str(k)) = (map, key) { m.borrow_mut().remove(&k); return Value::Num(1); } Value::Num(0) }
"#;

const MATH_RUNTIME: &str = r#"
pub fn builtin_math_abs(v: Value) -> Value { Value::Num(v.as_i64().abs()) }
pub fn builtin_math_max(v1: Value, v2: Value) -> Value { Value::Num(v1.as_i64().max(v2.as_i64())) }
pub fn builtin_math_min(v1: Value, v2: Value) -> Value { Value::Num(v1.as_i64().min(v2.as_i64())) }
pub fn builtin_math_rand(min: Value, max: Value) -> Value { let a = min.as_i64(); let b = max.as_i64(); if a >= b { return Value::Num(a); } let mut bytes = [0u8; 8]; if let Ok(mut f) = fs::File::open("/dev/urandom") { f.read_exact(&mut bytes).ok(); } let rand_num = u64::from_le_bytes(bytes); let range = (b - a + 1) as u64; Value::Num(a + (rand_num % range) as i64) }
pub fn builtin_math_pow(base: Value, exp: Value) -> Value { Value::Num(base.as_i64().pow(exp.as_i64() as u32)) }
pub fn builtin_math_sqrt(v: Value) -> Value { Value::Num((v.as_i64() as f64).sqrt() as i64) }
pub fn builtin_math_log(v: Value) -> Value { Value::Num((v.as_i64() as f64).ln() as i64) }
pub fn builtin_math_sin(v: Value) -> Value { Value::Num((v.as_i64() as f64).sin() as i64) }
pub fn builtin_math_cos(v: Value) -> Value { Value::Num((v.as_i64() as f64).cos() as i64) }
pub fn builtin_math_floor(v: Value) -> Value { Value::Num((v.as_i64() as f64).floor() as i64) }
pub fn builtin_math_ceil(v: Value) -> Value { Value::Num((v.as_i64() as f64).ceil() as i64) }
pub fn builtin_math_round(v: Value) -> Value { Value::Num((v.as_i64() as f64).round() as i64) }
"#;

const SYS_RUNTIME: &str = r#"
pub fn builtin_sys_exec(cmd: Value) -> Value { if let Value::Str(c) = cmd { if let Ok(o) = Command::new("sh").arg("-c").arg(c).output() { return Value::Str(String::from_utf8_lossy(&o.stdout).to_string()); } } Value::None }
pub fn builtin_sys_env(n: Value) -> Value { if let Value::Str(name) = n { if let Ok(v) = std::env::var(name) { return Value::Str(v); } } Value::None }
pub fn builtin_sys_time() -> Value { Value::Num(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64) }
pub fn builtin_sys_exit(code: Value) -> Value { std::process::exit(code.as_i64() as i32); }
pub fn builtin_sys_args() -> Value { let args: Vec<Value> = std::env::args().skip(1).map(Value::Str).collect(); Value::Array(Rc::new(RefCell::new(args))) }
pub fn builtin_sys_cwd() -> Value { if let Ok(p) = std::env::current_dir() { Value::Str(p.to_string_lossy().to_string()) } else { Value::None } }
pub fn builtin_sys_cd(path: Value) -> Value { if let Value::Str(p) = path { if std::env::set_current_dir(p).is_ok() { return Value::Num(1); } } Value::Num(0) }
pub fn builtin_sys_shell(cmd: Value) -> Value { if let Value::Str(c) = cmd { if let Ok(mut status) = Command::new("sh").arg("-c").arg(c).spawn() { if let Ok(s) = status.wait() { return Value::Num(s.code().unwrap_or(0) as i64); } } } Value::Num(-1) }
pub fn builtin_sys_sleep(ms: Value) -> Value { std::thread::sleep(std::time::Duration::from_millis(ms.as_i64() as u64)); Value::None }
pub fn builtin_sys_pid() -> Value { Value::Num(std::process::id() as i64) }
pub fn builtin_sys_os() -> Value { Value::Str(std::env::consts::OS.to_string()) }
pub fn builtin_sys_hostname() -> Value { if let Ok(h) = fs::read_to_string("/proc/sys/kernel/hostname") { return Value::Str(h.trim().to_string()); } Value::Str("unknown".to_string()) }
pub fn builtin_sys_username() -> Value { if let Ok(u) = std::env::var("USER") { return Value::Str(u); } Value::Str("unknown".to_string()) }
pub fn builtin_sys_user_home() -> Value { if let Ok(u) = std::env::var("HOME") { return Value::Str(u); } Value::Str("unknown".to_string()) }
"#;

const NET_RUNTIME: &str = r#"
use std::net::{TcpListener, TcpStream};
use std::io::{BufRead, BufReader};
pub fn builtin_net_get(url: Value) -> Value { if let Value::Str(u) = url { if let Ok(o) = Command::new("curl").arg("-s").arg("-L").arg(u).output() { if o.status.success() { return Value::Str(String::from_utf8_lossy(&o.stdout).to_string()); } } } Value::None }
pub fn builtin_net_post(url: Value, body: Value) -> Value { if let (Value::Str(u), Value::Str(b)) = (url, body) { if let Ok(o) = Command::new("curl").arg("-s").arg("-X").arg("POST").arg("-d").arg(b).arg(u).output() { if o.status.success() { return Value::Str(String::from_utf8_lossy(&o.stdout).to_string()); } } } Value::None }
pub fn builtin_net_download(url: Value, path: Value) -> Value { if let (Value::Str(u), Value::Str(p)) = (url, path) { if let Ok(mut s) = Command::new("curl").arg("-s").arg("-L").arg("-o").arg(p).arg(u).spawn() { if let Ok(es) = s.wait() { return Value::Num(if es.success() { 1 } else { 0 }); } } } Value::Num(0) }
pub fn builtin_net_ping(host: Value) -> Value { if let Value::Str(h) = host { let s = Command::new("ping").arg("-c").arg("1").arg("-W").arg("1").arg(h).status(); return Value::Num(if s.map(|x| x.success()).unwrap_or(false) { 1 } else { 0 }); } Value::Num(0) }
pub fn builtin_net_serve(port: Value) -> Value { let p = port.as_i64(); if let Ok(listener) = TcpListener::bind(format!("0.0.0.0:{}", p)) { if let Ok((mut stream, _)) = listener.accept() { let mut reader = BufReader::new(&stream); let mut request_line = String::new(); if reader.read_line(&mut request_line).is_ok() { let parts: Vec<&str> = request_line.split_whitespace().collect(); if parts.len() >= 2 { let map = Rc::new(RefCell::new(HashMap::new())); map.borrow_mut().insert("method".to_string(), Value::Str(parts[0].to_string())); map.borrow_mut().insert("path".to_string(), Value::Str(parts[1].to_string())); let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK"; stream.write_all(response.as_bytes()).ok(); return Value::Map(map); } } } } Value::None }
"#;

const ARR_RUNTIME: &str = r#"
pub fn builtin_arr_push(arr: Value, val: Value) -> Value { if let Value::Array(a) = arr { a.borrow_mut().push(val); return Value::Num(1); } Value::Num(0) }
pub fn builtin_arr_pop(arr: Value) -> Value { if let Value::Array(a) = arr { return a.borrow_mut().pop().unwrap_or(Value::None); } Value::None }
pub fn builtin_arr_insert(arr: Value, idx: Value, val: Value) -> Value { if let Value::Array(a) = arr { let i = idx.as_i64(); if i >= 0 && (i as usize) <= a.borrow().len() { a.borrow_mut().insert(i as usize, val); return Value::Num(1); } } Value::Num(0) }
pub fn builtin_arr_len(arr: Value) -> Value { if let Value::Array(a) = arr { return Value::Num(a.borrow().len() as i64); } Value::Num(0) }
pub fn builtin_arr_remove(arr: Value, idx: Value) -> Value { if let Value::Array(a) = arr { let i = idx.as_i64(); if i >= 0 && (i as usize) < a.borrow().len() { return a.borrow_mut().remove(i as usize); } } Value::None }
"#;

const GUI_RUNTIME: &str = r#"
pub fn builtin_gui_msg(msg: Value, title: Value) -> Value { if let (Value::Str(m), Value::Str(t)) = (msg, title) { Command::new("zenity").arg("--info").arg("--text").arg(m).arg("--title").arg(t).output().ok(); } Value::None }
pub fn builtin_gui_error(msg: Value, title: Value) -> Value { if let (Value::Str(m), Value::Str(t)) = (msg, title) { Command::new("zenity").arg("--error").arg("--text").arg(m).arg("--title").arg(t).output().ok(); } Value::None }
pub fn builtin_gui_warning(msg: Value, title: Value) -> Value { if let (Value::Str(m), Value::Str(t)) = (msg, title) { Command::new("zenity").arg("--warning").arg("--text").arg(m).arg("--title").arg(t).output().ok(); } Value::None }
pub fn builtin_gui_question(msg: Value, title: Value) -> Value { if let (Value::Str(m), Value::Str(t)) = (msg, title) { if let Ok(out) = Command::new("zenity").arg("--question").arg("--text").arg(m).arg("--title").arg(t).status() { if out.success() { return Value::Num(1); } } } Value::Num(0) }
pub fn builtin_gui_input(msg: Value, title: Value) -> Value { if let (Value::Str(m), Value::Str(t)) = (msg, title) { if let Ok(out) = Command::new("zenity").arg("--entry").arg("--text").arg(m).arg("--title").arg(t).output() { if out.status.success() { return Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()); } } } Value::None }
pub fn builtin_gui_password(title: Value) -> Value { if let Value::Str(t) = title { if let Ok(out) = Command::new("zenity").arg("--password").arg("--title").arg(t).output() { if out.status.success() { return Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()); } } } Value::None }
pub fn builtin_gui_file(title: Value) -> Value { if let Value::Str(t) = title { if let Ok(out) = Command::new("zenity").arg("--file-selection").arg("--title").arg(t).output() { if out.status.success() { return Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()); } } } Value::None }
pub fn builtin_gui_calendar(title: Value) -> Value { if let Value::Str(t) = title { if let Ok(out) = Command::new("zenity").arg("--calendar").arg("--title").arg(t).arg("--date-format").arg("%Y-%m-%d").output() { if out.status.success() { return Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()); } } } Value::None }
pub fn builtin_gui_color(title: Value) -> Value { if let Value::Str(t) = title { if let Ok(out) = Command::new("zenity").arg("--color-selection").arg("--title").arg(t).output() { if out.status.success() { return Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()); } } } Value::None }
pub fn builtin_gui_list(title: Value, col: Value, items: Value) -> Value { if let (Value::Str(t), Value::Str(c), Value::Array(a)) = (title, col, items) { let mut cmd = Command::new("zenity"); cmd.arg("--list").arg("--title").arg(&t).arg(format!("--column={}", c)); for item in a.borrow().iter() { cmd.arg(item.to_string()); } if let Ok(out) = cmd.output() { if out.status.success() { return Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()); } } } Value::None }
pub fn builtin_gui_scale(title: Value, text: Value, min: Value, max: Value, val: Value, step: Value) -> Value { if let (Value::Str(t), Value::Str(tx)) = (title, text) { if let Ok(out) = Command::new("zenity").arg("--scale").arg("--title").arg(t).arg("--text").arg(tx).arg(format!("--min-value={}", min.as_i64())).arg(format!("--max-value={}", max.as_i64())).arg(format!("--value={}", val.as_i64())).arg(format!("--step={}", step.as_i64())).output() { if out.status.success() { let res = String::from_utf8_lossy(&out.stdout).trim().to_string(); if let Ok(n) = res.parse::<i64>() { return Value::Num(n); } } } } Value::None }
"#;

const JSON_RUNTIME: &str = r#"
pub fn builtin_json_parse(v: Value) -> Value {
    if let Value::Str(s) = v {
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        fn skip_ws(chars: &[char], i: &mut usize) { while *i < chars.len() && chars[*i].is_whitespace() { *i += 1; } }
        fn parse_val(chars: &[char], i: &mut usize) -> Value {
            skip_ws(chars, i);
            if *i >= chars.len() { return Value::None; }
            match chars[*i] {
                '{' => {
                    *i += 1; let map = Rc::new(RefCell::new(HashMap::new()));
                    loop {
                        skip_ws(chars, i); if *i < chars.len() && chars[*i] == '}' { *i += 1; break; }
                        let key = match parse_val(chars, i) { Value::Str(s) => s, _ => break };
                        skip_ws(chars, i); if *i < chars.len() && chars[*i] == ':' { *i += 1; }
                        let val = parse_val(chars, i); map.borrow_mut().insert(key, val);
                        skip_ws(chars, i); if *i < chars.len() && chars[*i] == ',' { *i += 1; }
                        else if *i < chars.len() && chars[*i] == '}' { *i += 1; break; } else { break; }
                    }
                    Value::Map(map)
                }
                '[' => {
                    *i += 1; let arr = Rc::new(RefCell::new(Vec::new()));
                    loop {
                        skip_ws(chars, i); if *i < chars.len() && chars[*i] == ']' { *i += 1; break; }
                        let val = parse_val(chars, i); arr.borrow_mut().push(val);
                        skip_ws(chars, i); if *i < chars.len() && chars[*i] == ',' { *i += 1; }
                        else if *i < chars.len() && chars[*i] == ']' { *i += 1; break; } else { break; }
                    }
                    Value::Array(arr)
                }
                '"' => {
                    *i += 1; let mut s = String::new();
                    while *i < chars.len() && chars[*i] != '"' { if chars[*i] == '\\' { *i += 1; } s.push(chars[*i]); *i += 1; }
                    if *i < chars.len() { *i += 1; } Value::Str(s)
                }
                '0'..='9' | '-' => {
                    let mut s = String::new();
                    while *i < chars.len() && (chars[*i].is_digit(10) || chars[*i] == '.' || chars[*i] == '-') { s.push(chars[*i]); *i += 1; }
                    if let Ok(n) = s.parse::<i64>() { Value::Num(n) } else if let Ok(f) = s.parse::<f64>() { Value::Num(f as i64) } else { Value::None }
                }
                't' => { if *i + 4 <= chars.len() && chars[*i..*i+4] == ['t','r','u','e'] { *i += 4; Value::Num(1) } else { Value::None } }
                'f' => { if *i + 5 <= chars.len() && chars[*i..*i+5] == ['f','a','l','s','e'] { *i += 5; Value::Num(0) } else { Value::None } }
                'n' => { if *i + 4 <= chars.len() && chars[*i..*i+4] == ['n','u','l','l'] { *i += 4; Value::None } else { Value::None } }
                _ => { *i += 1; Value::None }
            }
        }
        return parse_val(&chars, &mut i);
    }
    Value::None
}
"#;