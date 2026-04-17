use std::{collections::HashMap, fs, ops::Index, process::Output, vec};
use regex::Regex;

//  This stuct is responsible for keeping track of the source location for error traceback
#[derive(Debug)]
pub struct SourceLine
{
    pub file: String,
    pub line_number: usize,
    pub content: String,
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
    const RESERVED: [&'static str ; 11] = [
        "#define",  //Done
        "#undef",   //Done
        "#include", //Done-ish
        "#ifdef",   //Done
        "#ifndef",  //Done
        "#endif",   //Done
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
        println!("Including: {}", file);
        //  Take off the first and last characters and save them to make sure the file is properly closed
        let fileType = file.remove(0);

        match fileType
        {
            '<' => { self.external_imports.push(file); self.process_index += 1; }
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
                match extension.as_str()
                {
                    ".rs"  => { self.process_index += 1; /* Do Nothing, handled during lexing */ }
                    
                    ".crs" | ".hrs" => {
                        // Replace the "#include ..." line with the actual lines from said file, 
                        self.lines.splice(
                            self.process_index..self.process_index+1, 
                            fileToVec(file)
                        );
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
            let line = self.lines[self.process_index].content.clone();

            if !line.starts_with('#')
            {
                for key in self.define_map.keys()
                {
                    let pattern = format!(r"\b{}\b", regex::escape(key));
                    let re = Regex::new(&pattern).unwrap(); //  TODO handle regex errors better. just continue?

                    if re.is_match(&line)
                    {
                        self.lines[self.process_index].content = line.replace(
                            key,
                            self.define_map.get(key).expect("Impossible")   //  Should never fail due to interation
                        );
                        //  Continue without incrementing to check for nested defines
                        //      Theoretically could cause problems if define is recurssive (TODO?)
                        continue;
                    }
                }

                //  Otherwise skip other processing
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
                "#ifdef" => {
                    if !self.define_map.contains_key(&value)
                    {
                        println!("Removing: {:?}", self.lines[self.process_index]);
                        self.lines.remove(self.process_index);

                        //  If not defined, skip over until #endif
                        while self.lines[self.process_index].content != "#endif"
                        {
                            println!("Removing: {:?}", self.lines[self.process_index]);
                            self.lines.remove(self.process_index);
                        }
                    }
                }
                "#ifndef" => {
                    if self.define_map.contains_key(&value)
                    {
                        println!("Removing: {:?}", self.lines[self.process_index]);
                        self.lines.remove(self.process_index);

                        //  If not defined, skip over until #endif
                        while self.lines[self.process_index].content != "#endif"
                        {
                            println!("Removing: {:?}", self.lines[self.process_index]);
                            self.lines.remove(self.process_index);
                        }
                    }
                }
                "#endif" => { }
                "#include" => { 
                    self.include(value);
                    continue; // Removing would get rid of includes that should stay
                }
                _ => {} //  TODO
            }

            //  If it makes it all the way here, then the statment has already been processed and can be thrown away
            println!("Removing: {:?}", self.lines[self.process_index]);
            self.lines.remove(self.process_index);

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
    let file_lines = input.lines();

    let mut in_multi_line_comment = false;
    let mut output : Vec<SourceLine> = vec![];

    // Comment removal must be done before any processing so it is done immediatly on import
    for (i, line) in file_lines.enumerate()
    {
        //  Remove single line comments
        let mut line = match line.split_once("//")
        {
            None => line.to_string(),
            Some((v1, v2)) => v1.to_string()
        };

        //  Handle multi line comment start
        if line.contains("/*")
        {
            let start = line.find("/*").expect("Impossible"); //  guarenteed due to the if statments so expect is fine

            //  If the multi line comment starts and ends in the same line, it can bee handled here
            if line.contains("*/")
            {
                //  Replace all of the characters in the comment with ' ' to keep position based errors accurate
                let end = line.find("*/").expect("Impossible");
                let range = start..end+2;
                let spaces = " ".repeat(range.len());
                line.replace_range(range, &spaces);
            }
            else
            {
                //  Otherwise its a true multiline comment so just set the flag to true
                in_multi_line_comment = true;
                let end = line.len();
                let range = start..end;
                let spaces = " ".repeat(range.len());
                line.replace_range(range, &spaces);
            }
        }

        //  Handle multiline comment end
        if in_multi_line_comment
        {
            if !line.contains("*/")
            {
                //  If we are already in a multi line comment, and the current line does not end it,
                //      That means the line is useless in terms of executable code.
                continue;
            }
            else
            {
                //  And because of the last check we know this line must end the comment if it gets to here
                let end = line.find("*/").expect("Impossible");
                let range = 0..end+2;
                let spaces = " ".repeat(range.len());
                line.replace_range(range, &spaces);

                in_multi_line_comment = false;
            }
        }

        //  If the line is entirely empty, no need to keep it
        if line.trim().is_empty()
        {
            continue;
        }

        output.push(
            SourceLine
            {
                file: file.clone(),
                line_number: i + 1,
                content: line.to_string(),
            }
        );
    };

    output
}