//! An important use of Lua is as a configuration language.
//! We will illustrate how we can use Lua to configure a program, starting with a simple example and
//! evolving it to perform increasingly complex tasks.
extern crate lua;

mod config {
    use std::{fs::File, path::Path, io::Read};

    use crate::get_global_integer;

    #[derive(Debug)]
    pub struct Config {
        width: i64,
        height: i64,
    }
    
    impl Config {
        pub fn open<P: AsRef<Path>>(path: P, state: &mut lua::State) -> lua::Result<Self> {
            let mut file = File::open(path)?;
            let mut buf = Vec::with_capacity(1_024);
            file.read_to_end(&mut buf)?;

            state.load_string(buf)?;
            state.pcall(0, 0, 0)?;

            Ok(Self {
                width: get_global_integer(state, "width")?,
                height: get_global_integer(state, "height")?,
            })
        }
    }
}

fn get_global_integer(state: &mut lua::State, name: &str) -> lua::Result<i64> {
    state.get_global(name)?;
    state.to_integer(-1).ok_or_else(|| {
        // remove result from the stack
        state.pop(1);

        lua::Error::new(lua::ErrorKind::InvalidInput, format!("{:?} should be a number", name))
    })
}

fn main() -> lua::Result<()> {

    let mut state = lua::State::new();

    let config = config::Config::open("examples/extend.lua", &mut state)?;
    println!("config = {:?}", config);

    Ok(())
}