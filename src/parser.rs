use super::tokenizer::{Token, TokenType};


//  Simplify keeping track of token locations from original file,
#[derive(Debug,Clone)]
pub struct Span
{
    start_token : usize,
    end_token   : usize,
}

#[derive(Debug)]
pub struct AST
{
    pub Node: ASTNode,
    pub Span: Span,
}

#[derive(Debug)]
//  Tokens were a stuct because they largly had the same structure,
//  This is an enum because each individual type has very specific requirments 
pub enum ASTNode
{
    Root(Vec<ASTNode>), //    The Head of all AST's

    Include(String),    //    Include propigation for local and external rust files
    
    Function
    {
        return_type: String,
        name       : String,
        params     : Vec<AST>,
        body       : Vec<AST>
    },

    Declartion
    {
        var_type: String,
        name    : String,
        init    : Option<Box<AST>>  //  Has to be boxed to prevent recursive struct with infinate size. Similar uses going forward
    },

    Compound(Vec<AST>),     //  List of basic statments

    If
    {
        condition  : Box<AST>,
        then_branch: Box<AST>,
        else_branch: Box<AST>,  //  If else also contained here, just nested statments all the way down
    },

    While   //  All loops are converted down into this type
    {
        condition: Box<AST>,
        body     : Box<AST>,
    },

    Return(Option<Box<AST>>),

    Expression(Box<AST>),   //  Wrapper for anything that needs to be evaluated

    Binary                  //  All multi step operations are compiled down into a series of binary operations
    {                       //  Its best to think of it as sets of parenthesis being added to an expression
        op   : TokenType,   //  1 + 2 * 3 --> (1 + (2 * 3))
        left : Box<AST>,    //  This preserves the proper order of operations
        right: Box<AST>
    },

    Unary
    {
        op  : TokenType,
        expr: Box<AST>
    },

    Literal(String),        //  Any raw value. "A String", 'c' (char), 123 (Int), 3.14 (float)

    Identifer(String),      //  variable names

    Call
    {
        function_name : String,
        arguments : Vec<AST>
    }
}

pub struct Parser
{
    tokens: Vec<Token>,
    pos: usize,
    
    includes: Vec<String>,
    external: Vec<String>,
}

impl Parser
{
    pub fn new(tokens : Vec<Token>) -> Self
    {
        Parser {tokens, pos : 0, includes : vec![], external : vec![] }
    }

    pub fn parse(&mut self) -> AST
    {
        let mut nodes = Vec::new();

        while self.pos < self.tokens.len()
        {
            match self.peekType()
            {
                TokenType::Include => nodes.push(self.parseInclude()),
                TokenType::Keyword => nodes.push(self.parseDeclarationOrFunction()),
                _ => self.advance(),
            }
        }

        AST
        {
            Node: ASTNode::Root(nodes.into_iter().map(|n| n.Node).collect()),
            Span: Span { start_token: 0, end_token: self.tokens.len() }
        }
    }

    fn parseInclude(&mut self) -> AST
    {
        AST
        {
            Node: ASTNode::Include(self.peek().value.clone()),
            Span: Span { start_token: self.pos, end_token: self.pos}
        }
    }

    fn parseDeclarationOrFunction(&mut self) -> AST
    {
        let start_index = self.pos;
        let type_ = self.peek().clone();
        self.advance();

        let name = self.peek().value.clone();
        self.advance();

        match self.peekType()
        {
            //  Its a function
            TokenType::OpenRound => 
            {
                self.advance();

                let params = self.parseParams();
                
                self.expect(TokenType::CloseRound);

                let body = self.parseBlock();

                AST
                {
                    Node: ASTNode::Function { return_type: type_.value, name, params, body },
                    Span: Span { start_token: start_index, end_token: self.pos }
                }
            },
            //  Not a function
            _ =>
            {
                let mut value = None;
                if *self.peekType() == TokenType::Equals
                {
                    self.advance();
                    value = Some(Box::new(self.parseExpression()));
                }

                self.expect(TokenType::Semicolon);

                AST
                {
                    Node: ASTNode::Declartion { var_type: type_.value, name, init: value },
                    Span: Span { start_token: start_index, end_token: self.pos }
                }
            }
        }
    }

