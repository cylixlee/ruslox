use std::{
    env, fs,
    io::{self, Write},
    path::Path,
};

use codespan_reporting::{
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use runtime::vm::VirtualMachine;

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

fn run(_vm: &mut VirtualMachine, source: impl AsRef<str>, filename: impl AsRef<str>) {
    let mut files = SimpleFiles::new();
    let file_id = files.add(filename.as_ref(), source.as_ref());
    let source = source.as_ref().chars().collect();
    if let Err(errors) = compiler::compile(&source, file_id) {
        let stream = StandardStream::stderr(ColorChoice::Always);
        let stream = &mut stream.lock();
        let config = Default::default();
        for error in &errors {
            term::emit(stream, &config, &files, error).expect("internal diagnostic error");
        }
    }
}
