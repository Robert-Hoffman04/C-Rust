use crate::CRustError;
use super::tokenizer::{Token, TokenType};
use std::fmt;
use std::io::{self, Write};
//  Anonymous enums need an actual name for rust to function properly
use uuid::Uuid;

/*
    Big disclaimer is that I really dont know why the parsing code is done in this way.
    This is what I can half remember from 3 years ago when I wrote a C parser in python
    But it works so like, guess its fine?
*/

///  Simplify keeping track of token locations from original file
// This made more sense before but i changed some stuff so
#[derive(Debug, Clone)]
pub struct Span {
    start_token: usize,
    end_token: usize,
}

/// Tie a node to its token positions
#[derive(Debug, Clone)]
pub struct AST {
    pub Node: ASTNode,
    pub Span: Span,
}

#[derive(Debug, Clone)]
//  Tokens were a stuct because they largly had the same structure,
//  This is an enum because each individual type has very specific requirments
/// Main typing system to hold data about each code "action"
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
        callee: Box<AST>,
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

    //  for things like continue and break that are single tokens
    GenericKeyword(String)
}

/// Stores functions or variables for a struct in the same type
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

/// Stores raw values in a single type
#[derive(Debug, Clone)]
pub enum LiteralType {
    String(String),
    Char(String),
    Number(String),
}

/// Recursive struct to keep track of all the modifiers on a given type
/// Example `static const Custom<int>*`
#[derive(Debug, Clone)]
pub enum CType {
    Named(String),
    Pointer(Box<CType>),
    Reference(Box<CType>),
    Const(Box<CType>),
    Template(String, Vec<CType>),
}
/// Recursivly decend the type list to make sure its printed properly
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

///  helper function becasue printing string literals wasnt actually escaped
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
    //  It was really hard to debug the parser without a good way to visualize it
    //  So i went and 'stole' some code from my first ever github pull-request
    //      https://github.com/eliben/pycparser/pull/518
    //  Did it get accepted? No, but that doesnt mean it didnt work

    /// Recursive entry point for righting a current node branch
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

    ///  Map a node type to a a human readable string
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
            ASTNode::GenericKeyword { .. } => "GenericKeyword"
        }
    }

    ///  helper to extract all the relevant data from a node
    ///      for example the name of a function being called
    fn attrs(&self, attrnames: bool) -> Vec<String> {
        let fmt = |name: &str, val: &str| -> String {
            if attrnames {
                format!("{}={}", name, val)
            } else {
                val.to_string()
            }
        };

        //  Get diffrent data depending on the tpye of the node
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
            ASTNode::Call { .. } => vec![],
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
            ASTNode::GenericKeyword(keyword) => vec![fmt("Keyword", keyword)],

            //  these dont have any data only children
            ASTNode::Root(_)
            | ASTNode::Compound(_)
            | ASTNode::If { .. }
            | ASTNode::While { .. }
            | ASTNode::Return(_)
            | ASTNode::Expression(_) => vec![],
        }
    }

    ///  Returns all child ASTNodes with their relationship label
    ///      for example the seperate then and else blocks of a if statment
    fn children(&self) -> Vec<(&'static str, &ASTNode)> {
        //  diffrent nodes have diffrent kind of children
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

            ASTNode::Call { callee, arguments } => {
                let mut callChildren: Vec<(&'static str, &ASTNode)> = vec![("callee", &callee.Node)];
                callChildren.extend(arguments.iter().map(|a| ("arg", &a.Node)));
                callChildren
            }

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
            },
            ASTNode::GenericKeyword(_) => vec![]
        }
    }
}

//  just maping the display function to the tree printer
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

    struct_names: Vec<String>,
}

