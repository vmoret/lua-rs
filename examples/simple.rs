extern crate lua;

extern crate serde;
#[macro_use] extern crate serde_derive;

use std::fs::File;

use serde::Serialize;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Config {
    employees: Vec<Employee>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct Employee {
    id: i32,
    info: PersonInfo,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct PersonInfo {
    age: u8,
    gender: Gender,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
enum Gender {
    Male,
    Female,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut stack = lua::Stack::new();

    let config = Config {
        employees: vec![
            Employee {
                id: 1023,
                info: PersonInfo {
                    age: 54,
                    gender: Gender::Female,
                }
            },
            Employee {
                id: 2027,
                info: PersonInfo {
                    age: 24,
                    gender: Gender::Male,
                }
            }
        ],
    };

    let ret = config.serialize(&mut stack)?;
    println!("ret = {}", ret);
    dump(&stack);

    let c: Config = stack.get()?;
    println!("config = {:?}", c);
    dump(&stack);
    
    let mut file = File::open("examples/simple.lua")?;
    stack.load_buffer(&mut file, "simple", lua::Mode::Text)?;
    dump(&stack);
    
    stack.call(0, None)?;
    dump(&stack);
    
    let globals = lua::Globals::new(&stack);
    let width: u16 = globals.get("width")?;
    println!("width = {}", width);
    dump(&stack);

    let height: u16 = globals.get("height")?;
    println!("height = {}", height);
    dump(&stack);

    stack.push_slice(&[1u16, 2u16, 3u16, 4u16, 5u16, 6u16, 7u16, 8u16])?;
    dump(&stack);
    println!("value type = {}", stack.value_type(-1));
    println!("value type = {}", stack.value_type(1));

    Ok(())
}

fn dump(stack: &lua::Stack) {
    let dump = stack.dump();
    println!("Stack");
    println!("stack size = {}", stack.top());
    for (lvl, name) in dump {
        println!("{}: {}", lvl, name);
    }
    println!()
}