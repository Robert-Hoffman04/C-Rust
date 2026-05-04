use crate::parser::{AST, ASTNode, CType, LiteralType, StructMember, escape_str};
use crate::tokenizer::TokenType;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind, Result, Write}; //  TODO Make own error
//  help generate placeholder variable names so they dont conflict
use uuid::Uuid;

/*
Build out a map of all the enum names and the specifc enum they corrispond to. This lets us back map variables when writing
    Example:
        //  Enum decleration same in C and Rust
        enum Level {
            LOW,
            MEDIUM,
            HIGH
        };

        //  C usage
        enum Level myVar = MEDIUM;

        //  Rust usage
        let myVar : Level = Level::MEDIUM;
*/
fn build_enum_map(nodes: &Vec<ASTNode>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for ast in nodes {
        //  if the current node is a enum node
        if let ASTNode::Enum {
            name,
            implements,
            options,
        } = &ast
        {
            for option in options {
                map.insert(option.clone(), name.clone());
            }
        }
    }
    map
}

pub fn writer(ast: AST, output: String) {
    //  better error checking
    let mut file = File::create(output).expect("failed to create file");

    let nodes = if let ASTNode::Root(ref nodes) = ast.Node {
        nodes
    } else {
        return;
    };
    let enum_map = build_enum_map(nodes);

    write_node(&ast.Node, &mut file, 0, &enum_map).unwrap();
}

fn write_node(
    node: &ASTNode,
    file: &mut File,
    depth: usize,
    enum_map: &HashMap<String, String>,
) -> Result<()> {
    match node {
        ASTNode::Root(nodes) => {
            for n in nodes {
                write_node(n, file, depth, enum_map)?;
                write!(file, "\n")?;
            }
        }

        ASTNode::Include(path) => {
            let mut p = path.clone().trim().to_string();
            match p.pop() {
                Some(c) => {
                    match c {
                        '>' => {
                            //  Remove starting '<' and ending '>'
                            p.remove(0);
                            writeln!(file, "use {};", p)?;
                        }
                        '"' => {
                            p.remove(0);

                            p = match p.rfind(".") {
                                Some(index) => {
                                    p.split_off(index);
                                    p
                                }
                                None => p,
                            };

                            writeln!(file, "mod {};", p)?;
                            //  Unnecessary in this version but probably needed if functionality was extended further
                            //writeln!(file, "use {};", p)?;
                        }
                        _ => {
                            //println!("TESTING {} {}", c as u8, path);
                            return Err(Error::new(ErrorKind::InvalidData, "unexpected character"));
                        } // TODO Better error backprop
                    }
                }
                None => {
                    //println!("TESTING {} {}", p, path);
                    return Err(Error::new(ErrorKind::InvalidData, "unexpected character"));
                }
            }
        }

        ASTNode::Function {
            return_type,
            name,
            params,
            body,
        } => match body {
            Some(body) => {
                write!(file, "fn {}(", name)?;
                for (i, p) in params.iter().enumerate() {
                    if let ASTNode::Declaration {
                        var_type,
                        name,
                        init,
                    } = &p.Node
                    {
                        let (type_, _) = map_type(var_type);
                        write!(file, "{}: {}", name, type_)?;
                    }
                    if i != params.len() - 1 {
                        write!(file, ", ")?;
                    }
                }
                let (type_, _) = map_type(return_type);
                writeln!(file, ") -> {}", type_)?;
                indent(file, depth)?;
                writeln!(file, "{{")?;

                for statment in body {
                    write_node(&statment.Node, file, depth + 1, enum_map)?;
                }

                indent(file, depth)?;
                writeln!(file, "}}")?;
            }
            None => {}
        },

        ASTNode::Declaration {
            var_type,
            name,
            init,
        } => {
            indent(file, depth)?;

            let setCondition = if depth == 0 { "static" } else { "let" };

            let (type_, isMutable) = map_type(var_type);
            let mutablility = if isMutable && depth > 0 { "mut" } else { "" };

            write!(file, "{} {} {}", setCondition, mutablility, name)?;
            if type_ != "auto" {
                write!(file, ": {}", type_)?;
            }
            if let Some(init) = init {
                write!(file, " = ")?;
                write_expr(&init.Node, file, enum_map)?;
            }
            writeln!(file, ";")?;
        }

        ASTNode::Compound(statements) => {
            indent(file, depth)?;
            writeln!(file, "{{")?;
            for statement in statements {
                write_node(&statement.Node, file, depth + 1, enum_map)?;
            }
            indent(file, depth)?;
            writeln!(file, "}}")?;
        }

        ASTNode::If {
            condition,
            then_branch,
            else_branch,
        } => {
            indent(file, depth)?;
            write!(file, "if ")?;
            write_expr(&condition.Node, file, enum_map)?;
            writeln!(file, "")?;
            indent(file, depth)?;
            writeln!(file, "{{")?;

            write_node(&then_branch.Node, file, depth + 1, enum_map)?;

            indent(file, depth)?;
            writeln!(file, "}}")?;
            indent(file, depth)?;
            writeln!(file, "else")?;
            indent(file, depth)?;
            writeln!(file, "{{")?;

            write_node(&else_branch.Node, file, depth + 1, enum_map)?;

            indent(file, depth)?;
            writeln!(file, "}}")?;
        }

        ASTNode::While { condition, body } => {
            indent(file, depth)?;
            write!(file, "while ")?;
            write_expr(&condition.Node, file, enum_map)?;
            indent(file, depth)?;
            writeln!(file, "")?;
            indent(file, depth)?;
            writeln!(file, "{{")?;

            write_node(&body.Node, file, depth + 1, enum_map)?;

            indent(file, depth)?;
            writeln!(file, "}}")?;
        }

        ASTNode::Return(expr) => {
            indent(file, depth)?;
            write!(file, "return")?;
            if let Some(e) = expr {
                write!(file, " ")?;
                write_expr(&e.Node, file, enum_map)?;
            }
            writeln!(file, ";")?;
        }

        ASTNode::Expression(expr) => {
            indent(file, depth)?;
            write_expr(&expr.Node, file, enum_map)?;
            writeln!(file, ";")?;
        }

        ASTNode::Enum {
            name,
            implements,
            options,
        } => {
            write_derive(file, depth, implements)?;
            indent(file, depth)?;
            writeln!(file, "enum {}", name)?;
            indent(file, depth)?;
            writeln!(file, "{{")?;

            for option in options {
                indent(file, depth + 1)?;
                writeln!(file, "{},", option)?;
            }
            indent(file, depth)?;
            writeln!(file, "}}")?;
        }

        ASTNode::Struct {
            name,
            implements,
            members,
        } => {
            write_struct(file, name, implements, members, depth, enum_map)?;
        }
        _ => {}
    }

    Ok(())
}

