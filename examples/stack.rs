//! Pushing, querying elements and other stack operations.
extern crate lua;

fn main() -> lua::Result<()> {
    let mut state = lua::State::new();

    state.push_boolean(true);
    state.push_integer(1989);
    state.push_number(3000.0);
    state.push_string("foo bar")?;

    println!("{}", state.is_boolean(-4));
    println!("{}", state.is_integer(-3));
    println!("{}", state.is_number(-2));
    println!("{}", state.is_string(-1));

    println!("{}", state.to_boolean(-4));
    println!("{:?}", state.to_integer::<i64>(-3));
    println!("{:?}", state.to_number::<f64>(-2));
    println!("{:?}", state.as_bytes(-1));
    
    for info in state.dump() {
        println!("{}", info);
    }

    Ok(())
}