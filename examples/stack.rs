//! Pushing, querying elements and other stack operations.
extern crate lua;

fn main() -> lua::Result<()> {
    let mut state = lua::State::new();

    state.push_boolean(true);

    {
        // guard the stack size, this will pop all but the first element from
        // the stack when going out of scope
        let mut state = lua::state::StackGuard::new(&mut state);
        state.push_number(10);
        state.push_nil();
        state.push_string("hello")?;
    
        dump_stack(&state);
    
        state.push_value(-4); dump_stack(&state);
    
        state.replace(3); dump_stack(&state);
    
        state.set_top(6); dump_stack(&state);
    
        state.rotate(3, 1); dump_stack(&state);
    
        state.remove(-3); dump_stack(&state);
    }

    dump_stack(&state);

    Ok(())
}

fn dump_stack(state: &lua::State) {
    println!();
    for info in state.dump() {
        println!("{}", info);
    }
}