use std::{fs, process::Output};
use super::helper::strExtensions;

//  This stuct is responsible for keeping track of the source location for error traceback
struct SourceLine
{
    file: String,
    line_number: usize,
    content: String,
}

pub struct PreProcessor
{
    //  public
    pub original_input  : String,
    pub local_imports   : Vec<String>,
    pub external_imports    : Vec<String>,

    //  private
    process_index : usize,
    lines         : Vec<SourceLine>
}

impl PreProcessor
{
    const RESERVED: [&'static str ; 10] = [
        "#define",
        "#undef",
        "#include",
        "#ifdef",
        "#ifndef",
        "#if",
        "#elif",
        "#else",
        "#error",
        "#pragma"
    ];

    fn new(original_input : String) -> PreProcessor
    {
        PreProcessor {
            original_input,
            
            local_imports     : vec![],
            external_imports  : vec![],
            process_index     : 0,
            lines             : vec![]
        }
    }   
    
    fn include(&mut self, mut file : String) -> ()
    {
        //  Take off the first and last characters and save them to make sure the file is properly closed
        let fileType = file.remove(0);
        let fileCheck = file.pop();

        if file.starts_with("<")            //  Link to external library
        {
            self.external_imports.push(file);
        }
        else if file.starts_with("\"")      //  Link to local file
        {
            self.local_imports.push(file.clone());

            //  If it is a local file there are two possiblilities
            //      Its another C style file, in which case it can be directly included into lines
            //      Its a local Rust file, this needs no additional processing but it does need saved for AST conversion
            



        }
        else
        {
            // TODO: Exception on malformed import
        }
    }

    fn process_file(self)
    {
        //  Read in the actual text file
        let input = match fs::read_to_string(self.original_input.clone())
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to read {}: {}", self.original_input.as_str(), e);
                std::process::exit(1);
            }
        };

        //  Convert the singular file string into individual lines
        //      Use the SourceLine struct to keep track of the position in the file
        self.lines.extend(
        input.lines().enumerate().map(|(i, line)| SourceLine {
                file: self.original_input.clone(),
                line_number: i + 1,
                content: line.to_string(),
            })
        );

        //  Iterate through all of the avalible lines
        //      This automatically accounts for inport statments
        //      as when one is reached, the self.lines vector is
        //      spliced to contain the new file
        while self.process_index < self.lines.len()
        {
            let line = self.lines[self.process_index].content;
            if !line.starts_with('#')
            {
                //  If this is not a directive line, just skip over it
                //  TODO "define" replacement logic
                continue;
            }
            
            //  Actually simpler if the directive is stored as a &str at this point
            let (directive, value) = match line.split_once(' ')
            {
                None => (line.as_str(), "".to_string()),
                Some((v1, v2)) => (v1, v2.to_string())
            };
            
            
            if !PreProcessor::RESERVED.contains(&directive)
            {
                //  TODO: error checking for invalid 
                continue;
            }

            match directive
            {
                "#define"  => {}
                "#undef"   => {}
                "#include" => { self.include(value) }
                _ => {} //  TODO
            }

        }
        ()
    }
}