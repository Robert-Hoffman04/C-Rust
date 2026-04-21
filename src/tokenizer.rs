use core::panic;
use std::vec;

use super::preProcessor::SourceLine;

#[derive(Debug)]
enum TokenType
{
    Keyword,
    Identifer,
    Float,
    Integer,
    Asterisk,
    Period,
    Comma,
    OpenRound,
    OpenCurly,
    OpenAngle,
    OpenSquare,
    CloseRound,
    CloseCurly,
    CloseAngle,
    CloseSquare,
    Semicolon,
    Plus,
    Minus,
    Slash,
    Percent,
    Equals,
    PlusPlus,
    MinusMinus,
    PlusEquals,
    MinusEquals,
    AsteriskEquals,
    SlashEquals,
    PercentEquals,
    EqualsEquals,
    NotEquals,
    LessEquals,
    GreaterEquals,
    Ampersand,
    AmpersandEquals,
    AmpersandAmpersand,
    Pipe,
    PipeEquals,
    PipePipe,
    Caret,
    CaretEquals,
    Tilde,
    Exclamation,
    ShiftLeft,
    ShiftRight,
    Arrow,
    CharLiteral,
    StringLiteral,
    Include
}

const keywords : [&str ; 34] = [
    "auto",
    "break",
    "case",
    "char",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extern",
    "float",
    "for",
    "goto",
    "if",
    "inline",
    "int",
    "long",
    "register",
    "restrict",
    "return",
    "short",
    "signed",
    "sizeof",
    "static",
    "struct",
    "switch",
    "typedef",
    "union",
    "unsigned",
    "void",
    "volatile",
    "while"
];

#[derive(Debug)]
pub struct Token
{
    token_type : TokenType,
    value      : String,

    start_line : usize,
    start_char : usize,
    end_line   : usize,
    end_char   : usize,
}

pub struct Tokenizer
{
    //  Public
    pub lines  : Vec<SourceLine>,
    pub tokens : Vec<Token>,

    //  Private
    line_index : usize,
    char_index : usize
}

impl Tokenizer
{
    pub fn new(lines : Vec<SourceLine>) -> Tokenizer
    {
        Tokenizer
        {
            lines,
            
            tokens     : vec![],
            line_index : 0,
            char_index : 0,
        }
    }