    fn parseParams(&mut self) -> Vec<AST>
    {
        let mut params = Vec::new();

        while *self.peekType() != TokenType::CloseRound
        {
            let var_type = self.peek().value.clone();
            self.advance();

            let name = self.peek().value.clone();
            self.advance();

            params.push(AST {
                Node: ASTNode::Declartion {
                    var_type,
                    name,
                    init: None
                },
                Span: Span {
                    start_token: self.pos - 2,
                    end_token: self.pos
                }
            });

            if *self.peekType() == TokenType::Comma
            {
                self.advance();
            }
        }

        params
    }

    fn parseBlock(&mut self) -> Vec<AST>
    {
        let mut body = Vec::new();
        self.expect(TokenType::OpenCurly);

        while *self.peekType() != TokenType::CloseCurly
        {
            body.push(self.parseStatement());
        }

        self.expect(TokenType::CloseCurly);
        body
    }

    fn parseStatement(&mut self) -> AST
    {
        let start_token = self.pos;
        match self.peek().value.as_str()
        {
            "return" =>
            {
                self.advance();

                let expr = if *self.peekType() != TokenType::Semicolon
                {
                    Some(Box::new(self.parseExpression()))
                } else {
                    None
                };

                self.expect(TokenType::Semicolon);

                AST
                {
                    Node: ASTNode::Return(expr),
                    Span: Span {
                        start_token,
                        end_token: self.pos
                    }
                }
            }
            _ =>
            {
                let expr = self.parseExpression();
                self.expect(TokenType::Semicolon);

                AST
                {
                    Node: ASTNode::Expression(Box::new(expr)),
                    Span: Span {
                        start_token,
                        end_token: self.pos
                    }
                }
            }
        }
    }

    fn parseExpression(&mut self) -> AST
    {
        self.parseAssignment()
    }

    fn parseAssignment(&mut self) -> AST
    {
        let start_token = self.pos;
        let left = self.parseLogicalOr();

        let op = match self.peekType()
        {
              TokenType::Equals
            | TokenType::PlusEquals
            | TokenType::MinusEquals
            | TokenType::AsteriskEquals
            | TokenType::SlashEquals
            | TokenType::PercentEquals
            | TokenType::AmpersandEquals
            | TokenType::PipeEquals
            | TokenType::CaretEquals => self.peek().value.clone(),
            _ => return left
        };

        self.advance();
        let right = self.parseAssignment();

        AST
        {
            Node: ASTNode::Binary { op, left: Box::new(left), right: Box::new(right) },
            Span: Span { start_token, end_token: self.pos }
        }

    }

    fn parseLogicalOr(&mut self) -> AST
    {
        self.parseBinary(
            //  Refrence to a list of valid tokens, some allow multiple types
            &[TokenType::PipePipe],
            Self::parseLogicalAnd
            //  Passing a function to call to parseBinary
            //      Explained more in that function            
        )
    }

