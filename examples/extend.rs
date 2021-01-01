//! An important use of Lua is as a configuration language.
//! We will illustrate how we can use Lua to configure a program, starting with a simple example and
//! evolving it to perform increasingly complex tasks.
extern crate lua;

mod config {
    use std::{convert::TryFrom, fs::File, io::Read, path::Path};

    #[derive(Debug)]
    pub struct Config {
        width: u16,
        height: u16,
        background: Color,
    }

    #[derive(Debug)]
    pub struct Color(u8, u8, u8);

    const MAX_COLOR: u8 = 255;

    pub const COLOR_TABLE: [(&str, u8, u8, u8); 4] = [
        ("WHITE", MAX_COLOR, MAX_COLOR, MAX_COLOR),
        ("RED", MAX_COLOR, 0, 0),
        ("GREEN", 0, MAX_COLOR, 0),
        ("BLUE", 0, 0 , MAX_COLOR),
    ];

    impl TryFrom<&mut lua::State> for Color {
        type Error = lua::Error;
        fn try_from(state: &mut lua::State) -> Result<Self, Self::Error> {
            let tp = state.get_global("background")?;
            match tp {
                lua::types::LUA_TSTRING => {
                    let v = state.as_bytes(-1);
                    let name = std::str::from_utf8(v)?;
                    for (key, red, green, blue) in COLOR_TABLE.iter().cloned() {
                        if key.eq_ignore_ascii_case(name) {
                            return Ok(Color(red, green, blue));
                        }
                    }
                    Err(lua::Error::new(
                        lua::ErrorKind::InvalidInput,
                        format!("invalid color name ({})", name),
                    ))
                }
                lua::types::LUA_TTABLE => {
                    let red = get_color_field(state, "red")?;
                    let green = get_color_field(state, "green")?;
                    let blue = get_color_field(state, "blue")?;
                    Ok(Color(red, green, blue))
                }
                _ => Err(lua::Error::new(
                    lua::ErrorKind::InvalidInput,
                    "value on top of stack is not a table or string",
                ))
            }
        }
    }

    // assume that table is on the top of the stack
    fn get_color_field(state: &mut lua::State, key: &str) -> lua::Result<u8> {
        let mut state = lua::state::StackGuard::new(state);

        state.push_string(key)?; // push the key
        state.get_table(-2); // get background[key]
        let n: f32 = state.to_number(-1).ok_or_else(|| {
            lua::Error::new(
                lua::ErrorKind::InvalidInput,
                format!("invalid component {:?} in color", key),
            )
        })?;
        num_traits::cast(n * f32::from(MAX_COLOR)).ok_or_else(|| {
            lua::Error::new(
                lua::ErrorKind::InvalidInput,
                format!("invalid component {:?} in color", key),
            )
        })
    }

    impl Config {
        pub fn open<P: AsRef<Path>>(path: P, state: &mut lua::State) -> lua::Result<Self> {
            let mut state = lua::state::StackGuard::new(state);

            let mut file = File::open(path)?;
            let mut buf = Vec::with_capacity(1_024);
            file.read_to_end(&mut buf)?;

            state.load_string(buf)?;
            state.pcall(0, 0, 0)?;

            Ok(Self {
                width: get_global_u16(&mut state, "width")?,
                height: get_global_u16(&mut state, "height")?,
                background: Color::try_from(state.as_mut())?,
            })
        }
    }

    fn get_global_u16(state: &mut lua::State, name: &str) -> lua::Result<u16> {
        let mut state = lua::state::StackGuard::new(state);

        state.get_global(name)?;
        state.to_integer(-1).ok_or(lua::Error::new(
            lua::ErrorKind::InvalidInput,
            format!("{:?} should be a number", name),
        ))
    }
}

fn main() {
    env_logger::init();

    let mut state = lua::State::new();
    state.open_libs();

    match config::Config::open("examples/extend.lua", &mut state) {
        Ok(config) => {
            println!("config = {:?}", config);
        }
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }
}