    pub fn tokenize(&mut self)
    {
        let mut char_stack = String::new();
        let mut stack_line_start = 0usize;
        let mut stack_char_start = 0usize;

        let mut current_char = self.getCurrentChar();
        while current_char != '\0'
        {
            //  Since there are some left over include statments we need to handle them
            //      I've choosen to make it its own token for the AST converters to handle once it gets there
            if self.char_index == 0
            {
                let line = &self.lines[self.line_index].content;
                if line.trim().starts_with("#include")
                {
                    self.tokens.push(Token{
                        token_type: TokenType::Include,
                        value : line.clone().replace("#include ", ""),
                        start_line : self.line_index,
                        start_char : 0,
                        end_line : self.line_index,
                        end_char : line.len() - 1,
                    });

                    //  Jump to next line
                    self.line_index += 1;
                    current_char = self.getCurrentChar();
                    continue;
                }
            }

            //  is the current character part of a valid word or number?
            //      It can only contain a period if it is a number
            let is_word = current_char.is_ascii_alphanumeric() ||
                                current_char == '_' || 
                                ( current_char == '.' && !char_stack.is_empty() && char_stack.chars().all(|c| c.is_ascii_digit()) );

            //  if the current character is not part of a word or number, and the character stack has valid word or number in it
            if !is_word && !char_stack.is_empty()
            {
                self.resolveStack(
                    char_stack.clone(),
                    stack_line_start,
                    stack_char_start,
                    self.line_index,
                    self.char_index.saturating_sub(1)
                    );
                char_stack.clear();
            }

            match current_char
            {
                //  Single character tokens
                '~' => self.pushSingle(TokenType::Tilde),
                ',' => self.pushSingle(TokenType::Comma),
                '(' => self.pushSingle(TokenType::OpenRound),
                ')' => self.pushSingle(TokenType::CloseRound),
                '{' => self.pushSingle(TokenType::OpenCurly),
                '}' => self.pushSingle(TokenType::CloseCurly),
                '[' => self.pushSingle(TokenType::OpenSquare),
                ']' => self.pushSingle(TokenType::CloseSquare),
                ';' => self.pushSingle(TokenType::Semicolon),

                //  Double character tokens
                '+' => {
                    match self.peakNextChar()
                    {
                        '+' => { self.pushDouble(TokenType::PlusPlus);   self.char_index += 1; }
                        '=' => { self.pushDouble(TokenType::PlusEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Plus),
                    }
                }
                '-' => {
                    match self.peakNextChar()
                    {
                        '>' => { self.pushDouble(TokenType::Arrow);       self.char_index += 1; }
                        '-' => { self.pushDouble(TokenType::MinusMinus);  self.char_index += 1; }
                        '=' => { self.pushDouble(TokenType::MinusEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Minus),
                    }
                }
                '*' => {
                    match self.peakNextChar()
                    {
                        '=' => { self.pushDouble(TokenType::AsteriskEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Asterisk),
                    }
                }
                '/' => {
                    match self.peakNextChar()
                    {
                        '=' => { self.pushDouble(TokenType::SlashEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Slash),
                    }
                }
                '%' => {
                    match self.peakNextChar()
                    {
                        '=' => { self.pushDouble(TokenType::PercentEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Percent),
                    }
                }
                '=' => {
                    match self.peakNextChar()
                    {
                        '=' => { self.pushDouble(TokenType::EqualsEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Equals),
                    }
                }
                '!' => {
                    match self.peakNextChar()
                    {
                        '=' => { self.pushDouble(TokenType::NotEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Exclamation),
                    }
                }
                '<' => {
                    match self.peakNextChar()
                    {
                        '<' => { self.pushDouble(TokenType::ShiftLeft);  self.char_index += 1; }
                        '=' => { self.pushDouble(TokenType::LessEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::OpenAngle),
                    }
                }
                '>' => {
                    match self.peakNextChar()
                    {
                        '>' => { self.pushDouble(TokenType::ShiftRight);    self.char_index += 1; }
                        '=' => { self.pushDouble(TokenType::GreaterEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::CloseAngle),
                    }
                }
                '&' => {
                    match self.peakNextChar()
                    {
                        '&' => { self.pushDouble(TokenType::AmpersandAmpersand); self.char_index += 1; }
                        '=' => { self.pushDouble(TokenType::AmpersandEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Ampersand),
                    }
                }
                '|' => {
                    match self.peakNextChar()
                    {
                        '|' => { self.pushDouble(TokenType::PipePipe); self.char_index += 1; }
                        '=' => { self.pushDouble(TokenType::PipeEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Pipe),
                    }
                }
                '^' => {
                    match self.peakNextChar()
                    {
                        '=' => { self.pushDouble(TokenType::CaretEquals); self.char_index += 1; }
                        _ => self.pushSingle(TokenType::Caret),
                    }
                }
            
                //  Multicharacter tokens
                'a'..='z' | 'A'..='Z' | '_' => {
                    if char_stack.is_empty()
                    {
                        stack_line_start = self.line_index;
                        stack_char_start = self.char_index;
                    }
                    char_stack.push(current_char);
                }
                '0'..='9' => {
                    if char_stack.is_empty()
                    {
                        stack_line_start = self.line_index;
                        stack_char_start = self.char_index;
                    }
                    char_stack.push(current_char);
                }

                '.' => {
                    if !char_stack.is_empty() && char_stack.chars().all(|c| c.is_ascii_digit())
                    {
                        char_stack.push('.');
                    }
                    else
                    {
                        self.pushSingle(TokenType::Period);    
                    }
                },

                '\'' => {
                    let start_line = self.line_index;
                    let start_char = self.char_index;

                    self.char_index += 1;
                    let value = self.readCharLiteral().to_string();

                    self.tokens.push(
                        Token {
                            token_type: TokenType::CharLiteral,
                            value,
                            start_line,
                            start_char,
                            end_line: self.line_index,
                            end_char: self.char_index
                        }
                    );
                }

                '\"' => {
                    let start_line = self.line_index;
                    let start_char = self.char_index;
                    let value = self.readStringLiteral();
                    self.tokens.push(
                        Token {
                            token_type: TokenType::StringLiteral,
                            value,
                            start_line,
                            start_char,
                            end_line: self.line_index,
                            end_char: self.char_index
                        }
                    );
                }

                
                _ => { /* Nothing for now, maybe error? */}
            }

            //  Get ready for next loop iteration
            self.char_index += 1;
            current_char = self.getCurrentChar()
        }
    }

    fn pushSingle(&mut self, token_type : TokenType)
    {
        self.tokens.push(
            Token
            {
                token_type,
                value: String::new(),
                start_line: self.line_index,
                start_char: self.char_index,
                end_line: self.line_index,
                end_char: self.char_index
            }
        );
    }

    fn pushDouble(&mut self, token_type : TokenType)
    {
        self.tokens.push(
            Token
            {
                token_type,
                value: String::new(),
                start_line: self.line_index,
                start_char: self.char_index,
                end_line: self.line_index,
                end_char: self.char_index+1
            }
        );
    }

    fn resolveStack(&mut self, value : String, start_line : usize, start_char : usize, end_line : usize, end_char : usize)
    {
        let token_type = if value.contains('.')
        {
            TokenType::Float
        }
        else if value.chars().all(|c| c.is_ascii_digit())
        {
            TokenType::Integer
        }
        else if keywords.contains(&(value.as_str()))
        {
            TokenType::Keyword
        }
        else
        {
            TokenType::Identifer    
        };

        self.tokens.push(Token {
            token_type,
            value,
            start_line,
            start_char,
            end_line,
            end_char
        })
    }

    fn readStringLiteral(&mut self) -> String
    {
        let mut result = String::new();

        while !result.ends_with('"') || result.ends_with("\\\"")
        {
            self.char_index += 1;
            result.push(
                self.readCharLiteral()
            );
        }

        result.pop();   //  Remove final '"' before returning
        result
    }

    //  Reads the next char literal out of the token stream
    //      For example: char c = '\n'
    fn readCharLiteral(&mut self) -> char
    {        
        let c = self.getCurrentChar();

        match self.getCurrentChar()
        {
            '\\' => {
                match self.peakNextChar()
                {
                    '"' => {self.char_index += 1; '\"'}
                    'n' => {self.char_index += 1; '\n'}
                    't' => {self.char_index += 1; '\t'}
                    'r' => {self.char_index += 1; '\r'}
                    '0' => {self.char_index += 1; '\0'}
                    '\\' => {self.char_index += 1; '\\'}
                    '\'' => {self.char_index += 1; '\''}
                    
                    //  Otherwise it was a false positive, so relay the original character
                    _ => c
                }
            }
            _ => c
        }

    }

    fn getCurrentChar(&mut self) -> char
    {
        //  if all lines have been worked through, just return EOF
        if self.line_index >= self.lines.len()
        {
            return '\0'
        }
        let line = &self.lines[self.line_index].content;

        //  Get the proper character from the line
        //      if the line has been overflowed, move to the next one
        match line.chars().nth(self.char_index)
        {
            Some(v) => v,
            None => {
                self.line_index += 1;
                self.char_index  = 0;
                self.getCurrentChar()
            }
        }
        //  Because of how this function is set up, the program only needs to increment
        //  the char_index, line index manipulation is handled completly automatically
    }

    fn peakNextChar(&mut self) -> char
    {
        let next_char_index = self.char_index + 1;

        if self.line_index >= self.lines.len()
        {
            return '\0';
        }

        let line = &self.lines[self.line_index].content;

        match line.chars().nth(next_char_index)
        {
            Some(v) => v,
            None => '\0'    //  Dont peak across lines
        }
    }
}