impl Parser {
    /// Construtor for the parse class
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            struct_names: vec![],
        }
    }

    /// Actual parse function called by main
    pub fn parse(&mut self) -> Result<AST, CRustError> {
        //  Scan the incoming token list for any structure definitions ahead of time
        //      this lets me later change their constructors into the proper "::new()" function
        let mut willBeStruct = false;
        for token in &self.tokens {
            if token.value == "struct" {
                willBeStruct = true;
                continue;
            }

            if willBeStruct {
                self.struct_names.push(token.value.clone());
            }
            willBeStruct = false;
        }

        let mut nodes: Vec<AST> = Vec::new();

        //  Iterate though all of the tokens that were passed, running diffrent functions based on type
        while self.pos < self.tokens.len() {
            match self.peekType() {
                TokenType::Include => nodes.push(self.parseInclude()?),
                TokenType::Keyword => match self.peek().value.as_str() {
                    //  the typedef diffrence ended up being kind of pointless in the end
                    //      but kept because im not rewriting a bunch of code
                    "typedef" => {
                        let def = self.parseTypedef()?;
                        match def {
                            Some(val) => nodes.push(val),
                            None => {}
                        }
                    }

                    "enum" => {
                        if let Some(node) = self.parseEnum()? {
                            nodes.push(node);
                        }
                    }
                    "struct" => {
                        nodes.push(self.parseStruct()?);
                    }
                    //  Anything that wasnt part of the existing types there is must be a type name for a delecration or function
                    _ => nodes.push(self.parseDeclarationOrFunction(true)?),
                },
                //  Since you might have imported rust types identifers could also be a valid type
                TokenType::Identifier => nodes.push(self.parseDeclarationOrFunction(true)?),
                _ => self.advance(),
            }
        }

        //  Create the final root node to be returned
        Ok(AST {
            Node: ASTNode::Root(nodes.into_iter().map(|n| n.Node).collect()),
            Span: Span {
                start_token: 0,
                end_token: self.tokens.len(),
            },
        })
    }

    /// Converted include tokens to a single include node in the tree for the writer to finally use
    fn parseInclude(&mut self) -> Result<AST, CRustError> {
        let inc = AST {
            Node: ASTNode::Include(self.peek().value.clone()),
            Span: Span {
                start_token: self.pos,
                end_token: self.pos,
            },
        };
        self.advance();
        Ok(inc)
    }

    /// Decideds if an enum or struct comes next, and also where the name for it is
    fn parseTypedef(&mut self) -> Result<Option<AST>, CRustError> {
        self.advance();

        match self.peek().value.as_str() {
            "enum" => {
                let mut node = match self.parseEnum()? {
                    Some(val) => val,
                    None => return Ok(None)
                };

                if *self.peekType() != TokenType::Semicolon {
                    let alias = self.consumeValue();
                    //  Replace node name in place to use new name
                    if let ASTNode::Enum { ref mut name, .. } = node.Node {
                        *name = alias;
                    }
                }
                self.expect(TokenType::Semicolon)?;
                Ok(Some(node))
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
                self.expect(TokenType::Semicolon)?;
                Ok(Some(node))
            }
            _ => Ok(Some(self.parseDeclarationOrFunction(true)?)),
        }
    }

    /// parse enums into the enum node
    fn parseEnum(&mut self) -> Result<Option<AST>, CRustError> {
        let start = self.pos;
        self.advance(); //  alwasy just "enum"

        //  If the enum is anonymus, create a fake name for it to use in rust
        let name = if matches!(self.peekType(), TokenType::Identifier) {
            self.consumeValue()
        } else {
            format!("enum_{}", Uuid::new_v4().to_string().replace("-", "_"))
        };

        //  Look for any implements that it defines
        //      should really only be debug or clone
        let mut implements = vec![];
        if matches!(self.peekType(), TokenType::Keyword) && self.peek().value == "impliments" {
            self.advance();

            loop {
                if !matches!(self.peekType(), TokenType::Identifier) {
                    self.expect(TokenType::Identifier)?;
                }

                implements.push(self.consumeValue());

                if matches!(self.peekType(), TokenType::Comma) {
                    self.advance();
                    continue;
                }
                if matches!(self.peekType(), TokenType::OpenCurly) {
                    break;
                }

                self.expect(TokenType::OpenCurly)?;
            }
        }

        //  C techincally supports declaration of enums without actually defining them
        //      The later definition is the exact same format as a proper initilization
        //      so we can just throw this kind of def away
        if matches!(self.peekType(), TokenType::Semicolon) {
            return Ok(None);
        }

        self.expect(TokenType::OpenCurly)?;

        //  Look through all of the enum options and save them for the writer
        let mut options = vec![];
        loop {
            if !matches!(self.peekType(), TokenType::Identifier) {
                self.expect(TokenType::Identifier)?;
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

            self.expect(TokenType::CloseCurly)?;
        }

        //  Actual enum node, has to be some since this function might not actaully define the enum
        Ok(Some(AST {
            Node: ASTNode::Enum {
                name,
                implements,
                options,
            },
            Span: Span {
                start_token: start,
                end_token: self.pos,
            },
        }))
    }

    /// Parses the struct data and all of its function and variables
    fn parseStruct(&mut self) -> Result<AST, CRustError> {
        let start = self.pos;
        self.advance();

        //  You have to name you structs
        let name = if matches!(self.peekType(), TokenType::Identifier) {
            self.consumeValue()
        } else {
            self.expect(TokenType::Identifier)?;
            String::from("") //  Never actually returns this since this will force an expect panic
        };

        //  Scan for all the impliments the same way as for enums
        let mut implements = vec![];
        if matches!(self.peekType(), TokenType::Keyword) && self.peek().value == "impliments" {
            self.advance();

            loop {
                if !matches!(self.peekType(), TokenType::Identifier) {
                    self.expect(TokenType::Identifier)?;
                }

                implements.push(self.consumeValue());

                if matches!(self.peekType(), TokenType::Comma) {
                    self.advance();
                    continue;
                }
                if matches!(self.peekType(), TokenType::OpenCurly) {
                    break;
                }

                self.expect(TokenType::OpenCurly)?;
            }
        }

        self.expect(TokenType::OpenCurly)?;

        let mut members = vec![];
        let mut visibility = true;
        while !matches!(self.peekType(), TokenType::CloseCurly) {
            //  Read off the public / private settings
            if matches!(self.peekType(), TokenType::Identifier)
                && ["public", "private"].contains(&self.peek().value.as_str())
            {
                visibility = matches!(self.consumeValue().as_str(), "public");
                self.expect(TokenType::Colon)?;
            }

            //  mark the current possition as the start of a decleration
            let member_start = self.pos;
            let var_type = self.consumeTypeName();

            //  decide if the member is a variable, function, or function implementation (like from Display impl)
            let is_func_implement = matches!(self.peekType(), TokenType::Identifier)
                && self.pos + 1 < self.tokens.len()
                && matches!(self.tokens[self.pos + 1].token_type, TokenType::ColonColon);

            let is_function = matches!(self.peekType(), TokenType::OpenRound)
                || (matches!(self.peekType(), TokenType::Identifier)
                    && self.pos + 1 < self.tokens.len()
                    && matches!(self.tokens[self.pos + 1].token_type, TokenType::OpenRound));

            //  Push the generated function AST or Variable Node to the list of members
            if is_func_implement || is_function {
                self.pos = member_start;
                //  notConstructor=false only when the type name matches the struct name
                let not_constructor = var_type.to_string() != name || is_func_implement;
                let func = self.parseDeclarationOrFunction(not_constructor)?;
                members.push(StructMember::Function { visibility, func });
            }
            //  just a declaration
            else {
                self.pos = member_start;
                let declaration = self.parseDeclarationOrFunction(true)?;
                members.push(StructMember::Variable {
                    visibility,
                    declaration,
                })
            }
        }

        self.expect(TokenType::CloseCurly)?;

        //  Generate the actaul AST for the
        Ok(AST {
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

    /// Decide if the current position is a declaration of function since they have the same start
    fn parseDeclarationOrFunction(&mut self, notConstructor: bool) -> Result<AST, CRustError> {
        let start_index = self.pos;
        let type_ = self.consumeTypeName();

        //  If the function is a constructor (only ever true in a struct)
        //      skip reading a name (since it doesnt exist)
        //      and instead just use "new"
        let mut name = String::from("new");
        if notConstructor {
            name = self.peek().value.clone();
            self.advance();
        }

        match self.peekType() {
            //  This means its a struct function defined outside of a struct
            //  OR its a trait implementation in a struct
            //  Either way its handled the same and the writer figures it out
            TokenType::ColonColon => {
                self.advance();
                let method_name = self.consumeValue();
                self.expect(TokenType::OpenRound)?;
                let params = self.parseParams();
                self.expect(TokenType::CloseRound)?;
                let body = self.parseBlock()?;

                Ok(AST {
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
                })
            }
            //  Its a function
            TokenType::OpenRound => {
                self.advance();

                let params = self.parseParams();

                self.expect(TokenType::CloseRound)?;

                let body = match self.peekType() {
                    TokenType::OpenCurly => Some(self.parseBlock()?),
                    _ => {
                        self.expect(TokenType::Semicolon)?;
                        None
                    }
                };

                Ok(AST {
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
                })
            }
            //  Not a function
            _ => {
                let mut value = None;
                if *self.peekType() == TokenType::Equals {
                    self.advance();
                    value = Some(Box::new(self.parseExpression()?));
                }

                self.expect(TokenType::Semicolon)?;

                Ok(AST {
                    Node: ASTNode::Declaration {
                        var_type: type_,
                        name,
                        init: value,
                    },
                    Span: Span {
                        start_token: start_index,
                        end_token: self.pos,
                    },
                })
            }
        }
    }

    /// Parse variable declarations
    fn parseDeclaration(&mut self) -> Result<AST, CRustError> {
        let start_index = self.pos;
        let var_type = self.consumeTypeName();
        let name = self.consumeValue();

        let mut init = None;
        if self.has_tokens() && *self.peekType() == TokenType::Equals {
            self.advance();
            init = Some(Box::new(self.parseExpression()?));
        }

        self.expect(TokenType::Semicolon)?;

        Ok(AST {
            Node: ASTNode::Declaration {
                var_type,
                name,
                init,
            },
            Span: Span {
                start_token: start_index,
                end_token: self.pos,
            },
        })
    }

    /// Lookahead in the token stream to decide if its a dcleration or not
    fn isDeclaration(&self) -> bool {
        let current_is_type = matches!(self.peekType(), TokenType::Keyword | TokenType::Identifier);

        let mut lookahead = self.pos + 1;

        //  Skip past any pointer stars to find the name token
        if lookahead < self.tokens.len()
            && self.tokens[lookahead].token_type == TokenType::OpenAngle
        {
            let mut depth = 1;
            lookahead += 1;
            //  Might see something like "Vec<Vec<int>>" so we need to track internal type depth
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
            && self.tokens[lookahead].token_type == TokenType::Ampersand
        {
            lookahead += 1;
        }

        let next_is_name = lookahead < self.tokens.len()
            && matches!(self.tokens[lookahead].token_type, TokenType::Identifier);

        current_is_type && next_is_name
    }

    ///  Helper to make sure type names include other portions
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

    /// Helper for extracting parameters from function declerations 
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

    /// Parse the inside of a set of curly brackets
    fn parseBlock(&mut self) -> Result<Vec<AST>, CRustError> {
        let mut body = Vec::new();
        self.expect(TokenType::OpenCurly)?;

        while *self.peekType() != TokenType::CloseCurly {
            body.push(self.parseStatement()?);
        }

        self.expect(TokenType::CloseCurly)?;
        Ok(body)
    }

    /// parse the insides of ifs, whiles, fors, etc that have optional brackets or single lines
    fn parseStatement(&mut self) -> Result<AST, CRustError> {
        let start_token = self.pos;

        //  if we see a curly, it is a block so run that 
        if *self.peekType() == TokenType::OpenCurly {
            let block = self.parseBlock()?;
            return Ok(AST {
                Node: ASTNode::Compound(block),
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            });
        }

        //  otherwise, its a single line so figure out what its doing
        if *self.peekType() == TokenType::Keyword {
            match self.peek().value.as_str() {
                "return" => {
                    self.advance();

                    let expr = if *self.peekType() != TokenType::Semicolon {
                        Some(Box::new(self.parseExpression()?))
                    } else {
                        None
                    };

                    self.expect(TokenType::Semicolon)?;

                    return Ok(AST {
                        Node: ASTNode::Return(expr),
                        Span: Span {
                            start_token,
                            end_token: self.pos,
                        },
                    });
                }
                "if" => return self.parseIf(),
                "while" => return self.parseWhile(),
                "for" => return self.parseFor(),
                "continue" => {
                    self.advance();
                    self.expect(TokenType::Semicolon)?;
                    return Ok(AST {
                        Node: ASTNode::GenericKeyword(String::from("continue")),
                        Span: Span { start_token, end_token: self.pos },
                    });
                }
                "break" => {
                    self.advance();
                    self.expect(TokenType::Semicolon)?;
                    return Ok(AST {
                        Node: ASTNode::GenericKeyword(String::from("break")),
                        Span: Span { start_token, end_token: self.pos },
                    });
                }
                _ => return self.parseDeclaration(),
            }
        }

        if self.isDeclaration() {
            return self.parseDeclaration();
        }

        let expr = self.parseExpression()?;
        self.expect(TokenType::Semicolon)?;
        Ok(AST {
            Node: ASTNode::Expression(Box::new(expr)),
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        })
    }

    /// parse if statments
    fn parseIf(&mut self) -> Result<AST, CRustError> {
        let start_token = self.pos;
        self.advance();
        self.expect(TokenType::OpenRound)?;
        let condition = self.parseExpression()?;
        self.expect(TokenType::CloseRound)?;

        let then_branch = self.parseStatement()?;
        let else_branch = if *self.peekType() == TokenType::Keyword && self.peek().value == "else" {
            self.advance();
            self.parseStatement()?
        } else {
            AST {
                Node: ASTNode::Compound(vec![]),
                Span: Span {
                    start_token: self.pos,
                    end_token: self.pos,
                },
            }
        };

        Ok(AST {
            Node: ASTNode::If {
                condition: Box::new(condition),
                then_branch: Box::new(then_branch),
                else_branch: Box::new(else_branch),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        })
    }

    /// parse while statements
    fn parseWhile(&mut self) -> Result<AST, CRustError> {
        let start_token = self.pos;
        self.advance();
        self.expect(TokenType::OpenRound)?;
        let condition = self.parseExpression()?;
        self.expect(TokenType::CloseRound)?;
        let body = self.parseStatement()?;

        Ok(AST {
            Node: ASTNode::While {
                condition: Box::new(condition),
                body: Box::new(body),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        })
    }

    ///  All of the for loops are going to be converted down into while loops since
    ///      rust doesnt easily support classic for loops
    ///      functionality should be identical though
    fn parseFor(&mut self) -> Result<AST, CRustError> {
        let start_token = self.pos;
        self.advance();
        self.expect(TokenType::OpenRound)?;

        //  look for the first part of the if
        //      may be empty
        let init = if *self.peekType() == TokenType::Semicolon {
            self.advance();
            None
        } else if *self.peekType() == TokenType::Keyword {
            //  full normal decleration
            Some(self.parseDeclaration()?)
        } else {
            //  other cases such as
            //      int i;
            //      for (i = 0; i < 10; i++)
            //  ie, not a full decleration to start with
            let expr = self.parseExpression()?;
            self.expect(TokenType::Semicolon)?;
            Some(AST {
                Node: ASTNode::Expression(Box::new(expr)),
                Span: Span {
                    start_token: start_token,
                    end_token: self.pos,
                },
            })
        };

        //  get the middle portion
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
            let expr = self.parseExpression()?;
            self.expect(TokenType::Semicolon)?;
            expr
        };

        //  get the ending operation
        let increment = if *self.peekType() == TokenType::CloseRound {
            None
        } else {
            Some(self.parseExpression()?)
        };
        self.expect(TokenType::CloseRound)?;

        //  parse the inside of the while loop
        let loop_body_stmt = self.parseStatement()?;
        let mut while_body = match loop_body_stmt.Node {
            ASTNode::Compound(stmts) => stmts,
            _ => vec![loop_body_stmt],
        };

        //  add the increment portion of the for loop to the end of the loop body
        if let Some(inc) = increment {
            while_body.push(AST {
                Node: ASTNode::Expression(Box::new(inc)),
                Span: Span {
                    start_token: self.pos,
                    end_token: self.pos,
                },
            });
        }

        //  create a while loop with the same condition and the added increment at the end
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

        //  Put the initlializer at the before the while loop if necesary
        if let Some(init_stmt) = init {
            Ok(AST {
                Node: ASTNode::Compound(vec![init_stmt, while_ast]),
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            })
        } else {
            Ok(while_ast)
        }
    }

    /// Entry point into the massive recursive stack that is expression parsing
    fn parseExpression(&mut self) -> Result<AST, CRustError> {
        self.parseAssignment()
    }

    /*
    
        Im choosing here to try and explain the next dozen function because they basically all do the same thing
        To preserve order of operations in the parse tree, all expressions basically get forced parethesis.
        This is represented as an expression being a child of an expresion being a child of an expression (etc...)
        These function just decend upwards looking for the highest priority operation to either side of them, and 
        choosing which it should be with based on that

        I dont full understand the reasoning for doing it like this, but the last time i tried to write a parser this was how I did it so
    
     */


    /// Parses anything that is setting a value to a variable
    fn parseAssignment(&mut self) -> Result<AST, CRustError> {
        let start_token = self.pos;

        //  let the recursion begin
        let left = self.parseLogicalOr()?;

        //  is it a valid operation for assignment
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
            _ => return Ok(left),
        };

        let right = self.parseAssignment()?;

        Ok(AST {
            Node: ASTNode::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            },
            Span: Span {
                start_token,
                end_token: self.pos,
            },
        })
    }

    fn parseLogicalOr(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(
            //  Refrence to a list of valid tokens, some allow multiple types
            &[TokenType::PipePipe],
            Self::parseLogicalAnd, //  Passing a function to call to parseBinary, Explained more in that function
        )
    }

    fn parseLogicalAnd(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(&[TokenType::AmpersandAmpersand], Self::parseBitwiseOr)
    }

    fn parseBitwiseOr(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(&[TokenType::Pipe], Self::parseBitwiseXor)
    }

    fn parseBitwiseXor(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(&[TokenType::Caret], Self::parseBitwiseAnd)
    }

    fn parseBitwiseAnd(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(&[TokenType::Ampersand], Self::parseEquality)
    }

    fn parseEquality(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(
            &[TokenType::EqualsEquals, TokenType::NotEquals],
            Self::parseRelational,
        )
    }

    fn parseRelational(&mut self) -> Result<AST, CRustError> {
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

    fn parseShift(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(
            &[TokenType::ShiftLeft, TokenType::ShiftRight],
            Self::parseAdditive,
        )
    }

    fn parseAdditive(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(
            &[TokenType::Plus, TokenType::Minus],
            Self::parseMultiplicative,
        )
    }

    fn parseMultiplicative(&mut self) -> Result<AST, CRustError> {
        self.parseBinary(
            &[TokenType::Asterisk, TokenType::Slash, TokenType::Percent],
            Self::parseUnary,
        )
    }

    ///  This is the part that actually handles the ordering of operations
    ///  Functions call this function and pass in the next option.
    ///      If the current Token is not valid for a given operation,
    ///      it moves to the next
    fn parseBinary(&mut self, ops: &[TokenType], next: fn(&mut Self) -> Result<AST, CRustError>) -> Result<AST, CRustError> {
        let start_token = self.pos;
        let mut left = next(self)?;

        while ops.contains(self.peekType()) {
            let op = self.peekType().clone();
            self.advance();

            let right = next(self)?;
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

        Ok(left)
    }

    /// unary operations like not or negation or ++/-- or pointer stuff
    fn parseUnary(&mut self) -> Result<AST, CRustError> {
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
            let expr = self.parseUnary()?;
            return Ok(AST {
                Node: ASTNode::Unary {
                    op,
                    expr: Box::new(expr),
                },
                Span: Span {
                    start_token,
                    end_token: self.pos,
                },
            });
        }

        self.parsePostfix()
    }

    /// Any operatiors that are on the right hand side
    fn parsePostfix(&mut self) -> Result<AST, CRustError> {
        let start_token = self.pos;
        let mut expr = self.parsePrimary()?;

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
                    let index = self.parseExpression()?;
                    self.expect(TokenType::CloseSquare)?;
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

                //  Explicit case where a function returns and is immediatly called
                //  Example "function(parameter1).function2(parameter2)"
                TokenType::OpenRound => {
                    self.advance();
                    let args = self.parseArguments()?;
                    self.expect(TokenType::CloseRound)?;

                    expr = AST {
                        Node: ASTNode::Call {
                            callee: Box::new(expr),
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

        Ok(expr)
    }

    /// Parse the lead nodes of all the expressions
    /// any actual values or function calls end up here
    /// also handles manual parethesis
    fn parsePrimary(&mut self) -> Result<AST, CRustError> {
        let start = self.pos;

        match self.peekType() {
            //  Parenthesised expression
            TokenType::OpenRound => {
                self.advance();
                let expr = self.parseExpression()?;
                self.expect(TokenType::CloseRound)?;
                Ok(expr)
            }

            //  Integer / float literals
            TokenType::Integer | TokenType::Float => {
                let val = self.peek().value.clone();
                self.advance();
                Ok(AST {
                    Node: ASTNode::Literal(LiteralType::Number(val)),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                })
            }

            //  Char / string literals
            TokenType::StringLiteral => {
                let val = self.peek().value.clone();
                self.advance();
                Ok(AST {
                    Node: ASTNode::Literal(LiteralType::String(val)),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                })
            }
            TokenType::CharLiteral => {
                let val = self.peek().value.clone();
                self.advance();
                Ok(AST {
                    Node: ASTNode::Literal(LiteralType::Char(val)),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                })
            }

            //  Identifier
            TokenType::Identifier => {
                let mut name = self.peek().value.clone();
                self.advance();

                if name == "Macro" && *self.peekType() == TokenType::ColonColon {
                    self.advance(); // consume '::'

                    if *self.peekType() != TokenType::Identifier {
                        self.expect(TokenType::Identifier)?;
                    }

                    let inner = self.peek().value.clone();
                    self.advance();

                    // Replace with "(any)!"
                    name = format!("{}!", inner);
                }

                if *self.peekType() == TokenType::OpenRound {
                    // Map struct constructors to StructName::new
                    let callee_node = if self.struct_names.contains(&name) {
                        // Build a Binary node here with a :: operator
                        AST {
                            Node: ASTNode::Binary {
                                op: TokenType::ColonColon,
                                left: Box::new(AST {
                                    Node: ASTNode::Identifier(name.clone()),
                                    Span: Span {
                                        start_token: start,
                                        end_token: start + 1,
                                    },
                                }),
                                right: Box::new(AST {
                                    Node: ASTNode::Identifier("new".to_string()),
                                    Span: Span {
                                        start_token: start + 1,
                                        end_token: start + 2,
                                    },
                                }),
                            },
                            Span: Span {
                                start_token: start,
                                end_token: start + 2,
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
                    };

                    self.advance(); // consume '('
                    let arguments = self.parseArguments()?;
                    self.expect(TokenType::CloseRound)?;

                    Ok(AST {
                        Node: ASTNode::Call {
                            callee: Box::new(callee_node),
                            arguments,
                        },
                        Span: Span {
                            start_token: start,
                            end_token: self.pos,
                        },
                    })
                } else {
                    Ok(AST {
                        Node: ASTNode::Identifier(name),
                        Span: Span {
                            start_token: start,
                            end_token: self.pos,
                        },
                    })
                }
            }

            _ => {
                Err(CRustError::ParseError {
                    message: format!("Unexpected token in expression: {:?}", self.peek()), 
                    token_idx: self.pos
                })
            }
        }
    }

    /// helper to get function call arguments
    fn parseArguments(&mut self) -> Result<Vec<AST>, CRustError> {
        let mut args = Vec::new();

        while *self.peekType() != TokenType::CloseRound {
            args.push(self.parseExpression()?);

            if *self.peekType() == TokenType::Comma {
                self.advance();
            }
        }

        Ok(args)
    }

    /// what is the next token?
    fn peek(&self) -> &Token {
        &(self.tokens[self.pos])
    }

    /// what is the next token type?
    fn peekType(&self) -> &TokenType {
        &self.tokens[self.pos].token_type
    }

    /// are there tokens left?
    fn has_tokens(&self) -> bool {
        self.pos < self.tokens.len()
    }

    /// move token position forward
    fn advance(&mut self) {
        self.pos += 1;
    }

    /// get ownership of token type and advance 
    fn consumeType(&mut self) -> TokenType {
        let t = self.tokens[self.pos].token_type.clone();
        self.pos += 1;
        t
    }
    /// get ownership of token value and advance 
    fn consumeValue(&mut self) -> String {
        let t = self.tokens[self.pos].value.clone();
        self.pos += 1;
        t
    }

    /// Fail if the token passed is not the token we are seeing
    fn expect(&mut self, t: TokenType) -> Result<(), CRustError>
    {
        if *self.peekType() != t {
            return Err(CRustError::ParseError
            {
                message : format!("Expected {:?}, got {:?}",t,self.peekType()),
                token_idx : self.pos
            });
            //  TODO: Improve error handling here
        }
        self.advance();

        Ok(())
    }
}