    fn parseLogicalAnd(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::AmpersandAmpersand],
            Self::parseBitwiseOr
        )
    }

    fn parseBitwiseOr(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::Pipe],
            Self::parseBitwiseXor
        )
    }

    fn parseBitwiseXor(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::Caret],
            Self::parseBitwiseAnd
        )
    }

    fn parseBitwiseAnd(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::Ampersand],
            Self::parseEquality         
        )
    }

    fn parseEquality(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::EqualsEquals, TokenType::NotEquals],
            Self::parseRelational
        )
    }

    fn parseRelational(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::OpenAngle, TokenType::CloseAngle,
             TokenType::LessEquals, TokenType::GreaterEquals],
            Self::parseShift
        )
    }

    fn parseShift(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::ShiftLeft, TokenType::ShiftRight],
            Self::parseAdditive
        )
    }

    fn parseAdditive(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::Plus, TokenType::Minus],
            Self::parseMultiplicative
        )
    }

    fn parseMultiplicative(&mut self) -> AST
    {
        self.parseBinary(
            &[TokenType::Asterisk, TokenType::Slash, TokenType::Percent],
            Self::parseUnary
        )
    }

    //  This is the part that actually handles the ordering of operations
    //  Functions call this function and pass in the next option.
    //      If the current Token is not valid for a given operation,
    //      it moves to the next
    fn parseBinary(&mut self, ops: &[TokenType], next: fn(&mut Self) -> AST) -> AST
    {
        let start_token = self.pos;
        let mut left = next(self);

        while ops.contains(self.peekType())
        {
            let op = self.peekType();
            self.advance();

            let right = next(self);
            left = AST
            {
                Node: ASTNode::Binary { op: op.clone(), left: Box::new(left), right: Box::new(right)},
                Span: Span { start_token, end_token: self.pos }
            }
        }

        left
    }

    fn parseUnary(&mut self) -> AST
    {
        let start_token = self.pos;

        let valid_unary = &[
            TokenType::Exclamation,
            TokenType::Tilde,
            TokenType::Minus,
            TokenType::PlusPlus,
            TokenType::MinusMinus,
            TokenType::Ampersand,
            TokenType::Asterisk
        ];

        if valid_unary.contains(self.peekType())
        {
            let op = self.peekType().clone();
            self.advance();
            let expr = self.parseUnary();
            return AST
            {
                Node: ASTNode::Unary { op, expr: Box::new(expr) },
                Span: Span { start_token, end_token: self.pos}
            }
        }

        self.parsePostfix()
    }

    fn parsePostfix(&mut self) -> AST
    {
        let start_token = self.pos;
        let mut expr = self.parsePrimary();

        //  All other operators are right associative ie 1 * (2 * 3)
        //  But these ones are the opposite ie ( function.call() ).call2()
        //      So loop over recursion since decrementing the token counter is frowned apon
        loop
        {
            match self.peekType()
            {
                //  funcion calls
                TokenType::Period | TokenType::Arrow =>
                {
                    let op = self.peekType().clone();
                    self.advance();
                    let field = self.peek().value.clone();
                    self.advance();
                    expr = AST
                    {
                        Node: ASTNode::Binary { op, left: Box::new(expr), right: Box::new(AST {
                            Node: ASTNode::Identifer(field),
                            Span: Span { start_token: self.pos - 1, end_token: self.pos }
                        })},
                        Span: Span { start_token, end_token: self.pos}
                    };
                }
                //  Array indexing
                TokenType::OpenSquare =>
                {
                    self.advance();
                    let index = self.parseExpression();
                    self.expect(TokenType::CloseSquare);
                    expr = AST
                    {
                        //  Using opensquare as the operation since there isnt a specific one for this
                        Node: ASTNode::Binary { op: TokenType::OpenSquare, left: Box::new(expr), right: Box::new(index) },
                        Span: Span { start_token, end_token: self.pos }
                    }
                }
                // variable++ / variable--
                TokenType::PlusPlus | TokenType::MinusMinus =>
                {
                    expr = AST
                    {
                        Node: ASTNode::Unary { op: self.peekType().clone(), expr: Box::new(expr) },
                        Span: Span { start_token, end_token: self.pos + 1}
                    };
                    //  post increment to keep token peeking to operation
                    self.advance();
                }
                //  If we see anything else, its back to right associative, so exit loop
                _ => break,
            }    
        }

        expr
    }

    fn parsePrimary(&mut self) -> AST
    {
        let start_token = self.pos;

        match self.peekType()
        {
            
        }
    }

    fn peek(&self) -> &Token
    {
        &(self.tokens[self.pos])
    }
    fn peekType(&self) -> &TokenType
    {
        &self.tokens[self.pos].token_type
    }   

    fn advance(&mut self) { self.pos += 1; }

    fn expect(&mut self, t: TokenType)
    {
        if *self.peekType() != t
        {
            panic!("Expected {:?}, got {:?}", t, self.peekType());
            //  TODO: Improve error handling here
        }
        self.advance();
    }
}