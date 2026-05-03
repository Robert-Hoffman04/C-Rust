use std::vec;

use std::fmt;

static  imported_var: i32 = 10;

mod rustImportTest;

static  check4: &str = "But leave this intacked";

static  check5: &str = "this is a define stament test";

#[derive(Debug)]
enum TestEnum
{
    Option1,
    Option2,
}

#[derive(Debug)]
enum TestTypeEnum
{
    Choice1,
    Choice2,
}

#[derive(Debug)]
struct ExampleStruct {
    pub A: i32,
    pub B: i32,
    C: i32,
}

impl ExampleStruct {
    pub fn new(A: i32) -> ExampleStruct
    {
        let mut A_19183998_2a6f_427d_aa56_74d4eab17865 = A;
        ExampleStruct
        {
            A: A_19183998_2a6f_427d_aa56_74d4eab17865,
            B: Default::default(),
            C: Default::default(),
        }
    }
    pub fn math_and_stuff(&mut self) -> i32
    {
        return self . A + self . B * self . C - self . B;
    }
    pub fn test_print(&mut self, message: &str) -> ()
    {
        self.private_print("This is being printed in a private function");
        print!("Test Print: {}\n", message);
    }
    fn private_print(&mut self, message: &str) -> ()
    {
        print!("Private Print: {}\n", message);
    }
}


struct TypedefStruct {
    pub test: i32,
}


fn main() -> ()
{
    let mut rust_vector: Vec<i32> = Vec::new();
    let  const_mutablility_test: i64 = 10;
    let mut auto_mutablility_test: i64 = 10;
    let mut auto_non_mutablilty_test: i64 = 10;
    let mut test = 1;
    auto_mutablility_test += test;
    auto_non_mutablilty_test += test;
    rust_vector.push(1);
    rust_vector.push(2);
    rust_vector.push(3);
    rust_vector.pop();
    let mut structTest: ExampleStruct = ExampleStruct::new(15);
    structTest.test_print("This is from inside a struct");
    structTest.private_print("This is from inside a struct (FAIL)");
    structTest . A += structTest . B;
    structTest . B += structTest . B;
    structTest . C += structTest . B;
    print!("Testing print macro conversion {}", imported_var);
    rustImportTest::local_rust_function_test(structTest);
}

