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
    println!("stack size = {}", stack.top());

    let c: Config = stack.get()?;
    println!("config = {:?}", c);
    println!("stack size = {}", stack.top());
    
    let mut file = File::open("examples/simple.lua")?;
    stack.load_buffer(&mut file, "simple", lua::Mode::Text)?;
    println!("stack size = {}", stack.top());
    
    stack.call(0, 0, 0)?;
    println!("stack size = {}", stack.top());
    
    let globals = stack.as_globals();
    let width: u16 = globals.get("width")?;
    println!("width = {}", width);
    println!("stack size = {}", stack.top());

    let height: u16 = globals.get("height")?;
    println!("height = {}", height);
    println!("stack size = {}", stack.top());

    Ok(())
}