fn write_struct(
    file: &mut File,
    name: &str,
    implements: &Vec<String>,
    members: &Vec<StructMember>,
    depth: usize,
    enum_map: &HashMap<String, String>,
) -> Result<()> {
    //  Split all of the variables and functions into seperate lists
    let mut variables: Vec<(bool, &ASTNode)> = vec![];
    let mut functions: Vec<(bool, &ASTNode)> = vec![];
    for member in members {
        match member {
            StructMember::Variable {
                visibility,
                declaration,
            } => {
                variables.push((*visibility, &declaration.Node));
            }
            StructMember::Function { visibility, func } => {
                //  Functions without bodies are skipped, they are defined layer
                if let ASTNode::Function { body: Some(_), .. } = &func.Node {
                    functions.push((*visibility, &func.Node));
                    //println!("{:?}", func);
                }
            }
        }
    }

    indent(file, depth)?;
    write_derive(file, depth, implements)?;

    writeln!(file, "struct {} {{", name)?;

    //  Write out the variable definitions, and keep track of the default values (since you cant set them inline in rust)
    //      Defaults are later used for "new()"

    let mut default_map = HashMap::new();
    for (is_public, field) in &variables {
        if let ASTNode::Declaration {
            var_type,
            name: field_name,
            init,
        } = field
        {
            let (rust_type, _) = map_type(var_type);
            indent(file, depth + 1)?;
            if *is_public {
                write!(file, "pub ")?;
            }
            default_map.insert(field_name, init);
            writeln!(file, "{}: {},", field_name, rust_type)?;
        }
    }

    indent(file, depth)?;
    writeln!(file, "}}")?;
    write!(file, "\n")?;

    //  Write the functions
    if !functions.is_empty() {
        indent(file, depth)?;
        writeln!(file, "impl {} {{", name)?;

        for (is_public, method) in &functions {
            if let ASTNode::Function {
                return_type,
                name: method_name,
                params,
                body: Some(body),
            } = method
            {
                indent(file, depth + 1)?;
                if *is_public {
                    write!(file, "pub ")?;
                }

                write_struct_function(
                    file,
                    depth + 1,
                    name,
                    method_name,
                    params,
                    return_type,
                    body,
                    &default_map,
                    enum_map,
                )?;
            }
        }

        indent(file, depth)?;
        writeln!(file, "}}")?;
        writeln!(file)?;
    }

    //  We only write these for traits that are NOT handled by #[derive],
    //  since their method bodies come from FunctionImplement nodes at the top level.
    //  The FunctionImplement writer handles the actual body — here we just note
    //  which traits need a manual impl.  Display is the main case.
    //
    //  Nothing to emit here: the bodies are written when we hit the
    //  FunctionImplement nodes during the Root traversal.

    Ok(())
}

