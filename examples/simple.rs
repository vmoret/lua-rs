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

    let mut state = lua::State::new();

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

    let ret = config.serialize(&mut state)?;
    println!("ret = {}", ret);
    println!("stack size = {}", state.as_stack().top());

    let c: Config = state.get()?;
    println!("config = {:?}", c);
    println!("stack size = {}", state.as_stack().top());
    
    let mut file = File::open("examples/simple.lua")?;
    state.load_buffer(&mut file, "simple", lua::Mode::Text)?;
    println!("stack size = {}", state.as_stack().top());
    
    state.call(0, 0, 0)?;
    println!("stack size = {}", state.as_stack().top());
    
    let globals = state.as_globals();
    let width: u16 = globals.get("width")?;
    println!("width = {}", width);
    println!("stack size = {}", state.as_stack().top());

    let height: u16 = globals.get("height")?;
    println!("height = {}", height);
    println!("stack size = {}", state.as_stack().top());

    Ok(())
}