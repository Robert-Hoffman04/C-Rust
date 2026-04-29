use super::tokenizer::{Token, TokenType};
use std::fmt;
use std::io::{self, Write};

//  Simplify keeping track of token locations from original file,
#[derive(Debug, Clone)]
pub struct Span {
    start_token: usize,
    end_token: usize,
}

#[derive(Debug)]
pub struct AST {
    pub Node: ASTNode,
    pub Span: Span,
}

#[derive(Debug)]
//  Tokens were a stuct because they largly had the same structure,
//  This is an enum because each individual type has very specific requirments
pub enum ASTNode {
    Root(Vec<ASTNode>), //    The Head of all AST's

    Include(String), //    Include propigation for local and external rust files

    Function {
        return_type: String,
        name: String,
        params: Vec<AST>,
        body: Vec<AST>,
    },

    Declaration {
        var_type: String,
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

    Literal(String), //  Any raw value. "A String", 'c' (char), 123 (Int), 3.14 (float)

    Identifier(String), //  variable names

    Call {
        function_name: String,
        arguments: Vec<AST>,
    },
}

//  helper function becasue printing string literals wasnt actually escaped
fn escape_str(s: &str) -> String {
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
            ASTNode::Literal(_) => "Literal",
            ASTNode::Identifier(_) => "Identifier",
            ASTNode::Call { .. } => "Call",
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
            ASTNode::Literal(val) => vec![fmt("value", &escape_str(val))],
            ASTNode::Identifier(name) => vec![fmt("name", &escape_str(name))],

            ASTNode::Binary { op, .. } => vec![fmt("op", (op.to_string()).as_str())],
            ASTNode::Unary { op, .. } => vec![fmt("op", (op.to_string()).as_str())],

            ASTNode::Function {
                return_type, name, ..
            } => vec![fmt("return_type", return_type), fmt("name", name)],
            ASTNode::Declaration { var_type, name, .. } => {
                vec![fmt("var_type", var_type), fmt("name", name)]
            }
            ASTNode::Call { function_name, .. } => vec![fmt("function_name", function_name)],

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
            ASTNode::Include(_) | ASTNode::Literal(_) | ASTNode::Identifier(_) => vec![],
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
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            includes: vec![],
            external: vec![],
        }
    }

    pub fn parse(&mut self) -> AST {
        let mut nodes = Vec::new();

        while self.pos < self.tokens.len() {
            match self.peekType() {
                TokenType::Include => nodes.push(self.parseInclude()),
                TokenType::Keyword => nodes.push(self.parseDeclarationOrFunction()),
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

    fn parseDeclarationOrFunction(&mut self) -> AST {
        let start_index = self.pos;
        let type_ = self.consumeValue();

        let name = self.peek().value.clone();
        self.advance();

        match self.peekType() {
            //  Its a function
            TokenType::OpenRound => {
                self.advance();

                let params = self.parseParams();

                self.expect(TokenType::CloseRound);

                let body = self.parseBlock();

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
        let var_type = self.consumeValue();
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

    fn parseParams(&mut self) -> Vec<AST> {
        let mut params = Vec::new();

        while *self.peekType() != TokenType::CloseRound {
            let var_type = self.peek().value.clone();
            self.advance();

            let name = self.peek().value.clone();
            self.advance();

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

        match self.peek().value.as_str() {
            _ => {
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
                Node: ASTNode::Literal("1".to_string()),
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
                TokenType::Period | TokenType::Arrow => {
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

                    expr = AST {
                        Node: ASTNode::Call {
                            function_name: match expr.Node {
                                ASTNode::Identifier(ref name) => name.clone(),
                                _ => panic!("Invalid function call target"),
                            },
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
                    Node: ASTNode::Literal(val),
                    Span: Span {
                        start_token: start,
                        end_token: self.pos,
                    },
                }
            }

            //  Char / string literals
            TokenType::CharLiteral | TokenType::StringLiteral => {
                let val = self.peek().value.clone();
                self.advance();
                AST {
                    Node: ASTNode::Literal(val),
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
                    //  Function call
                    self.advance();
                    let arguments = self.parseArguments();
                    self.expect(TokenType::CloseRound);
                    AST {
                        Node: ASTNode::Call {
                            function_name: name,
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
            panic!("Expected {:?}, got {:?} at {:?}", t, self.peekType(), self.pos);
            //  TODO: Improve error handling here
        }
        self.advance();
    }
}
