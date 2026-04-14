use std::{default, env, fmt::Error, process::exit};

mod helper;
use helper::strExtensions;

mod preProcessor;
use preProcessor::PreProcessor;

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

    println!("{:?}", processor.lines);

}

fn main()
{
    let options = handleArgs(
        //env::args().collect()
        //  Hard to use actual arguments through vscode run
        vec!["C-Rust".to_string(), "test.crs".to_string(), "-o".to_string(), "output.rs".to_string()]
    );

    let mut test = "";
    test.remove_first_and_last();

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
