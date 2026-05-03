use std::{default, env, fmt::Error, process::exit};

mod preProcessor;
use preProcessor::PreProcessor;

mod tokenizer;
use tokenizer::Tokenizer;

mod parser;
use parser::Parser;

mod writer;
use writer::writer;

/*
 *  current valid args are "-o <output_filename>"" and just "<input_filename>""
 */

#[derive(Debug)]
struct Arguments
{
    input_file  : String,
    output_file : String,
}

enum ArgumentState
{
    None,
    OutputFile
}

fn handleArgs(mut args : Vec<String>) -> Result<Arguments, Error>
{
    //  Consume first argument as it is just the executable 
    args.remove(0);

    //  State
    //      0 (none)
    //      1 (output)
    let mut state : ArgumentState = ArgumentState::None;

    let mut arguments = Arguments{
        input_file : "".to_string(),
        output_file: "".to_string()
    };

    for arg in args
    {
        
        match arg.as_str()
        {
            "-o" => { state = ArgumentState::OutputFile },
            //  Other state arguments would go here
            _  => {
                match state
                {
                    ArgumentState::None       => { arguments.input_file  = arg.clone();                              }
                    ArgumentState::OutputFile => { arguments.output_file = arg.clone(); state = ArgumentState::None; }

                    _ => { return Err(Error); /* TODO: Replace with a better error type */ }
                }
                // state = 0;
                // Technically might be better to put state reset here, but some args might need more than one value
            }
        }
    }

    //  If no input file passed by user, fail
    if arguments.input_file == "".to_string()
    {
        return Err(Error); /* TODO: Replace with a better error type */
    }
    
    //  If no output file specified,
    //      use input file name, just with a diffrent file extension
    if arguments.output_file == "".to_string()
    {
        //  Seperate off the ending ".***" extension
        //      If none present just use full noame
        let mut name = match arguments.input_file.rfind(".")
        {
            Some(index) => {arguments.input_file.clone().split_off(index)}
            None => {arguments.input_file.clone()}
        };
        name.push_str(".rs");   //  Add rust out extension
        //  TODO: Decide if this should be a rs file of if it should be an executable path

        arguments.output_file = name;
    }


    Ok(arguments)
}

fn process(args : Arguments) -> ()
{
    let mut processor = PreProcessor::new(args.input_file);
    processor.process_file();

    for line in &processor.lines
    {
        println!("{:?}", line);
    }
    println!("\n\n");

    let mut tokenizer = Tokenizer::new(
        processor.lines
    );
    tokenizer.tokenize();

    for (i, token) in (&tokenizer.tokens).iter().enumerate()
    {
        println!("{:?}: {:?}", i, token)
    }

    let mut parser = Parser::new(
        tokenizer.tokens
    );
    let AST = parser.parse();

    println!("\n\n\nParseTree\n{}", AST.Node);
    
    writer(AST, args.output_file);
}

fn main()
{
    let options = handleArgs(
        //env::args().collect()
        //  Hard to use actual arguments through vscode run
        vec!["C-Rust".to_string(), "test.crs".to_string(), "-o".to_string(), "output.rs".to_string()]
    );

    let arguments = match options 
    {
        Ok(args) => args,
        Err(err) => {
            println!("Invalid arguments");
            exit(1);
            //  TODO Imporve error message 
        }
    };

    println!("{:?}", arguments);

    process(arguments);
}

/*
#[cfg(test)]
mod tests {
    use super::preProcessor::SourceLine;
    use super::tokenizer::{Tokenizer, TokenType};
    use super::parser::{Parser, ASTNode};

    //  One big test to go through each step and confirm its working
    //      Basically just tokenized and parsed a small function by hand
    //      then make sure output matches
    #[test]
    fn parses_requested_inline_program() {
        let lines = vec![
            SourceLine { file: "inline_test.crs".to_string(), line_number: 1, content: "int A = 1;".to_string() },
            SourceLine { file: "inline_test.crs".to_string(), line_number: 2, content: "int main(int argc, char** argv) { printf(\"Say Hi!\\n\"); if (true) return 1; else return 0;}".to_string() },
        ];

        let mut tokenizer = Tokenizer::new(lines);
        tokenizer.tokenize();

        assert!(tokenizer.tokens.len() > 0, "Tokenizer produced no tokens");
        assert!(tokenizer.tokens.iter().any(|t| t.token_type == TokenType::Keyword && t.value == "if"), "Missing if keyword token");
        assert!(tokenizer.tokens.iter().any(|t| t.token_type == TokenType::Identifier && t.value == "printf"), "Missing printf identifier token");
        assert!(tokenizer.tokens.iter().any(|t| t.token_type == TokenType::StringLiteral && t.value.contains("Say Hi!")), "Missing expected string literal token");

        let mut parser = Parser::new(tokenizer.tokens);
        let ast = parser.parse();

        let root = match ast.Node {
            ASTNode::Root(nodes) => nodes,
            _ => panic!("Expected root AST node"),
        };

        assert!(root.len() == 2, "Expected top-level declaration and function");

        match &root[0] {
            ASTNode::Declaration { var_type, name, .. } => {
                assert!(var_type == "int", "Expected first declaration type to be int");
                assert!(name == "A", "Expected first declaration name to be A");
            }
            _ => panic!("Expected first root node to be declaration"),
        }

        match &root[1] {
            ASTNode::Function { body, .. } => {
                match body
                {
                    Some(body) => {
                        assert!(body.len() == 2, "Expected function body to have printf and if");
                        match &body[1].Node {
                            ASTNode::If { then_branch, else_branch, .. } => {
                                match &then_branch.Node {
                                    ASTNode::Return(Some(_)) => {}
                                    _ => panic!("Expected then branch to be return 1"),
                                }
                                match &else_branch.Node {
                                    ASTNode::Return(Some(_)) => {}
                                    _ => panic!("Expected else branch to be return 0"),
                                }
                            }
                            _ => panic!("Expected second statement to be if"),
                        }
                    }
                    None => ()
                }
            }
            _ => panic!("Expected second root node to be function"),
        }
    }
} */