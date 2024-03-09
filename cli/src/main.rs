use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    process,
};

use runtime::vm::VirtualMachine;
use shared::error::InterpretResult;

const REPL_SIGN: &str = ">>";

fn main() -> InterpretResult {
    let args: Vec<String> = env::args().collect();
    let mut vm = VirtualMachine::new();

    match args.len() {
        1 => repl(&mut vm),
        2 => run_file(&mut vm, &args[1]),
        _ => {
            eprintln!("Usage: ruslox [script]");
            process::exit(64);
        }
    }
}

fn repl(vm: &mut VirtualMachine) -> InterpretResult {
    let mut line = String::new();
    loop {
        line.clear();

        print!("{} ", REPL_SIGN);
        io::stdout().flush()?;
        io::stdin().read_line(&mut line)?;

        if line.trim().is_empty() {
            return Ok(());
        }
        if let Err(err) = run(vm, &line) {
            eprintln!("{}", err);
            vm.clear_stack();
        }
    }
}

fn run_file(vm: &mut VirtualMachine, path: impl AsRef<Path>) -> InterpretResult {
    let source = fs::read_to_string(path)?;
    run(vm, source)
}

fn run(vm: &mut VirtualMachine, source: impl AsRef<str>) -> InterpretResult {
    let chunk = compiler::compile(source)?;
    vm.interpret(chunk)
}
