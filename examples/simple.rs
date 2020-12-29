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
    println!("stack size = {}", state.get_top());

    let c: Config = state.deserialize()?;
    println!("config = {:?}", c);
    println!("stack size = {}", state.get_top());
    
    let mut file = File::open("examples/simple.lua")?;
    state.load_buffer(&mut file, "simple", lua::Mode::Text)?;
    println!("stack size = {}", state.get_top());

    Ok(())
}