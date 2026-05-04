# C-Rust

Tired of programming in rust but still want memory safetly?
Well this probably wont fix that but its a start.

C-Rust acts as a preprocessing layer for a new C-Style syntax (See Examples folder).
Once it has processed your files it will automatically create a cargo directory and build the project.

---

## How to use

1. Create a valid `.crs` file
2. Open the terminal
3. Run the following command:
   ```
   C-Rust input.crs -o output_directory
   ```
4. The file is then processed and compiled
5. To run the file, either run the exectuable or instead add `-r` to automatically run once its built:
   ```
   C-Rust input.crs -o output_directory -r
   ```

---

## Syntax Changes

### Variable Definitions

In rust, variable defintions follow essentially two forms:

```rust
let varaible       = 10; // No Typing
let varaible : i32 = 10; //    Typing
```

Now varaibles follow the standard C format of:

```c
int variable = 10;
```

The keyword `auto` is also supported if they type is decernable.

```c
auto variable = 10;
```

---

### Mutablility

By default everything is mutable. If you want a non-mutable, use `static`:

```c
static int variable = 10;
```

---

### Include Statments

There are 3 main types of includes and, for the most part, all use the same syntax. Those types are:

1. **External Rust Libraries** — This is any rust code you did not write. Uses angle brackets `<>`. Same as a `use` statment.
2. **Local Rust Files** — `.rs` files that you wrote and want to use. Uses quotation marks `""`. Path to file.
3. **Local C-Rust Files** — `.crs`/`.hrs` files that you wrote and want to use. Uses quotation marks `""`. Path to file.

```c
//  In order
#include <std::fmt::{Display, Result, Formatter}>
#include "other.hrs"
#include "rustImportTest.rs"
```

---

### Define Directives

Some of the standard C defined directinves are supported in C-Rust, exactly the same as they are in a compiler. C-Rust also adds an internal define of `RUST` for `#ifdef` or any other statments.

Valid Directives:

```c
#define
#undef
#include
#ifdef
#ifndef
#endif
```

---

### Function Definitions

Similar changes have been made to the function system. In rust it is:

```rust
fn function(parameter : i32) -> Result
```

New syntax:

```c
Result function(int parameter)
```

---

### Return Statments

You have to explicitly say return. You cant just right the variable anymore.
Its like one word why did they skip it before?

```c
return variable;
```

---

### ++/-- Operators

I have added back in the `++`/`--` operators form C, however, only prefix version:

```c
variable++; variable--;
```

---

### Macro Mapping

In order to keep macro functionality in the new syntax, they take the following form:

```c
Macro::print("A print in {}", "C-Rust");
```

---

### For loops

I have brought back the original C style for loops. For each is not supported at this time.

```c
for (int i = 0; i < 10; i++)
```

---

### Enum Definitions

Enums have been simplified back into their orignal C usage.
Stop using them like structs. (Im aware of the irony if you look at my code)
I did leave the ability for them to implement basic traits, like debug for example.

```c
enum TestEnum impliments Debug
{
    Option1,
    Option2
};
```

---

### Struct Definitions

This is probably where most of my original frustration came from, and so has the most changes.

Structs can use `implements` the same way it is used for enums:

```c
struct Example impliments Debug
{
    int test;
};
```

Next I added implicit default values for struct members. These later get automatically applied for the constructor:

```c
struct Example
{
    int test = 10;
};
```

Another change is that everything is public by default. However, this visibility system works the same way it does in Rust, not how it works in C. i.e. if its in the same file, it can access it. To change the visibility:

```c
struct Example
{
    private:
        int NotVisible;
    public:
        int Visible
};
```

The biggest change is function are defined in the main block of the struct, not in a seperate section. They follow the same format as normal functions, but carry an implicit `&mut self` parameter, mapped as `this`:

```c
struct Example
{
    int math_and_stuff()
    {
        return this.A + this.B * (this.C - this.B);
    }
};
```

This one is what annoyed me the most. I have added constructors back in. They now automatically map to a `::new()` function. Any `this.*` statments are mapped to the Rust initalizer. At this time you can still only have one.

```c
struct Example
{
    Example(int param)
    {
        this.test = param;
    }
};
```

This also maps correctly when you try to use the constructor later:

```c
//  In C-Rust
struct ExampleStruct structTest = ExampleStruct(15);
//  Or when using with rust
let structTest = ExampleStruct::new(15);
```

When implementing a function needed by a trait, you can also do that inside the main struct body. However these only recieve a `&self` rather than `&mut self` at this time.

```c
struct Example
{
    Result Display::fmt(Formatter& f)
    {
        return Macro::write(f, "A: {}, B: {}", this.A, this.B);
    }
}
```