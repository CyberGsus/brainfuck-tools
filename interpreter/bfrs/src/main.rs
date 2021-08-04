use bfrs_common::{parser, BFCommand};
use std::error::Error;
use std::fs::File;
use std::io;

use std::collections::HashMap;

struct Program {
    instructions: Vec<BFCommand>,
    tape_size: usize,
    jumps: HashMap<usize, usize>,
}

impl Program {
    pub fn from_instructions(instructions: Vec<BFCommand>, tape_size: usize) -> Self {
        let mut jumps = HashMap::new();
        let mut jumps_backlog = Vec::new();
        for (i, instr) in instructions.iter().enumerate() {
            match instr {
                BFCommand::BeginLoop => jumps_backlog.push(i),
                BFCommand::EndLoop => {
                    let other_i = jumps_backlog.pop().unwrap();
                    jumps.insert(other_i, i);
                    jumps.insert(i, other_i);
                }
                _ => (),
            }
        }
        Program {
            instructions,
            tape_size,
            jumps,
        }
    }
}

fn interpret(target: &Program) -> io::Result<Vec<u8>> {
    use std::io::{Read, Write};
    let mut instruction_i = 0;
    let mut tape: Vec<u8> = Vec::with_capacity(target.tape_size);
    unsafe {
        tape.set_len(target.tape_size);
    }
    for x in tape.iter_mut() {
        *x = 0;
    }
    let mut pivot = 0;
    let stdin = io::stdin();
    let stdout = io::stdout();

    while let Some(i) = target.instructions.get(instruction_i) {
        match i {
            BFCommand::BeginLoop => {
                if tape[pivot] == 0 {
                    instruction_i = target.jumps[&instruction_i];
                }
            }
            BFCommand::EndLoop => {
                if tape[pivot] != 0 {
                    instruction_i = target.jumps[&instruction_i];
                }
            }
            BFCommand::Decrement => tape[pivot] = tape[pivot].wrapping_sub(1),
            BFCommand::Left => {
                pivot = if pivot == 0 {
                    target.tape_size - 1
                } else {
                    pivot - 1
                }
            }
            BFCommand::Right => {
                pivot = if pivot == target.tape_size - 1 {
                    0
                } else {
                    pivot + 1
                }
            }
            BFCommand::Increment => tape[pivot] = tape[pivot].wrapping_add(1),
            BFCommand::Print => {
                let mut lock = stdout.lock();
                lock.write_all(&tape[pivot..pivot + 1])?;
                lock.flush()?;
            }
            BFCommand::Read => {
                let amt_read = stdin.lock().read(&mut tape[pivot..pivot + 1])?;
                if amt_read == 0 {
                    tape[pivot] = 255; // EOF translates to -1
                }
            }
        }
        instruction_i += 1;
    }

    Ok(tape)
}

fn highlight_code(program: &Program) {
    let mut current_color = 6;

    for instr in program.instructions.iter() {
        match instr {
            BFCommand::BeginLoop => {
                current_color -= 1;
                print!("\x1b[38;5;{}m{}", current_color, instr);
            }
            BFCommand::EndLoop => {
                current_color += 1;
                print!("]\x1b[38;5;{}m", current_color);
            }
            _ => print!("{}", instr),
        }
    }
    println!()
}

enum Input {
    Stdin(io::Stdin),
    File(File),
}

impl Input {
    fn from_optional_arg(arg: Option<String>) -> io::Result<(Self, String)> {
        match arg {
            Some(filename) if filename != "-" => {
                File::open(&filename).map(|x| (Self::File(x), filename))
            }
            _ => {
                eprintln!("[-][Reading from stdin]");
                Ok((Self::Stdin(io::stdin()), String::from("<stdin>")))
            }
        }
    }
}

impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Stdin(s) => s.lock().read(buf),
            Self::File(f) => f.read(buf),
        }
    }
}

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "bfrs", about = "a simple brainfuck interpreter")]
struct Opt {
    /// Amount of cells to use
    #[structopt(short, long, default_value = "30000")]
    cells: usize,

    /// Input file
    #[structopt()]
    input: Option<String>,

    /// Only highlight the code, don't run it
    #[structopt(long = "highlight")]
    highlight_only: bool,

    /// Show the tape after
    #[structopt(short, long)]
    show_tape: bool,
}

fn main() {
    if let Err(ref err) = run() {
        eprintln!("Error: {}", err);
        ::std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    use std::time::Instant;
    let opt = Opt::from_args();
    let (input, filename) = Input::from_optional_arg(opt.input)?;
    let instructions: Vec<_> =
        parser::parse(bfrs_input::bytes::BufferedBytes::new(input)).collect::<Result<_, _>>()?;
    let program = Program::from_instructions(instructions, opt.cells);
    if opt.highlight_only {
        highlight_code(&program);
    } else {
        let start_time = Instant::now();
        let result_tape = interpret(&program)?;
        let time = Instant::now().duration_since(start_time);
        eprintln!("program {} executed in {}us", filename, time.as_micros());
        if opt.show_tape {
            eprintln!("result tape: {:?}", result_tape);
        }
    }
    Ok(())
}
