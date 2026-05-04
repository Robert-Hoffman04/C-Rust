use super::*;

//  Super basic testing just to make sure it can interphase with rust syntax
//      Also proves the display function works and the implements works
pub fn local_rust_function_test(message : ExampleStruct) -> ()
{
    println!("This is a print test from inside rust: {}", message);
}   