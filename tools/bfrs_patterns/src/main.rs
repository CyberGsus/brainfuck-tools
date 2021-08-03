use bfrs_common::parser;
use bfrs_common::{BFCommand, Position};
use bfrs_input::bytes::BufferedBytes;
use bfrs_patterns::pattern::Pattern;
use bfrs_patterns::r#match::MatchSM;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "bfrs_patterns",
    about = "detect patterns within brainfuck code"
)]
struct Opt {
    /// the pattern to search for
    #[structopt(short, long)]
    pattern: String,

    /// the file to search in
    #[structopt(parse(from_os_str))]
    file: PathBuf,
}

#[derive(Debug)]
enum ParseError {
    UnknownChar { bad_char: char, position: Position },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UnknownChar { bad_char, position } => {
                write!(f, "invalid char at {}: {:?}", position, bad_char)
            }
        }
    }
}

impl Error for ParseError {}

fn main() {
    if let Err(ref e) = run() {
        eprintln!("Error: {}", e);
        ::std::process::exit(1)
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let pat = parse_pattern(&opt.pattern)?;
    let src = {
        use std::fs::File;
        let input = File::open(opt.file)?;
        BufferedBytes::new(input)
    };

    let instructions: Vec<_> = parser::parse(src).collect::<Result<_, _>>()?;

    for res in MatchSM::find_all(&instructions, &pat) {
        let str: String = res.commands.iter().map(|&i| i as u8 as char).collect();
        println!("result: `{}`", str);
        for (key, offsets) in res.relative_offsets {
            println!("offsets for `{}`", key);
            for (other, offt) in offsets.iter().filter(|(k, _)| k != &&key) {
                println!("\t`{}` -> {}", other, offt);
            }
        }
    }

    Ok(())
}

fn parse_pattern(line: &str) -> Result<Vec<Pattern>, ParseError> {
    let mut output = Vec::new();
    let line_chars: Vec<_> = line.chars().collect();
    let mut line_i = 0;
    let mut current_pos = Position::default();
    while let Some(&ch) = line_chars.get(line_i) {
        if ch.is_ascii() {
            if let Some(instr) = BFCommand::from_u8(ch as u8) {
                current_pos.advance_char(ch);
                output.push(Pattern::Instruction(instr));
                line_i += 1;
                continue;
            }
        }
        if ch == '_' || ch.is_alphabetic() {
            let mut name = String::from(ch);
            current_pos.advance_char(ch);
            line_i += 1;
            while let Some(&ch) = line_chars
                .get(line_i)
                .filter(|&&ch| ch == '_' || ch.is_alphanumeric())
            {
                current_pos.advance_char(ch);
                name.push(ch);
                line_i += 1;
            }
            output.push(Pattern::Address { binding: name });
            continue;
        }
        if ch.is_whitespace() {
            line_i += 1;
            continue;
        }
        return Err(ParseError::UnknownChar {
            bad_char: ch,
            position: current_pos,
        });
    }
    Ok(output)
}