fn write_derive(file: &mut File, depth: usize, implements: &Vec<String>) -> Result<()> {
    //  Implemetns contains define things as well as traits
    let derivable: Vec<&str> = implements
        .iter()
        .filter_map(|t| match t.as_str() {
            //  In a real application, this would be much more dynamic. Deciding for each individually instead of just mapping
            //  However, to do that here would require knowledge of all the traits or defines that the imported rust files use
            //      Probably pretty easy if this was tied directly into "cargo" instead of just a preprocessing layer
            "Debug" => Some("Debug"),
            _ => None,
        })
        .collect();
    if !derivable.is_empty() {
        writeln!(file, "#[derive({})]", derivable.join(", "))?;
        indent(file, depth)?;
    }

    Ok(())
}

fn write_struct_function(
    file: &mut File,
    depth: usize,
    struct_name: &str,
    name: &str,
    params: &Vec<AST>,
    return_type: &CType,
    body: &Vec<AST>,
    default_map: &HashMap<&String, &Option<Box<AST>>>,
    enum_map: &HashMap<String, String>,
) -> Result<()> {
    write!(file, "fn {}(", name)?;

    //  no static methods because honestly I'm out of time for this project
    //  Also wanted to add defulat values to function, but skipping for same reason

    if name != "new" {
        write!(file, "&mut self")?;
        if params.len() > 0
        {
            write!(file, ", ")?;
        }
    }

    if params.len() != 0 {
        for (i, param) in params.iter().enumerate() {

            if let ASTNode::Declaration {
                var_type,
                name: param_name,
                ..
            } = &param.Node
            {
                let (rust_type, _) = map_type(var_type);
                write!(file, "{}: {}", param_name, rust_type)?;
            }

            if i < params.len()-1
            {
                write!(file, ", ")?;
            }
        }
    }

    write!(file, ")")?;

    let (rust_return, _) = map_type(return_type);
    writeln!(file, " -> {}", rust_return)?;
    indent(file, depth)?;
    writeln!(file, "{{")?;

    if name == "new" {
        let mut init_map = HashMap::new();
        for node in body {
            match &node.Node {
                ASTNode::Expression(ast) => {
                    //  Look specifically for any statements that are of the following form
                    //      this.val = ...
                    //  They need to be mapped to a placeholder variable incase the order of function calls matters
                    //      then those placeholders are used to initilize the struct variables

                    if let ASTNode::Binary { left, right, .. } = &ast.Node
                        && let ASTNode::Binary {
                            left: l, right: r, ..
                        } = &left.Node
                        && let ASTNode::Identifier(obj) = &l.Node
                        && obj == "this"
                        && let ASTNode::Identifier(field) = &r.Node
                    {
                        let placeholder =
                            format!("{}_{}", field, Uuid::new_v4().to_string().replace("-", "_"));

                        init_map.insert(field.clone(), placeholder.clone());

                        //  Generate a new subtree that assigns the proper value to the placeholder, based on the exisitng data
                        let new_decleration = ASTNode::Declaration {
                            var_type: CType::Named(String::from("auto")),
                            name: placeholder,
                            init: Some(right.clone()),
                        };
                        write_node(&new_decleration, file, depth + 1, enum_map)?;
                    } else {
                        write_node(&node.Node, file, depth + 1, enum_map)?;
                    }
                }
                _ => write_node(&node.Node, file, depth + 1, enum_map)?,
            }
        }

        indent(file, depth + 1)?;
        writeln!(file, "{}", struct_name)?;
        indent(file, depth + 1)?;
        writeln!(file, "{{")?;

        //  Write the actual rust constructor at the bottom of the file
        for (field, default) in default_map {
            indent(file, depth + 2)?;

            if let Some(placeholder) = init_map.get(*field) {
                //  Constructor assigned this field, use the placeholder
                writeln!(file, "{}: {},", field, placeholder)?;
            } else if let Some(default_ast) = default {
                //  No constructor assignment but has a default value, write it inline
                write!(file, "{}: ", field)?;
                write_node(&default_ast.Node, file, depth + 2, enum_map)?;
                writeln!(file, ",")?;
            } else {
                //  No assignment and no default, defer to Default::default()
                writeln!(file, "{}: Default::default(),", field)?;
            }
        }

        indent(file, depth + 1)?;
        writeln!(file, "}}")?;
    } else {
        for node in body {
            write_node(&node.Node, file, depth + 1, enum_map)?;
        }
    }

    indent(file, depth)?;
    writeln!(file, "}}")?;
    Ok(())
}

