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
    "while",
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
                '*' => self.pushSingle(TokenType::Asterisk),
                ',' => self.pushSingle(TokenType::Comma),
                '(' => self.pushSingle(TokenType::OpenRound),
                ')' => self.pushSingle(TokenType::CloseRound),
                '{' => self.pushSingle(TokenType::OpenCurly),
                '}' => self.pushSingle(TokenType::CloseCurly),
                '<' => self.pushSingle(TokenType::OpenAngle),
                '>' => self.pushSingle(TokenType::CloseAngle),
                '[' => self.pushSingle(TokenType::OpenSquare),
                ']' => self.pushSingle(TokenType::CloseSquare),
                ';' => self.pushSingle(TokenType::Semicolon),

                //  Multicharacter tokens
                'a'..='z' | 'A'..='Z' | '_' => {
                    if char_stack.is_empty()
                    {
                        stack_line_start = self.line_index;
                        stack_char_start = self.char_index;
                    }
                    char_stack.push(current_char);
                }
                '1'..='9' => {
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

    fn getCurrentChar(&mut self) -> char
    {
        //  if all lines have been worked through, just return EOF
        if self.line_index >= self.lines.len()
        {
            return '\0'
        }
        let line = self.lines[self.line_index].content.clone();

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

        //  Because of how this function is set up, the program only needs to increment or decrement
        //  the char_index, line index manipulation is handled completly automatically
    }
}