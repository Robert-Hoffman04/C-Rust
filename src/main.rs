use std::fs;
use std::path::Path;
use std::{
    default, env,
    fmt::Error,
    process::{Command, exit},
};

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
struct Arguments {
    input_file: String,
    output_dir: String,
    runOutput: bool,
}

enum ArgumentState {
    None,
    OutputFile,
}

fn handleArgs(mut args: Vec<String>) -> Result<Arguments, Error> {
    //  Consume first argument as it is just the executable
    args.remove(0);

    //  State
    //      0 (none)
    //      1 (output)
    let mut state: ArgumentState = ArgumentState::None;

    let mut arguments = Arguments {
        input_file: "".to_string(),
        output_dir: "".to_string(),
        runOutput:  false,
    };

    for arg in args {
        match arg.as_str() {
            "-o" => state = ArgumentState::OutputFile,
            "-r" => arguments.runOutput = true,
            //  Other state arguments would go here
            _ => {
                match state {
                    ArgumentState::None => {
                        arguments.input_file = arg.clone();
                    }
                    ArgumentState::OutputFile => {
                        arguments.output_dir = arg.clone();
                        state = ArgumentState::None;
                    }

                    _ => {
                        return Err(Error); /* TODO: Replace with a better error type */
                    }
                }
                // state = 0;
                // Technically might be better to put state reset here, but some args might need more than one value
            }
        }
    }

    //  If no input file passed by user, fail
    if arguments.input_file == "".to_string() {
        return Err(Error); /* TODO: Replace with a better error type */
    }

    //  If no output dir specified,
    //      use input file name, just without a file extension
    if arguments.output_dir == "".to_string() {
        //  Seperate off the ending ".***" extension
        //      If none present just use full noame
        let mut name = match arguments.input_file.rfind(".") {
            Some(index) => arguments.input_file.clone().split_off(index),
            None => arguments.input_file.clone(),
        };
        name.push_str(".rs"); //  Add rust out extension
        //  TODO: Decide if this should be a rs file of if it should be an executable path

        arguments.output_dir = name;
    }

    Ok(arguments)
}

fn process(args: Arguments) -> () {
    let mut processor = PreProcessor::new(args.input_file);
    processor.process_file();

    for line in &processor.lines {
        println!("{:?}", line);
    }
    println!("\n\n");

    let mut tokenizer = Tokenizer::new(processor.lines);
    tokenizer.tokenize();

    for (i, token) in (&tokenizer.tokens).iter().enumerate() {
        println!("{:?}: {:?}", i, token)
    }

    let mut parser = Parser::new(tokenizer.tokens);
    let ast = parser.parse();

    println!("\n\n\nParseTree\n{}", ast.Node);
    
    //  Get all of the files that the preprocessor found
    //      this also copies the local c-style files but eh who cares
    let mut build_files = processor.local_imports.clone();

    //  create the cargo file for the final script
    cargoCreate(&args.output_dir, build_files)
        .expect("Failed to build/run generated project");

    writer(ast, format!("{}/src/main.rs", args.output_dir));

    cargoBuildOrRun(args.output_dir, args.runOutput);
}

//      Frankly i dont understand why this result is diffrent
fn cargoCreate(dir_name: &String, files: Vec<String>) -> std::io::Result<()> {
    //  create new cargo project
    Command::new("cargo").arg("new").arg(&dir_name).status()?;

    //  path to src directory inside new project
    let src_path = Path::new(&dir_name).join("src");

    //  copy each file into src
    for file in files {
        let file_path = Path::new(&file);

        if let Some(filename) = file_path.file_name() {
            let dest = src_path.join(filename);
            fs::copy(file_path, dest)?;
        }
    }

    Ok(())
}

fn cargoBuildOrRun(dir_name: String, run: bool) -> std::io::Result<()>
{
    //  decide if building or running
    let mut cmd = Command::new("cargo");
    if run {
        cmd.arg("run");
    } else {
        cmd.arg("build");
    }

    //  actually build the project
    cmd.current_dir(&dir_name).status()?;

    Ok(())
}

fn main() {
    let options = handleArgs(
        env::args().collect()
        //  Hard to use actual arguments through vscode run
        /*
        vec![
            "C-Rust".to_string(),
            "test.crs".to_string(),
            "-o".to_string(),
            "output".to_string(),
        ],
        */
    );

    let arguments = match options {
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

#[cfg(test)]
mod tests {
    use super::parser::{ASTNode, Parser};
    use super::preProcessor::SourceLine;
    use super::tokenizer::{TokenType, Tokenizer};

    const SRC: &str = r#"void main() { Macro::print("Macro Test: {}", 10 + 5 * 2); }"#;

    //  helper to generate source line based on passed content
    //      normally done by "preprocessor" but that only works on files
    fn make_source(content: &str) -> Vec<SourceLine> {
        vec![SourceLine {
            file: "inline_test.crs".to_string(),
            line_number: 1,
            content: content.to_string(),
        }]
    }

    fn parse_root() -> Vec<ASTNode> {
        let mut t = Tokenizer::new(make_source(SRC));
        t.tokenize();
        let mut p = Parser::new(t.tokens);
        match p.parse().Node {
            ASTNode::Root(nodes) => nodes,
            _ => panic!("Expected Root node"),
        }
    }

    //  Tokenizer tests
    //      Does it produce anything?
    #[test]
    fn tokenizer_produces_tokens() {
        let mut t = Tokenizer::new(make_source(SRC));
        t.tokenize();
        assert!(!t.tokens.is_empty(), "Tokenizer produced no tokens");
    }

    //      Can it identify a keyword?
    #[test]
    fn tokenizer_identifies_void_keyword() {
        let mut t = Tokenizer::new(make_source(SRC));
        t.tokenize();
        assert!(
            t.tokens
                .iter()
                .any(|t| t.token_type == TokenType::Keyword && t.value == "void"),
            "Missing 'void' keyword token"
        );
    }

    //      Can it extract a string literal?
    #[test]
    fn tokenizer_identifies_string_literal() {
        let mut t = Tokenizer::new(make_source(SRC));
        t.tokenize();
        assert!(
            t.tokens.iter().any(
                |t| t.token_type == TokenType::StringLiteral && t.value.contains("Macro Test:")
            ),
            "Missing expected string literal token"
        );
    }

    //  Parser tests
    //      Can the parser correctly convert macros?
    #[test]
    fn parser_body_statement_is_macro_call() {
        let root = parse_root();
        match &root[0] {
            ASTNode::Function {
                body: Some(body), ..
            } => match &body[0].Node {
                ASTNode::Expression(inner) => match &inner.Node {
                    ASTNode::Call { function_name, .. } => {
                        assert_eq!(
                            function_name, "print!",
                            "Expected macro call rewritten to print!"
                        );
                    }
                    _ => panic!("Expected Call inside Expression"),
                },
                _ => panic!("Expected Expression statement"),
            },
            _ => panic!("Expected Function with body"),
        }
    }
}
