//! A bare-bones stand-alone Lua interpreter
extern crate lua;

use std::io::{self, BufRead};

fn main() -> lua::Result<()> {
    let stdin = io::stdin();

    // opens Lua
    let mut state = lua::State::new();

    // opens the standard libraries
    state.open_libs();

    for line in stdin.lock().lines() {
        let line = line.unwrap();

        let ret = state.load_string(line).and_then(|_| {
            state.pcall(0, 0, 0)
        });

        if let Err(e) = ret {
            println!("{}", e);

            // pope error message
            state.pop(-1);
        }
    }

    Ok(())
}
