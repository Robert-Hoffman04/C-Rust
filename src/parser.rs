use super::tokenizer::{Token, TokenType};


//  Simplify keeping track of character locations from original file,
//      probably should have used this in tokenizer but im not going back to fix it
#[derive(Debug,Clone)]
pub struct Span
{
    start_line : usize,
    start_char : usize,
    end_line   : usize,
    end_char   : usize,
}

impl From<&Token> for Span
{
    fn from(t : &Token) -> Self
    {
        Span
        {
            start_line : t.start_line,
            start_char : t.start_char,
            end_line : t.end_line,
            end_char : t.end_char
        }
    }
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
        op   : String,      //  1 + 2 * 3 --> (1 + (2 * 3))
        left : String,      //  This preserves the proper order of operations
        right: String
    },

    Unary
    {
        op  : String,
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

    pub fn parse(&self)
    {
        let mut functions = Vec::new();
        let mut globals = Vec::new();

        while self.pos < self.tokens.len()
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
}