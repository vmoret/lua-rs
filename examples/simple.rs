extern crate lua;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut state = lua::State::new();

    Ok(())
}
