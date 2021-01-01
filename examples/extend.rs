//! An important use of Lua is as a configuration language.
//! We will illustrate how we can use Lua to configure a program, starting with a simple example and
//! evolving it to perform increasingly complex tasks.
extern crate lua;

mod config {
    use std::{fs::File, path::Path, io::Read};

    #[derive(Debug)]
    pub struct Config {
        width: u16,
        height: u16,
    }
    
    impl Config {
        pub fn open<P: AsRef<Path>>(path: P, state: &mut lua::State) -> lua::Result<Self> {
            let mut file = File::open(path)?;
            let mut buf = Vec::with_capacity(1_024);
            file.read_to_end(&mut buf)?;

            state.load_string(buf)?;
            state.pcall(0, 0, 0)?;

            Ok(Self {
                width: get_global_u16(state, "width")?,
                height: get_global_u16(state, "height")?,
            })
        }
    }

    fn get_global_u16(state: &mut lua::State, name: &str) -> lua::Result<u16> {
        state.get_global(name)?;
        state.to_integer(-1).ok_or_else(|| {
            // remove result from the stack
            state.pop(1);
    
            lua::Error::new(lua::ErrorKind::InvalidInput, format!("{:?} should be a number", name))
        })
    }
}

fn main() -> lua::Result<()> {

    let mut state = lua::State::new();
    state.open_libs();

    let config = config::Config::open("examples/extend.lua", &mut state)?;
    println!("config = {:?}", config);

    Ok(())
}