use std::error::Error;
use std::fs::File;
use std::io;
#[repr(u8)]
enum Instruction {
    GoLeft,
    GoRight,
    Increment,
    Decrement,
    // stores where the loop ends
    BeginLoop,
    // stores where the loop starts
    EndLoop,
    Print,
    Read,
}

impl Instruction {
    pub fn from_u8(byte: u8) -> Option<Self> {
        Some(match byte {
            b'+' => Self::Increment,
            b'-' => Self::Decrement,
            b'[' => Self::BeginLoop,
            b']' => Self::EndLoop,
            b'.' => Self::Print,
            b',' => Self::Read,
            b'<' => Self::GoLeft,
            b'>' => Self::GoRight,
            _ => return None,
        })
    }
}

impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Increment => '+',
                Self::Decrement => '-',
                Self::BeginLoop => '[',
                Self::EndLoop => ']',
                Self::Print => '.',
                Self::Read => ',',
                Self::GoLeft => '<',
                Self::GoRight => '>',
            }
        )
    }
}

#[derive(Debug)]
enum ParseError {
    MissingLB(usize),
    MissingRB(usize),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingLB(i) => {
                write!(f, "Missing matching square brace for closing one at {}", i)
            }
            Self::MissingRB(i) => {
                write!(f, "Missing matching square brace for opening one at {}", i)
            }
        }
    }
}

impl Error for ParseError {}

fn parse(bytes: &[u8]) -> Result<Program, ParseError> {
    let mut instructions = Vec::new();
    let mut begin_vec = Vec::new();
    let mut jumps = HashMap::new();
    for (i, &byte) in bytes.iter().enumerate() {
        if let Some(s) = Instruction::from_u8(byte) {
            match s {
                Instruction::BeginLoop => begin_vec.push(instructions.len()),
                Instruction::EndLoop => match begin_vec.pop() {
                    None => return Err(ParseError::MissingLB(i)),
                    Some(end) => {
                        let curr_i = instructions.len();
                        jumps.insert(end, curr_i);
                        jumps.insert(curr_i, end);
                    }
                },
                _ => (),
            }
            instructions.push(s);
        }
    }
    if let Some(i) = begin_vec.pop() {
        Err(ParseError::MissingRB(i))
    } else {
        Ok(Program {
            instructions,
            jumps,
            tape_size: 30000,
        })
    }
}

use std::collections::HashMap;

struct Program {
    instructions: Vec<Instruction>,
    tape_size: usize,
    jumps: HashMap<usize, usize>,
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
            Instruction::BeginLoop => {
                if tape[pivot] == 0 {
                    instruction_i = target.jumps[&instruction_i];
                }
            }
            Instruction::EndLoop => {
                if tape[pivot] != 0 {
                    instruction_i = target.jumps[&instruction_i];
                }
            }
            Instruction::Decrement => tape[pivot] = tape[pivot].wrapping_sub(1),
            Instruction::GoLeft => {
                pivot = if pivot == 0 {
                    target.tape_size - 1
                } else {
                    pivot - 1
                }
            }
            Instruction::GoRight => {
                pivot = if pivot == target.tape_size - 1 {
                    0
                } else {
                    pivot + 1
                }
            }
            Instruction::Increment => tape[pivot] = tape[pivot].wrapping_add(1),
            Instruction::Print => {
                let mut lock = stdout.lock();
                lock.write_all(&tape[pivot..pivot + 1])?;
                lock.flush()?;
            }
            Instruction::Read => {
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

    for (i, instr) in program.instructions.iter().enumerate() {
        match instr {
            Instruction::BeginLoop => {
                current_color -= 1;
                print!("\x1b[38;5;{}m{}", current_color, instr);
            }
            Instruction::EndLoop => {
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
    File(File, String),
}

impl Input {
    fn from_optional_arg(arg: Option<String>) -> io::Result<Self> {
        match arg {
            Some(filename) if filename != "-" => {
                File::open(&filename).map(|x| Self::File(x, filename))
            }
            _ => {
                eprintln!("[-][Reading from stdin]");
                Ok(Self::Stdin(io::stdin()))
            }
        }
    }
}

impl io::Read for Input {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Stdin(s) => s.lock().read(buf),
            Self::File(f, _) => f.read(buf),
        }
    }
}

impl std::fmt::Display for Input {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::File(_, fname) => write!(f, "{}", fname),
            Self::Stdin(_) => write!(f, "<stdin>"),
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
    use std::io::Read;
    use std::time::Instant;
    let opt = Opt::from_args();
    let mut input = Input::from_optional_arg(opt.input)?;
    let mut input_bytes = Vec::new();
    input.read_to_end(&mut input_bytes)?;
    let mut program = parse(&input_bytes)?;
    program.tape_size = opt.cells;
    if opt.highlight_only {
        highlight_code(&program);
    } else {
        let start_time = Instant::now();
        let result_tape = interpret(&program)?;
        let time = Instant::now().duration_since(start_time);
        eprintln!("program {} executed in {}us", input, time.as_micros());
        if opt.show_tape {
            eprintln!("result tape: {:?}", result_tape);
        }
    }
    Ok(())
}
