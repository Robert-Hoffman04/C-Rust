use super::tokenizer::{Token, TokenType};
use std::fmt;
use std::io::{self, Write};
//  Anonymous enums need an actual name for rust to function properly
use uuid::Uuid;

//  Simplify keeping track of token locations from original file,
#[derive(Debug, Clone)]
pub struct Span {
    start_token: usize,
    end_token: usize,
}

#[derive(Debug, Clone)]
pub struct AST {
    pub Node: ASTNode,
    pub Span: Span,
}

#[derive(Debug, Clone)]
//  Tokens were a stuct because they largly had the same structure,
//  This is an enum because each individual type has very specific requirments
pub enum ASTNode {
    Root(Vec<ASTNode>), //    The Head of all AST's

    Include(String), //    Include propigation for local and external rust files

    Function {
        return_type: CType,
        name: String,
        params: Vec<AST>,
        body: Option<Vec<AST>>,
    },

    Declaration {
        var_type: CType,
        name: String,
        init: Option<Box<AST>>, //  Has to be boxed to prevent recursive struct with infinate size. Similar uses going forward
    },

    Compound(Vec<AST>), //  List of basic statments

    If {
        condition: Box<AST>,
        then_branch: Box<AST>,
        else_branch: Box<AST>, //  If else also contained here, just nested statments all the way down
    },

    While {
        condition: Box<AST>,
        body: Box<AST>,
    },

    Return(Option<Box<AST>>),

    Expression(Box<AST>), //  Wrapper for anything that needs to be evaluated

    Binary {
        //  Its best to think of it as sets of parenthesis being added to an expression
        op: TokenType,  //  1 + 2 * 3 --> (1 + (2 * 3))
        left: Box<AST>, //  This preserves the proper order of operations
        right: Box<AST>,
    },

    Unary {
        op: TokenType,
        expr: Box<AST>,
    },

    Literal(LiteralType), //  Any raw value. "A String", 'c' (char), 123 (Int), 3.14 (float)

    Identifier(String), //  variable names

    Call {
        function_name: String,
        arguments: Vec<AST>,
    },

    Enum {
        name: String,
        implements: Vec<String>,
        options: Vec<String>,
    },

    Struct {
        name: String,
        implements: Vec<String>,
        members: Vec<StructMember>,
    },

    //  For when a function is part of a struct, but defined outside it
    FunctionImplement {
        struct_name: String,
        return_type: CType,
        method_name: String,
        params: Vec<AST>,
        body: Vec<AST>,
    },
}

#[derive(Debug, Clone)]
pub enum StructMember {
    Variable {
        visibility: bool, //  true = public
        declaration: AST,
    },
    Function {
        visibility: bool,
        func: AST,
    },
}

#[derive(Debug, Clone)]
pub enum LiteralType {
    String(String),
    Char(String),
    Number(String),
}

#[derive(Debug, Clone)]
pub enum CType {
    Named(String),
    Pointer(Box<CType>),
    Reference(Box<CType>),
    Const(Box<CType>),
    Template(String, Vec<CType>),
}
impl fmt::Display for CType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CType::Named(name) => write!(f, "{}", name),
            CType::Const(inner) => write!(f, "const {}", inner),
            CType::Pointer(inner) => write!(f, "{}*", inner),
            CType::Reference(inner) => write!(f, "{}&", inner),
            CType::Template(name, args) => {
                write!(f, "{}<", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
        }
    }
}

//  helper function becasue printing string literals wasnt actually escaped
pub fn escape_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

impl ASTNode {
    //  The following code is actually just converted python from a similar project I did ~3 years ago.
    //      Mostly just to help debug the output for the parser
    pub fn show<W: Write>(
        &self,
        buf: &mut W,
        attrnames: bool,
        nodenames: bool,
        showspan: bool,
        node_name: Option<&str>,
        indent: &str,
        islast: bool,
    ) -> io::Result<()> {
        let marker = if islast { "└─" } else { "├─" };
        let lead = format!("{}{}", indent, marker);
        let indent_next = format!("{}{}", indent, if islast { "  " } else { "│ " });

        //  write this node's header
        if nodenames {
            if let Some(name) = node_name {
                write!(buf, "{}{} <{}>: ", lead, self.node_name(), name)?;
            } else {
                write!(buf, "{}{}: ", lead, self.node_name())?;
            }
        } else {
            write!(buf, "{}{}: ", lead, self.node_name())?;
        }

        //  write attributes (non-child data on this node)
        //      For example, the conditional block of an if
        let attrs = self.attrs(attrnames);
        if !attrs.is_empty() {
            write!(buf, "{}", attrs.join(", "))?;
        }

        writeln!(buf)?;

        //  recurse into children
        let children = self.children();
        let count = children.len();
        for (idx, (child_name, child)) in children.into_iter().enumerate() {
            child.show(
                buf,
                attrnames,
                nodenames,
                showspan,
                Some(child_name),
                &indent_next,
                idx == count - 1,
            )?;
        }

        Ok(())
    }

