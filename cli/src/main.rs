use std::{
    env, fs,
    io::{self, Write},
    path::Path,
};

use runtime::vm::VirtualMachine;
use shared::error::SourceFileManager;

const REPL_SIGN: &str = ">>";

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut vm = VirtualMachine::new();

    match args.len() {
        1 => repl(&mut vm)?,
        2 => run_file(&mut vm, &args[1])?,
        _ => {
            eprintln!("Usage: ruslox [script]");
        }
    }
    Ok(())
}

fn repl(vm: &mut VirtualMachine) -> io::Result<()> {
    let mut line = String::new();
    loop {
        line.clear();

        print!("{} ", REPL_SIGN);
        io::stdout().flush()?;
        io::stdin().read_line(&mut line)?;

        if line.trim().is_empty() {
            return Ok(());
        }
        run(vm, &line, "<input>");
    }
}

fn run_file(vm: &mut VirtualMachine, path: impl AsRef<Path>) -> io::Result<()> {
    let filename = path.as_ref().to_string_lossy().into_owned();
    let source = fs::read_to_string(path)?;
    run(vm, source, filename);
    Ok(())
}

fn run(vm: &mut VirtualMachine, source: impl AsRef<str>, filename: impl AsRef<str>) {
    // codespan-reporting environments.
    let mut files = SourceFileManager::new();
    let file_id = files.add(filename.as_ref(), source.as_ref());

    match compiler::compile(file_id, source.as_ref()) {
        Ok(chunk) => {
            if let Err(error) = vm.interpret(chunk) {
                error.emit(&files);
            }
            vm.clear_stack();
        }
        Err(error) => error.emit(&files),
    }
}
