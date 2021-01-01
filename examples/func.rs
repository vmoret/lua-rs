//! Calling C from Lua
extern crate lua;

use std::{fs::File, io::Read};

use lua::state::{RustFunction, Pull};

fn main() -> lua::Result<()> {
    env_logger::init();

    // opens Lua
    let mut state = lua::State::new();

    // opens the standard libraries
    state.open_libs();

    // register our function
    let func = RustFunction::new(|n: f32| Ok(n.sin()));
    state.push(func)?;
    state.set_global("mysin")?;

    let mut file = File::open("examples/func.lua")?;
    let mut buf = Vec::with_capacity(1_024);
    file.read_to_end(&mut buf)?;

    state.load_string(buf)?;
    state.pcall(0, 0, 0)?;

    state.get_global("width")?;
    println!("result = {}", f64::pull(&state, -1)?);

    Ok(())
}