    //  helper to show the display name of this node variant
    fn node_name(&self) -> &'static str {
        match self {
            ASTNode::Root(_) => "Root",
            ASTNode::Include(_) => "Include",
            ASTNode::Function { .. } => "Function",
            ASTNode::Declaration { .. } => "Declaration",
            ASTNode::Compound(_) => "Compound",
            ASTNode::If { .. } => "If",
            ASTNode::While { .. } => "While",
            ASTNode::Return(_) => "Return",
            ASTNode::Expression(_) => "Expression",
            ASTNode::Binary { .. } => "Binary",
            ASTNode::Unary { .. } => "Unary",
            ASTNode::Literal(..) => "Literal",
            ASTNode::Identifier(_) => "Identifier",
            ASTNode::Call { .. } => "Call",
            ASTNode::Enum { .. } => "Enum",
            ASTNode::Struct { .. } => "Struct",
            ASTNode::FunctionImplement { .. } => "FunctionImplement",
        }
    }

    //  helper to extract all the relevant data from a node
    //      for example the name of a function being called
    fn attrs(&self, attrnames: bool) -> Vec<String> {
        let fmt = |name: &str, val: &str| -> String {
            if attrnames {
                format!("{}={}", name, val)
            } else {
                val.to_string()
            }
        };

        match self {
            ASTNode::Include(path) => vec![fmt("path", path)],
            ASTNode::Literal(lit) => match lit {
                LiteralType::String(s) => vec![fmt("value", &escape_str(s))],
                LiteralType::Char(c) => vec![fmt("value", &escape_str(c))],
                LiteralType::Number(n) => vec![fmt("value", n)],
            },
            ASTNode::Identifier(name) => vec![fmt("name", &escape_str(name))],

            ASTNode::Binary { op, .. } => vec![fmt("op", (op.to_string()).as_str())],
            ASTNode::Unary { op, .. } => vec![fmt("op", (op.to_string()).as_str())],

            ASTNode::Function {
                return_type, name, ..
            } => vec![
                fmt("return_type", return_type.to_string().as_str()),
                fmt("name", name),
            ],
            ASTNode::FunctionImplement {
                return_type,
                struct_name,
                method_name,
                ..
            } => vec![
                fmt("return_type", return_type.to_string().as_str()),
                fmt("struct_name", struct_name),
                fmt("name", method_name),
            ],
            ASTNode::Declaration { var_type, name, .. } => {
                vec![
                    fmt("var_type", var_type.to_string().as_str()),
                    fmt("name", name),
                ]
            }
            ASTNode::Call { function_name, .. } => vec![fmt("function_name", function_name)],
            ASTNode::Enum {
                name,
                implements,
                options,
            } => {
                vec![
                    fmt("name", name),
                    format!("impliments={:?}", implements),
                    format!("options={:?}", options),
                ]
            }
            ASTNode::Struct {
                name, implements, ..
            } => vec![fmt("name", name), format!("impliments={:?}", implements)],

            //  these dont have any data only children
            ASTNode::Root(_)
            | ASTNode::Compound(_)
            | ASTNode::If { .. }
            | ASTNode::While { .. }
            | ASTNode::Return(_)
            | ASTNode::Expression(_) => vec![],
        }
    }

    //  Returns all child ASTNodes with their relationship label
    //      for example the seperate then and else blocks of a if statment
    fn children(&self) -> Vec<(&'static str, &ASTNode)> {
        match self {
            ASTNode::Root(nodes) => nodes.iter().map(|n| ("item", n)).collect(),

            ASTNode::Function { params, body, .. } => {
                let mut c: Vec<(&'static str, &ASTNode)> = Vec::new();
                c.extend(params.iter().map(|p| ("param", &p.Node)));
                c.extend(
                    body.iter()
                        .flat_map(|v| v.iter())
                        .map(|s| ("stmt", &s.Node)),
                );
                c
            }
            ASTNode::FunctionImplement { params, body, .. } => {
                let mut c: Vec<(&'static str, &ASTNode)> = Vec::new();
                c.extend(params.iter().map(|p| ("param", &p.Node)));
                c.extend(body.iter().map(|s| ("stmt", &s.Node)));
                c
            }

            ASTNode::Declaration { init, .. } => init.iter().map(|i| ("init", &i.Node)).collect(),

            ASTNode::Compound(stmts) => stmts.iter().map(|s| ("stmt", &s.Node)).collect(),

            ASTNode::If {
                condition,
                then_branch,
                else_branch,
            } => vec![
                ("condition", &condition.Node),
                ("then_branch", &then_branch.Node),
                ("else_branch", &else_branch.Node),
            ],

            ASTNode::While { condition, body } => {
                vec![("condition", &condition.Node), ("body", &body.Node)]
            }

            ASTNode::Return(expr) => expr.iter().map(|e| ("value", &e.Node)).collect(),

            ASTNode::Expression(inner) => vec![("expr", &inner.Node)],

            ASTNode::Binary { left, right, .. } => {
                vec![("left", &left.Node), ("right", &right.Node)]
            }

            ASTNode::Unary { expr, .. } => vec![("expr", &expr.Node)],

            ASTNode::Call { arguments, .. } => arguments.iter().map(|a| ("arg", &a.Node)).collect(),

            //  Leaves no children
            ASTNode::Include(_) | ASTNode::Literal(..) | ASTNode::Identifier(_) => vec![],
            ASTNode::Enum { .. } => vec![],
            ASTNode::Struct { members, .. } => {
                let mut c = vec![];
                for member in members {
                    match member {
                        StructMember::Variable { declaration, .. } => {
                            c.push(("declaration", &declaration.Node))
                        }
                        StructMember::Function { func, .. } => c.push(("Function", &func.Node)),
                    }
                }
                c
            }
        }
    }
}

impl fmt::Display for ASTNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = Vec::new();
        self.show(&mut buf, true, true, false, None, "", true)
            .map_err(|_| fmt::Error)?;
        write!(f, "{}", String::from_utf8_lossy(&buf))
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,

    includes: Vec<String>,
    external: Vec<String>,
    struct_names: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            includes: vec![],
            external: vec![],
            struct_names: vec![]
        }
    }

    pub fn parse(&mut self) -> AST {

        let mut willBeStruct = false;
        for token in &self.tokens
        {
            if token.value == "struct"
            {
                willBeStruct = true;
                continue;
            }

            if willBeStruct
            {
                self.struct_names.push(token.value.clone());
            }
            willBeStruct = false;
        }

        let mut nodes = Vec::new();

        while self.pos < self.tokens.len() {
            match self.peekType() {
                TokenType::Include => nodes.push(self.parseInclude()),
                TokenType::Keyword => match self.peek().value.as_str() {
                    "typedef" => {
                        let def = self.parseTypedef();
                        match def {
                            Some(val) => nodes.push(val),
                            None => {}
                        }
                    }

                    "enum" => {
                        if let Some(node) = self.parseEnum() {
                            nodes.push(node);
                        }
                    }
                    "struct" => {
                        if let Some(node) = self.parseStruct() {
                            nodes.push(node);
                        }
                    }

                    _ => nodes.push(self.parseDeclarationOrFunction(true)),
                },
                TokenType::Identifier => nodes.push(self.parseDeclarationOrFunction(true)),
                _ => self.advance(),
            }
        }

        AST {
            Node: ASTNode::Root(nodes.into_iter().map(|n| n.Node).collect()),
            Span: Span {
                start_token: 0,
                end_token: self.tokens.len(),
            },
        }
    }

    fn parseInclude(&mut self) -> AST {
        let inc = AST {
            Node: ASTNode::Include(self.peek().value.clone()),
            Span: Span {
                start_token: self.pos,
                end_token: self.pos,
            },
        };
        self.advance();
        inc
    }

    fn parseTypedef(&mut self) -> Option<AST> {
        self.advance();

        match self.peek().value.as_str() {
            "enum" => {
                let mut node = self.parseEnum()?;

                if *self.peekType() != TokenType::Semicolon {
                    let alias = self.consumeValue();
                    //  Replace node name in place to use new name
                    if let ASTNode::Enum { ref mut name, .. } = node.Node {
                        *name = alias;
                    }
                }
                self.expect(TokenType::Semicolon);
                Some(node)
            }
            "struct" => {
                let mut node = self.parseStruct()?;

                if *self.peekType() != TokenType::Semicolon {
                    let alias = self.consumeValue();
                    //  Replace node name in place to use new name
                    if let ASTNode::Enum { ref mut name, .. } = node.Node {
                        *name = alias;
                    }
                }
                self.expect(TokenType::Semicolon);
                Some(node)
            }
            _ => Some(self.parseDeclarationOrFunction(true)),
        }
    }

    fn parseEnum(&mut self) -> Option<AST> {
        let start = self.pos;
        self.advance(); //  alwasy just "enum"

        let name = if matches!(self.peekType(), TokenType::Identifier) {
            self.consumeValue()
        } else {
            format!("enum_{}", Uuid::new_v4().to_string().replace("-", "_"))
        };

        let mut implements = vec![];
        if matches!(self.peekType(), TokenType::Keyword) && self.peek().value == "impliments" {
            self.advance();

            loop {
                if !matches!(self.peekType(), TokenType::Identifier) {
                    self.expect(TokenType::Identifier);
                }

                implements.push(self.consumeValue());

                if matches!(self.peekType(), TokenType::Comma) {
                    self.advance();
                    continue;
                }
                if matches!(self.peekType(), TokenType::OpenCurly) {
                    break;
                }

                self.expect(TokenType::OpenCurly);
            }
        }

        //  C techincally supports declaration of enums without actually defining them
        //      The later definition is the exact same format as a proper initilization
        //      so we can just through this kind of def away
        if matches!(self.peekType(), TokenType::Semicolon) {
            return None;
        }

        self.expect(TokenType::OpenCurly);

        let mut options = vec![];

        loop {
            if !matches!(self.peekType(), TokenType::Identifier) {
                self.expect(TokenType::Identifier);
            }

            options.push(self.consumeValue());

            if matches!(self.peekType(), TokenType::Comma) {
                self.advance();
                continue;
            }

            if matches!(self.peekType(), TokenType::CloseCurly) {
                self.advance();
                break;
            }

            self.expect(TokenType::CloseCurly);
        }

        Some(AST {
            Node: ASTNode::Enum {
                name,
                implements,
                options,
            },
            Span: Span {
                start_token: start,
                end_token: self.pos,
            },
        })
    }

    fn parseStruct(&mut self) -> Option<AST> {
        let start = self.pos;
        self.advance();

        let name = if matches!(self.peekType(), TokenType::Identifier) {
            self.consumeValue()
        } else {
            self.expect(TokenType::Identifier);
            String::from("") //  Never actually returns this since this will force an expect panic
        };

        let mut implements = vec![];
        if matches!(self.peekType(), TokenType::Keyword) && self.peek().value == "impliments" {
            self.advance();

            loop {
                if !matches!(self.peekType(), TokenType::Identifier) {
                    self.expect(TokenType::Identifier);
                }

                implements.push(self.consumeValue());

                if matches!(self.peekType(), TokenType::Comma) {
                    self.advance();
                    continue;
                }
                if matches!(self.peekType(), TokenType::OpenCurly) {
                    break;
                }

                self.expect(TokenType::OpenCurly);
            }
        }

        self.expect(TokenType::OpenCurly);

        let mut members = vec![];
        let mut visibility = true;

        while !matches!(self.peekType(), TokenType::CloseCurly) {
            //  Read off the public / private settings
            if matches!(self.peekType(), TokenType::Identifier)
                && ["public", "private"].contains(&self.peek().value.as_str())
            {
                visibility = matches!(self.consumeValue().as_str(), "public");
                self.expect(TokenType::Colon);
            }

            let member_start = self.pos;
            let var_type = self.consumeTypeName();

            let is_func_implement = matches!(self.peekType(), TokenType::Identifier)
                && self.pos + 1 < self.tokens.len()
                && matches!(self.tokens[self.pos + 1].token_type, TokenType::ColonColon);

            let is_function = matches!(self.peekType(), TokenType::OpenRound)
                || (matches!(self.peekType(), TokenType::Identifier)
                    && self.pos + 1 < self.tokens.len()
                    && matches!(self.tokens[self.pos + 1].token_type, TokenType::OpenRound));

            if is_func_implement || is_function {
                self.pos = member_start;
                //  notConstructor=false only when the type name matches the struct name
                let not_constructor = var_type.to_string() != name || is_func_implement;
                let func = self.parseDeclarationOrFunction(not_constructor);
                members.push(StructMember::Function { visibility, func });
            }
            //  just a declaration
            else {
                self.pos = member_start;
                let declaration = self.parseDeclarationOrFunction(true);
                members.push(StructMember::Variable {
                    visibility,
                    declaration,
                })
            }
        }

        self.expect(TokenType::CloseCurly);

        println!("{}", self.pos);
        Some(AST {
            Node: ASTNode::Struct {
                name,
                implements,
                members,
            },
            Span: Span {
                start_token: start,
                end_token: self.pos,
            },
        })
    }

    fn parseDeclarationOrFunction(&mut self, notConstructor: bool) -> AST {
        let start_index = self.pos;
        let type_ = self.consumeTypeName();

        let mut name = String::from("new");
        if notConstructor {
            name = self.peek().value.clone();
            self.advance();
        }

        if name == "Display"
        {
            let halt = 1 + 1;
        }

        match self.peekType() {
            //  This means its a struct function defined outside of a struct
            //  OR its a trait implementation in a struct
            //  Either way its handled the same and the writer figures it out
            TokenType::ColonColon => {
                self.advance();
                let method_name = self.consumeValue();
                self.expect(TokenType::OpenRound);
                let params = self.parseParams();
                self.expect(TokenType::CloseRound);
                let body = self.parseBlock();

                AST {
                    Node: ASTNode::FunctionImplement {
                        struct_name: name,
                        return_type: type_,
                        method_name,
                        params,
                        body,
                    },
                    Span: Span {
                        start_token: start_index,
                        end_token: self.pos,
                    },
                }
            }
            //  Its a function
            TokenType::OpenRound => {
                self.advance();

                let params = self.parseParams();

                self.expect(TokenType::CloseRound);

                let body = match self.peekType() {
                    TokenType::OpenCurly => Some(self.parseBlock()),
                    _ => {
                        self.expect(TokenType::Semicolon);
                        None
                    }
                };

                AST {
                    Node: ASTNode::Function {
                        return_type: type_,
                        name,
                        params,
                        body,
                    },
                    Span: Span {
                        start_token: start_index,
                        end_token: self.pos,
                    },
                }
            }
            //  Not a function
            _ => {
                let mut value = None;
                if *self.peekType() == TokenType::Equals {
                    self.advance();
                    value = Some(Box::new(self.parseExpression()));
                }

                self.expect(TokenType::Semicolon);

                AST {
                    Node: ASTNode::Declaration {
                        var_type: type_,
                        name,
                        init: value,
                    },
                    Span: Span {
                        start_token: start_index,
                        end_token: self.pos,
                    },
                }
            }
        }
    }

    fn parseDeclaration(&mut self) -> AST {
        let start_index = self.pos;
        let var_type = self.consumeTypeName();
        let name = self.consumeValue();

        let mut init = None;
        if self.has_tokens() && *self.peekType() == TokenType::Equals {
            self.advance();
            init = Some(Box::new(self.parseExpression()));
        }

        self.expect(TokenType::Semicolon);

        AST {
            Node: ASTNode::Declaration {
                var_type,
                name,
                init,
            },
            Span: Span {
                start_token: start_index,
                end_token: self.pos,
            },
        }
    }

    fn isDeclaration(&self) -> bool {
        let current_is_type = matches!(self.peekType(), TokenType::Keyword | TokenType::Identifier);

        let mut lookahead = self.pos + 1;

        //  Skip past any pointer stars to find the name token
        if lookahead < self.tokens.len()
            && self.tokens[lookahead].token_type == TokenType::OpenAngle
        {
            let mut depth = 1;
            lookahead += 1;
            while lookahead < self.tokens.len() && depth > 0 {
                match self.tokens[lookahead].token_type {
                    TokenType::OpenAngle => depth += 1,
                    TokenType::CloseAngle => depth -= 1,
                    _ => {}
                }
                lookahead += 1;
            }
        }
 
        //  Skip past any pointer stars after the type or after the template close
        while lookahead < self.tokens.len()
            && self.tokens[lookahead].token_type == TokenType::Asterisk
        {
            lookahead += 1;
        }
 
        let next_is_name = lookahead < self.tokens.len()
            && matches!(self.tokens[lookahead].token_type, TokenType::Identifier);
 
        current_is_type && next_is_name

    }

    //  Helper to make sure type names include their pointer portions
    fn consumeTypeName(&mut self) -> CType {

        

        let base = self.consumeValue();

        let mut type_ = match base.as_str() {
            "const" => {
                let inner = self.consumeTypeName();
                CType::Const(Box::new(inner))
            }
            "enum" | "struct" => {
                //  consume the actual name after the keyword
                if matches!(self.peekType(), TokenType::Identifier) {
                    CType::Named(self.consumeValue())
                } else {
                    CType::Named(base)
                }
            }
            _ => CType::Named(base),
        };

        //  Handle template args, like Vec<int, char>
        if matches!(self.peekType(), TokenType::OpenAngle) {
            self.advance();
            let mut args = vec![];
            loop {
                args.push(self.consumeTypeName());
                match self.peekType() {
                    TokenType::Comma => {
                        self.advance();
                    }
                    TokenType::CloseAngle => {
                        self.advance();
                        break;
                    }
                    _ => break,
                }
            }
            let name = match type_ {
                CType::Named(n) => n,
                _ => panic!("Template on non-named type"),
            };
            type_ = CType::Template(name, args);
        }

        //  Handle pointer '*' and reference '&'
        loop {
            match self.peekType() {
                TokenType::Asterisk => {
                    self.advance();
                    type_ = CType::Pointer(Box::new(type_));
                }
                TokenType::Ampersand => {
                    self.advance();
                    type_ = CType::Reference(Box::new(type_));
                }
                _ => break,
            }
        }

        type_
    }

    fn parseParams(&mut self) -> Vec<AST> {
        let mut params = Vec::new();

        while *self.peekType() != TokenType::CloseRound {
            let var_type = self.consumeTypeName();
            let name = self.consumeValue();

            params.push(AST {
                Node: ASTNode::Declaration {
                    var_type,
                    name,
                    init: None,
                },
                Span: Span {
                    start_token: self.pos - 2,
                    end_token: self.pos,
                },
            });

            if *self.peekType() == TokenType::Comma {
                self.advance();
            }
        }

        params
    }

    fn parseBlock(&mut self) -> Vec<AST> {
        let mut body = Vec::new();
        self.expect(TokenType::OpenCurly);

        while *self.peekType() != TokenType::CloseCurly {
            body.push(self.parseStatement());
        }

        self.expect(TokenType::CloseCurly);
        body
    }

    fn parseStatement(&mut self) -> AST {
        let start_token = self.pos;

        if *self.peekType() == TokenType::OpenCurly {
            let block = self.parseBlock();
            return AST {
                Node: ASTNode::Compound(block),
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            };
        }

        if *self.peekType() == TokenType::Keyword {
            match self.peek().value.as_str() {
                "return" => {
                    self.advance();

                    let expr = if *self.peekType() != TokenType::Semicolon {
                        Some(Box::new(self.parseExpression()))
                    } else {
                        None
                    };

                    self.expect(TokenType::Semicolon);

                    return AST {
                        Node: ASTNode::Return(expr),
                        Span: Span {
                            start_token,
                            end_token: self.pos,
                        },
                    };
                }
                "if" => return self.parseIf(),
                "while" => return self.parseWhile(),
                "for" => return self.parseFor(),
                _ => return self.parseDeclaration(),
            }
        }

        if self.isDeclaration() {
            return self.parseDeclaration();
        }

        let expr = self.parseExpression();
        self.expect(TokenType::Semicolon);
        AST {
            Node: ASTNode::Expression(Box::new(expr)),
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        }
    }

    fn parseIf(&mut self) -> AST {
        let start_token = self.pos;
        self.advance();
        self.expect(TokenType::OpenRound);
        let condition = self.parseExpression();
        self.expect(TokenType::CloseRound);

        let then_branch = self.parseStatement();
        let else_branch = if *self.peekType() == TokenType::Keyword && self.peek().value == "else" {
            self.advance();
            self.parseStatement()
        } else {
            AST {
                Node: ASTNode::Compound(vec![]),
                Span: Span {
                    start_token: self.pos,
                    end_token: self.pos,
                },
            }
        };

        AST {
            Node: ASTNode::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        }
    }

    fn parseWhile(&mut self) -> AST {
        let start_token = self.pos;
        self.advance();
        self.expect(TokenType::OpenRound);
        let condition = self.parseExpression();
        self.expect(TokenType::CloseRound);
        let body = self.parseStatement();

        AST {
            Node: ASTNode::While {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        }
    }

    //  All of the for loops are going to be converted down into while loops since
    //      rust doesnt easily support classic for loops
    //      functionality should be identical though
    fn parseFor(&mut self) -> AST {
        let start_token = self.pos;
        self.advance();
        self.expect(TokenType::OpenRound);

        let init = if *self.peekType() == TokenType::Semicolon {
            self.advance();
            None
        } else if *self.peekType() == TokenType::Keyword {
            Some(self.parseDeclaration())
        } else {
            let expr = self.parseExpression();
            self.expect(TokenType::Semicolon);
            Some(AST {
                Node: ASTNode::Expression(Box::new(expr)),
                Span: Span {
                    start_token: start_token,
                    end_token: self.pos,
                },
            })
        };

        let condition = if *self.peekType() == TokenType::Semicolon {
            self.advance();
            AST {
                Node: ASTNode::Literal(LiteralType::Number("1".to_string())),
                Span: Span {
                    start_token: self.pos,
                    end_token: self.pos,
                },
            }
        } else {
            let expr = self.parseExpression();
            self.expect(TokenType::Semicolon);
            expr
        };

        let increment = if *self.peekType() == TokenType::CloseRound {
            None
        } else {
            Some(self.parseExpression())
        };
        self.expect(TokenType::CloseRound);

        let loop_body_stmt = self.parseStatement();
        let mut while_body = match loop_body_stmt.Node {
            ASTNode::Compound(stmts) => stmts,
            _ => vec![loop_body_stmt],
        };

        if let Some(inc) = increment {
            while_body.push(AST {
                Node: ASTNode::Expression(Box::new(inc)),
                Span: Span {
                    start_token: self.pos,
                    end_token: self.pos,
                },
            });
        }

        let while_ast = AST {
            Node: ASTNode::While {
                condition: Box::new(condition),
                body: Box::new(AST {
                    Node: ASTNode::Compound(while_body),
                    Span: Span {
                        start_token: self.pos,
                        end_token: self.pos,
                    },
                }),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        };

        if let Some(init_stmt) = init {
            AST {
                Node: ASTNode::Compound(vec![init_stmt, while_ast]),
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            }
        } else {
            while_ast
        }
    }

    fn parseExpression(&mut self) -> AST {
        self.parseAssignment()
    }

    fn parseAssignment(&mut self) -> AST {
        let start_token = self.pos;
        let left = self.parseLogicalOr();

        let op = match self.peekType() {
            TokenType::Equals
            | TokenType::PlusEquals
            | TokenType::MinusEquals
            | TokenType::AsteriskEquals
            | TokenType::SlashEquals
            | TokenType::PercentEquals
            | TokenType::AmpersandEquals
            | TokenType::PipeEquals
            | TokenType::CaretEquals => self.consumeType(),
            _ => return left,
        };

        let right = self.parseAssignment();

        AST {
            Node: ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        }
    }

    fn parseLogicalOr(&mut self) -> AST {
        self.parseBinary(
            //  Refrence to a list of valid tokens, some allow multiple types
            &[TokenType::PipePipe],
            Self::parseLogicalAnd, //  Passing a function to call to parseBinary, Explained more in that function
        )
    }

    fn parseLogicalAnd(&mut self) -> AST {
        self.parseBinary(&[TokenType::AmpersandAmpersand], Self::parseBitwiseOr)
    }

    fn parseBitwiseOr(&mut self) -> AST {
        self.parseBinary(&[TokenType::Pipe], Self::parseBitwiseXor)
    }

    fn parseBitwiseXor(&mut self) -> AST {
        self.parseBinary(&[TokenType::Caret], Self::parseBitwiseAnd)
    }

    fn parseBitwiseAnd(&mut self) -> AST {
        self.parseBinary(&[TokenType::Ampersand], Self::parseEquality)
    }

    fn parseEquality(&mut self) -> AST {
        self.parseBinary(
            &[TokenType::EqualsEquals, TokenType::NotEquals],
            Self::parseRelational,
        )
    }

    fn parseRelational(&mut self) -> AST {
        self.parseBinary(
            &[
                TokenType::OpenAngle,
                TokenType::CloseAngle,
                TokenType::LessEquals,
                TokenType::GreaterEquals,
            ],
            Self::parseShift,
        )
    }

    fn parseShift(&mut self) -> AST {
        self.parseBinary(
            &[TokenType::ShiftLeft, TokenType::ShiftRight],
            Self::parseAdditive,
        )
    }

    fn parseAdditive(&mut self) -> AST {
        self.parseBinary(
            &[TokenType::Plus, TokenType::Minus],
            Self::parseMultiplicative,
        )
    }

    fn parseMultiplicative(&mut self) -> AST {
        self.parseBinary(
            &[TokenType::Asterisk, TokenType::Slash, TokenType::Percent],
            Self::parseUnary,
        )
    }

    //  This is the part that actually handles the ordering of operations
    //  Functions call this function and pass in the next option.
    //      If the current Token is not valid for a given operation,
    //      it moves to the next
    fn parseBinary(&mut self, ops: &[TokenType], next: fn(&mut Self) -> AST) -> AST {
        let start_token = self.pos;
        let mut left = next(self);

        while ops.contains(self.peekType()) {
            let op = self.peekType().clone();
            self.advance();

            let right = next(self);
            left = AST {
                Node: ASTNode::Binary {
                    op: op.clone(),
                    left: Box::new(left),
                    right: Box::new(right),
                },
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            }
        }

        left
    }

    fn parseUnary(&mut self) -> AST {
        let start_token = self.pos;

        let valid_unary = &[
            TokenType::Exclamation,
            TokenType::Tilde,
            TokenType::Minus,
            TokenType::PlusPlus,
            TokenType::MinusMinus,
            TokenType::Ampersand,
            TokenType::Asterisk,
        ];

        if valid_unary.contains(self.peekType()) {
            let op = self.peekType().clone();
            self.advance();
            let expr = self.parseUnary();
            return AST {
                Node: ASTNode::Unary {
                    op,
                    expr: Box::new(expr),
                },
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            };
        }

        self.parsePostfix()
    }

    fn parsePostfix(&mut self) -> AST {
        let start_token = self.pos;
        let mut expr = self.parsePrimary();

        //  All other operators are right associative ie 1 * (2 * 3)
        //  But these ones are the opposite ie ( function.call() ).call2()
        //      So loop over recursion since decrementing the token counter is frowned apon
        loop {
            match self.peekType() {
                //  funcion calls
                TokenType::Period | TokenType::Arrow | TokenType::ColonColon => {
                    let op = self.peekType().clone();
                    self.advance();
                    let field = self.peek().value.clone();
                    self.advance();
                    expr = AST {
                        Node: ASTNode::Binary {
                            op,
                            left: Box::new(expr),
                            right: Box::new(AST {
                                Node: ASTNode::Identifier(field),
                                Span: Span {
                                    start_token: self.pos - 1,
                                    end_token: self.pos,
                                },
                            }),
                        },
                        Span: Span {
                            start_token,
                            end_token: self.pos,
                        },
                    };
                }
                //  Array indexing
                TokenType::OpenSquare => {
                    self.advance();
                    let index = self.parseExpression();
                    self.expect(TokenType::CloseSquare);
                    expr = AST {
                        //  Using opensquare as the operation since there isnt a specific one for this
                        Node: ASTNode::Binary {
                            op: TokenType::OpenSquare,
                            left: Box::new(expr),
                            right: Box::new(index),
                        },
                        Span: Span {
                            start_token,
                            end_token: self.pos,
                        },
                    }
                }
                // variable++ / variable--
                TokenType::PlusPlus | TokenType::MinusMinus => {
                    expr = AST {
                        Node: ASTNode::Unary {
                            op: self.peekType().clone(),
                            expr: Box::new(expr),
                        },
                        Span: Span {
                            start_token,
                            end_token: self.pos + 1,
                        },
                    };
                    //  post increment to keep token peeking to operation
                    self.advance();
                }

                //  Explicit case where a function returns a function refrence that is immediatly called
                //  Example "function(parameter1)(parameter2)"
                TokenType::OpenRound => {
                    self.advance();
                    let args = self.parseArguments();
                    self.expect(TokenType::CloseRound);

                    let name = self.extract_name(&Box::new(expr));
                    expr = AST {
                        Node: ASTNode::Call {
                            function_name: name,
                            arguments: args,
                        },
                        Span: Span {
                            start_token,
                            end_token: self.pos,
                        },
                    };
                }
                //  If we see anything else, its back to right associative, so exit loop
                _ => break,
            }
        }

        expr
    }

    //  This handles function calls in relation to type's
    //      Example: io::stdout() or node.value.to_string()
    //  Also handles the macro override functions
    //      Macro::print -> print!
    fn extract_name(&self, node: &Box<AST>) -> String {
        match &node.Node {
            ASTNode::Identifier(name) => name.clone(),
            ASTNode::Binary { op, left, right } => {
                let sep = match op {
                    TokenType::ColonColon => "::",
                    TokenType::Period => ".",
                    TokenType::Arrow => "->",
                    _ => {
                        println!("TESTING");
                        panic!("Unexpected operator in call target")},
                };
                let leftName = self.extract_name(left);
                if leftName.chars().any(|c| matches!(c, ':' | '.' | '>')) {
                    return format!("{}{}{}", leftName, sep, self.extract_name(right));
                }

                if leftName == "Macro" {
                    return format!("{}!", self.extract_name(right));
                }

                format!("{}{}{}", leftName, sep, self.extract_name(right))
            }
            _ => panic!("Unresolvable call target"),
        }
    }

    fn parsePrimary(&mut self) -> AST {
        let start = self.pos;

        match self.peekType() {
            //  Parenthesised expression
            TokenType::OpenRound => {
                self.advance();
                let expr = self.parseExpression();
                self.expect(TokenType::CloseRound);
                expr
            }

            //  Integer / float literals
            TokenType::Integer | TokenType::Float => {
                let val = self.peek().value.clone();
                self.advance();
                AST {
                    Node: ASTNode::Literal(LiteralType::Number(val)),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                }
            }

            //  Char / string literals
            TokenType::StringLiteral => {
                let val = self.peek().value.clone();
                self.advance();
                AST {
                    Node: ASTNode::Literal(LiteralType::String(val)),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                }
            }
            TokenType::CharLiteral => {
                let val = self.peek().value.clone();
                self.advance();
                AST {
                    Node: ASTNode::Literal(LiteralType::Char(val)),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                }
            }

            //  Identifier
            TokenType::Identifier => {
                let name = self.peek().value.clone();
                self.advance();

                if *self.peekType() == TokenType::OpenRound {
                    
                    //  Constructor mapping.
                    //  if the function name matches an existing struct name, replace the function call with "name::new()"
                    let function_name = if self.struct_names.contains(&name) {
                        format!("{}::new", name)
                    } else {
                        name
                    };


                    self.advance();
                    let arguments = self.parseArguments();
                    self.expect(TokenType::CloseRound);
                    AST {
                        Node: ASTNode::Call {
                            function_name,
                            arguments,
                        },
                        Span: Span {
                            start_token: start,
                            end_token: self.pos,
                        },
                    }
                } else {
                    AST {
                        Node: ASTNode::Identifier(name),
                        Span: Span {
                            start_token: start,
                            end_token: self.pos,
                        },
                    }
                }
            }

            _ => {
                println!("TESTING");
                panic!("Unexpected token in expression: {:?}", self.peek());
            }
        }
    }

    fn parseArguments(&mut self) -> Vec<AST> {
        let mut args = Vec::new();

        while *self.peekType() != TokenType::CloseRound {
            args.push(self.parseExpression());

            if *self.peekType() == TokenType::Comma {
                self.advance();
            }
        }

        args
    }

    fn peek(&self) -> &Token {
        &(self.tokens[self.pos])
    }

    fn peekType(&self) -> &TokenType {
        &self.tokens[self.pos].token_type
    }

    fn has_tokens(&self) -> bool {
        self.pos < self.tokens.len()
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    //  cant borrow out a token type, then advance i guess?
    //      Honestly i still dont get borrowing well enough but this works so
    fn consumeType(&mut self) -> TokenType {
        let t = self.tokens[self.pos].token_type.clone();
        self.pos += 1;
        t
    }
    fn consumeValue(&mut self) -> String {
        let t = self.tokens[self.pos].value.clone();
        self.pos += 1;
        t
    }

    fn expect(&mut self, t: TokenType) {
        if *self.peekType() != t {
            panic!(
                "Expected {:?}, got {:?} at {:?}",
                t,
                self.peekType(),
                self.pos
            );
            //  TODO: Improve error handling here
        }
        self.advance();
    }
}
