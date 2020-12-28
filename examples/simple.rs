extern crate lua;

extern crate serde;
#[macro_use] extern crate serde_derive;

use serde::Serialize;

#[derive(Debug, Serialize)]
struct Config {
    employees: Vec<Employee>,
}

#[derive(Debug, Serialize)]
struct Employee {
    id: i32,
    info: PersonInfo,
}

#[derive(Debug, Serialize)]
struct PersonInfo {
    age: u8,
    gender: Gender,
}

#[derive(Debug, Serialize)]
enum Gender {
    Male,
    Female,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state = lua::State::new();

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

    let ret = config.serialize(state.clone())?;
    println!("ret = {}", ret);
    println!("stack size = {}", state.get_top());
    
    Ok(())
}