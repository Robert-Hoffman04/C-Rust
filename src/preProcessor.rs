use std::{collections::HashMap, fs, process::Output};
use super::helper::strExtensions;

//  This stuct is responsible for keeping track of the source location for error traceback
#[derive(Debug)]
pub struct SourceLine
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

    //  private, public for debugging only
    pub process_index : usize,
    pub lines         : Vec<SourceLine>,
    pub define_map    : HashMap<String, String>
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

    pub fn new(original_input : String) -> PreProcessor
    {
        PreProcessor {
            original_input,

            //  Initialize to empty/0
            local_imports     : vec![],
            external_imports  : vec![],
            process_index     : 0,
            lines             : vec![],
            define_map        : HashMap::new()
        }
    }   

    fn include(&mut self, mut file : String) -> ()
    {
        println!("including file: {}", file);
        //  Take off the first and last characters and save them to make sure the file is properly closed
        let fileType = file.remove(0);

        match fileType
        {
            '<' => { self.external_imports.push(file); }
            '"' => {
                match file.rfind('"')
                {
                    None => { return; /* TODO EXCEPTION */ }
                    Some(i) => { file.truncate(i); }
                };
                self.local_imports.push(file.clone());

                //  If it is a local file there are two possiblilities
                //      Its another C style file, in which case it can be directly included into lines
                //      Its a local Rust file, this needs no additional processing but it does need saved for AST conversion
                
                let extension = match file.rfind(".")
                {
                    Some(index) => {file.clone().split_off(index)}
                    None => { ".crs".to_string() } //   Default to C-style file if unknown
                };
                println!("Adding: {} {}", file, extension);
                match extension.as_str()
                {
                    ".rs"  => { /* Do Nothing, handled during lexing */ }
                    
                    ".crs" | ".hrs" => {
                        // Replace the "#include ..." line with the actual lines from said file, 
                        println!("Adding: {}", file);
                        self.lines.splice(
                            self.process_index..self.process_index+1, 
                            fileToVec(file)
                        );
                        self.process_index -= 1;
                    }
                    _ => {
                        // TODO: Exception or default to one of the options
                    }
                }
            }
            _ => { /* TODO: Exception on malformed import */ }
        }
    }

    pub fn process_file(&mut self)
    {
        self.lines.extend(
            fileToVec(self.original_input.clone())
        );

        //  Iterate through all of the avalible lines
        //      This automatically accounts for inport statments
        //      as when one is reached, the self.lines vector is
        //      spliced to contain the new file
        while self.process_index < self.lines.len()
        {
            let mut line = self.lines[self.process_index].content.clone();
            
            //  Remove single line comments
            line = match line.split_once("//")
            {
                None => line,
                Some((v1, v2)) => v1.to_string()
            };

            if !line.starts_with('#')
            {
                //  If this is not a directive line, just skip over it
                //  TODO "define" replacement logic
                self.process_index += 1;
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
                //  TODO: error checking for invalid instead of just continuing
                self.process_index += 1;
                continue;
            }

            match directive
            {
                "#define"  => { 
                    let (statment, definition) = match value.split_once(' ')
                    {
                        None => (value, "".to_string()),
                        Some((v1, v2)) => (v1.to_string(), v2.to_string())
                    };
                    self.define_map.insert(statment, definition);
                }
                "#undef"   => {
                    self.define_map.remove(&value);
                }
                "#include" => { 
                    self.include(value);
                }
                _ => {} //  TODO
            }
            self.process_index += 1;

        }
        ()
    }
}



fn fileToVec(file : String) -> Vec<SourceLine>
{
    //  Read in the actual text file
    let input = match fs::read_to_string(file.clone())
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to read {}: {}", file.as_str(), e);
            std::process::exit(1);
        }
    };

    //  Convert the singular file string into individual lines
    //      Use the SourceLine struct to keep track of the position in the file
    Vec::from_iter(
        input.lines().enumerate().map(|(i, line)| SourceLine {
            file: file.clone(),
            line_number: i + 1,
            content: line.to_string(),
        })
    )
}