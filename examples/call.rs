//! Calling Lua Functions
extern crate lua;

use std::{fs::File, io::Read, path::Path};

use lua::state::Function;

fn load<P: AsRef<Path>>(path: P, state: &mut lua::State) -> lua::Result<(f32, String)> {
    let mut state = lua::state::StackGuard::new(state);

    let mut file = File::open(path)?;
    let mut buf = Vec::with_capacity(1_024);
    file.read_to_end(&mut buf)?;

    state.load_string(buf)?;
    state.pcall(0, 0, 0)?;

    let f = Function::new(&mut state, "f");
    f(200.0, 300.0)
}

fn main() {
    env_logger::init();

    let mut state = lua::State::new();
    state.open_libs();

    match load("examples/call.lua", &mut state) {
        Ok(result) => {
            println!("result = {:?}", result);
        }
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }
}
