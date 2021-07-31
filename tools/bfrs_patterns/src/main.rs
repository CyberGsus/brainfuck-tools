use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;
use bfrs_common::{BFCommand, Position};

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
enum Pattern {
    Instruction(BFCommand),
    Address { binding: String },
}

struct MatchResult<'a> {
    pub commands: &'a [BFCommand],
    pub relative_offsets: HashMap<String, HashMap<String, isize>>,
}

#[derive(Debug)]
enum ParseError {
    MissingLB(Position),
    MissingRB(Position),
    UnknownChar { bad_char: char, position: Position },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingLB(lb_pos) => write!(f, "Unmatched right bracket at {}", lb_pos),
            Self::MissingRB(rb_pos) => {
                write!(f, "Unclosed loop: last opening was found at {}", rb_pos)
            }
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
        use std::io::Read;
        let mut input = File::open(opt.file)?;
        let mut str = Vec::new();
        input.read_to_end(&mut str)?;
        str
    };
    let instructions = parse_instructions(&src)?;

    for res in find_all_matches(&instructions, &pat) {
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

fn find_match<'a>(instructions: &'a [BFCommand], patterns: &[Pattern]) -> Option<MatchResult<'a>> {
    let mut last_known_address = None;
    let mut registry = HashMap::new();
    let mut offset = 0;

    for target in patterns {
        match target {
            // literal instructions are checked directly against the source
            Pattern::Instruction(instr) => {
                if instructions.get(offset).filter(|&i| i == instr).is_some() {
                    offset += 1;
                } else {
                    return None;
                }
            }
            Pattern::Address { binding } => {
                // calculate the offset. The first direction instruction
                // dictates what direction is the offset going, and the rest
                // will be matched according to that.
                let binding_offset = if let Some(direction) = instructions
                    .get(offset)
                    .filter(|i| matches!(i, BFCommand::Left | BFCommand::Right))
                {
                    let current_offset = offset;
                    offset += 1;
                    while instructions
                        .get(offset)
                        .filter(|&i| i == direction)
                        .is_some()
                    {
                        offset += 1;
                    }
                    (offset as isize - current_offset as isize)
                        * match direction {
                            BFCommand::Left => -1,
                            BFCommand::Right => 1,
                            _ => unreachable!(),
                        }
                } else {
                    0
                };

                match last_known_address {
                    None => {
                        // first address always matches.
                        // insert the value into the registry
                        // with a recorded offset to itself (0)
                        registry.insert(binding.clone(), {
                            let mut map = HashMap::new();
                            map.insert(binding.clone(), 0);
                            map
                        });
                    }
                    Some(ref last) => {
                        if !registry.contains_key(binding) {
                            // Adjust the offsets from all currently known keys,
                            // using their respective offset from `last`.
                            let mut this_offsets: HashMap<String, isize> = registry
                                .iter()
                                .map(|(other_k, other_map)| {
                                    (other_k.clone(), other_map[last] + binding_offset)
                                })
                                .collect();

                            this_offsets.insert(binding.clone(), 0);

                            // add the offset to every key, based on their
                            // respective offset from `last`.
                            for (_, map) in registry.iter_mut() {
                                map.insert(binding.clone(), -map[last] - binding_offset);
                            }

                            registry.insert(binding.clone(), this_offsets);

                            // a new binding always matches, since there's no way to
                            // check the validity of its position.
                        } else {
                            // for the address to be consistent, the offset
                            // that was computed now must be the same as what was computed
                            // when it was first introduced
                            let bind_map = &registry[binding];

                            if bind_map[last] != binding_offset {
                                return None;
                            }
                        }
                    }
                }

                last_known_address = Some(binding.clone());
            }
        }
    }
    Some(MatchResult {
        commands: if offset == 0 {
            instructions
        } else {
            &instructions[..offset]
        },
        relative_offsets: registry,
    })
}

fn find_all_matches<'a>(
    instructions: &'a [BFCommand],
    patterns: &[Pattern],
) -> Vec<MatchResult<'a>> {
    let mut offset = 0;
    let mut out = Vec::new();

    while instructions.len() > offset {
        if let Some(res) = find_match(&instructions[offset..], patterns) {
            offset += res.commands.len();
            out.push(res);
        } else {
            offset += 1;
        }
    }

    out
}

fn parse_instructions(source: &[u8]) -> Result<Vec<BFCommand>, ParseError> {
    let mut output = Vec::new();
    let mut loop_backlog = Vec::new();
    let mut current_pos = Position::default();
    for byte in source {
        if let Some(instr) = BFCommand::from_u8(*byte) {
            match instr {
                BFCommand::BeginLoop => loop_backlog.push(current_pos),
                BFCommand::EndLoop => {
                    if loop_backlog.pop().is_none() {
                        return Err(ParseError::MissingLB(current_pos));
                    }
                }
                _ => (),
            }
            output.push(instr);
        }
        if byte.is_ascii() {
            current_pos.advance_char(*byte as char)
        } else {
            current_pos.advance_col()
        }
    }
    if let Some(pos) = loop_backlog.pop() {
        Err(ParseError::MissingRB(pos))
    } else {
        Ok(output)
    }
}

fn parse_pattern(line: &str) -> Result<Vec<Pattern>, ParseError> {
    let mut output = Vec::new();
    let mut loop_backlog = Vec::new();
    let line_chars: Vec<_> = line.chars().collect();
    let mut line_i = 0;
    let mut current_pos = Position::default();
    while let Some(&ch) = line_chars.get(line_i) {
        if ch.is_ascii() {
            if let Some(instr) = BFCommand::from_u8(ch as u8) {
                match instr {
                    BFCommand::BeginLoop => loop_backlog.push(current_pos),
                    BFCommand::EndLoop => {
                        if loop_backlog.pop().is_none() {
                            return Err(ParseError::MissingLB(current_pos));
                        }
                    }
                    _ => (),
                }
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
    if let Some(pos) = loop_backlog.pop() {
        Err(ParseError::MissingRB(pos))
    } else {
        Ok(output)
    }
}