fn write_expr(node: &ASTNode, file: &mut File, enum_map: &HashMap<String, String>) -> Result<()> {
    match node {
        ASTNode::Binary { op, left, right } => {
            write_expr(&left.Node, file, enum_map)?;
            write!(file, " {} ", map_operation(op))?;
            write_expr(&right.Node, file, enum_map)?;
        }
        ASTNode::Unary { op, expr } => {
            write!(file, "{}", map_operation(op))?;
            write_expr(&expr.Node, file, enum_map)?;
        }
        ASTNode::Literal(val) => {
            write!(file, "{}", escape_literal(val))?;
        }
        ASTNode::Identifier(val) => {
            //  Remap the C 'this' for structs to 'self'
            if val == "this" {
                write!(file, "self")?;
            }
            //  if the identifier name is in one of the enums
            else if let Some(enum_name) = enum_map.get(val) {
                write!(file, "{}::{}", enum_name, val)?;
            } else {
                write!(file, "{}", val)?;
            }
        }
        ASTNode::Call {
            function_name,
            arguments,
        } => {
            write!(file, "{}(", function_name)?;
            for (i, arg) in arguments.iter().enumerate() {
                write_expr(&arg.Node, file, enum_map)?;
                if i != arguments.len() - 1 {
                    write!(file, ", ")?;
                }
            }
            write!(file, ")")?;
        }
        //  Realistically should never happen
        _ => {}
    }

    Ok(())
}

//  used to make sure it can properly write things like printf("A line\n");
fn escape_literal(lit: &LiteralType) -> String {
    match lit {
        LiteralType::String(s) => format!("\"{}\"", escape_str(s.as_str())),
        LiteralType::Char(c) => format!("'{}'", escape_str(c.as_str())),
        LiteralType::Number(n) => n.clone(),
    }
}

fn map_type(c_type: &CType) -> (String, bool) {
    match c_type {
        CType::Const(inner) => {
            let (s, _) = map_type(inner);
            (s, false) // const = not mutable
        }
        CType::Pointer(inner) => match inner.as_ref() {
            CType::Named(n) if n == "char" => ("&str".to_string(), false),
            _ => {
                let (s, _) = map_type(inner);
                (format!("*mut {}", s), true)
            }
        },
        CType::Reference(inner) => {
            let (s, _) = map_type(inner);
            (format!("&mut {}", s), true)
        },
        CType::Template(name, args) => {
            let mapped_args: Vec<String> = args.iter().map(|a| map_type(a).0).collect();
            let rust_name = match name.as_str() {
                "vector" | "Vec" => "Vec",
                "map" => "HashMap",
                "unordered_map" => "HashMap",
                n => n,
            };
            (format!("{}<{}>", rust_name, mapped_args.join(", ")), true)
        }
        CType::Named(name) => match name.as_str() {
            "short" => ("i16".to_string(), true),
            "int" => ("i32".to_string(), true),
            "long" => ("i64".to_string(), true),
            "float" => ("f32".to_string(), true),
            "double" => ("f64".to_string(), true),
            "char" => ("char".to_string(), true),
            "void" => ("()".to_string(), true),
            _ => (name.clone(), true),
        },
    }
}

fn map_operation(op: &TokenType) -> &str {
    match op {
        TokenType::Plus => "+",
        TokenType::Minus => "-",
        TokenType::Asterisk => "*",
        TokenType::Slash => "/",
        TokenType::Percent => "%",

        TokenType::Equals => "=",
        TokenType::PlusEquals => "+=",
        TokenType::MinusEquals => "-=",
        TokenType::AsteriskEquals => "*=",
        TokenType::SlashEquals => "/=",
        TokenType::PercentEquals => "%=",

        TokenType::EqualsEquals => "==",
        TokenType::NotEquals => "!=",
        TokenType::LessEquals => "<=",
        TokenType::GreaterEquals => ">=",
        TokenType::OpenAngle => "<",
        TokenType::CloseAngle => ">",

        TokenType::Ampersand => "&",
        TokenType::AmpersandAmpersand => "&&",
        TokenType::AmpersandEquals => "&=",

        TokenType::Pipe => "|",
        TokenType::PipePipe => "||",
        TokenType::PipeEquals => "|=",

        TokenType::Caret => "^",
        TokenType::CaretEquals => "^=",

        TokenType::Tilde => "!",
        TokenType::Exclamation => "!",

        TokenType::ShiftLeft => "<<",
        TokenType::ShiftRight => ">>",

        TokenType::Period => ".",

        TokenType::Arrow => ".",

        _ => "/* unknown_op */",
    }
}

fn indent(file: &mut File, depth: usize) -> Result<()> {
    for _ in 0..depth {
        write!(file, "    ")?;
    }
    Ok(())
}
