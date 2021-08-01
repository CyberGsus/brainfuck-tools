use bfrs_common::{BFCommand, Position};
use std::collections::HashMap;
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

/// A state machine to keep track of local state
/// in a matching context
struct MatchSM<'a> {
    instructions: &'a [BFCommand],
    registry: HashMap<String, HashMap<String, isize>>,
    offset: usize,
    last_binding: Option<String>,
}

// NOTE: there's A TON of string copying being involved. Take a look at this.
// Probably using only `usize` in the registry, and having a separate binding vector
// is the way to go. It would be interesting that a pattern can have a scope of associated
// binding names, and use numbers there as well.
impl<'a> MatchSM<'a> {
    /// Obtain all possible matches from the same pattern group
    // NOTE: make pattern groups a distinction from a pattern itself.
    pub fn find_all(instructions: &'a [BFCommand], patterns: &[Pattern]) -> Vec<MatchResult<'a>> {
        let mut offset = 0;
        let mut result = Vec::new();
        while offset < instructions.len() {
            if let Some(res) = Self::match_single(&instructions[offset..], patterns) {
                // advance by the match length.
                offset += res.commands.len();
                result.push(res);
            } else {
                offset += 1;
            }
        }
        result
    }
    /// Match a pattern through the beginning of the instructions
    pub fn match_single(
        instructions: &'a [BFCommand],
        patterns: &[Pattern],
    ) -> Option<MatchResult<'a>> {
        let mut machine = Self::new(instructions);
        for pat in patterns {
            if let Some(optional_action) = machine.match_target(pat) {
                if let Some(action) = optional_action {
                    machine.run_action(action);
                }
            } else {
                return None;
            }
        }
        Some(MatchResult {
            commands: if machine.offset == 0 {
                instructions
            } else {
                &instructions[..machine.offset]
            },
            relative_offsets: machine.registry,
        })
    }
    fn new(instructions: &'a [BFCommand]) -> Self {
        Self {
            instructions,
            registry: HashMap::new(),
            offset: 0,
            last_binding: None,
        }
    }
    fn run_action(&mut self, action: MatchSMAction) {
        match action {
            MatchSMAction::AdvanceInput { amount } => self.offset += amount,
            MatchSMAction::SetLastBinding { name } => self.last_binding = Some(name),
            MatchSMAction::NewBinding {
                offset_from_last,
                name,
            } => {
                match self.last_binding {
                    None => {
                        // without a known last, the `offset_from_last` parameter
                        // is ignored and the binding is created with a single reference
                        // to itself.
                        self.registry.insert(name.clone(), {
                            let mut map = HashMap::new();
                            map.insert(name, 0);
                            map
                        });
                    }
                    Some(ref last) => {
                        // calculate the offsets from the name to the
                        // others, using its offset from the last one
                        // as the only common thing between them.
                        let this_offsets: HashMap<String, isize> = {
                            let mut initial: HashMap<_, _> = self
                                .registry
                                .iter()
                                .map(|(other_k, other_map)| {
                                    (other_k.clone(), other_map[last] + offset_from_last)
                                })
                                .collect();

                            initial.insert(name.clone(), 0);
                            initial
                        };

                        // now make edges in the opposite direction.
                        for (other_k, other_map) in self.registry.iter_mut() {
                            other_map.insert(name.clone(), -this_offsets[other_k]);
                        }

                        self.registry.insert(name, this_offsets);
                    }
                }
            }
            MatchSMAction::Chain(chain) => {
                // a chain of actions... it's simple to deal with.
                for action in chain {
                    self.run_action(action);
                }
            }
        }
    }

    fn match_target(&self, target: &Pattern) -> Option<Option<MatchSMAction>> {
        match target {
            Pattern::Instruction(instr) => self.match_instruction(*instr).map(Some),
            Pattern::Address { binding } => {
                let (offt, a) = self.calculate_offset();
                self.match_binding(binding, offt).map(|b| {
                    MatchSMAction::chain_optionals(a, b).map(|c| {
                        c.chain_with(MatchSMAction::SetLastBinding {
                            name: binding.clone(),
                        })
                    })
                })
            }
        }
    }

    fn match_binding(
        &self,
        binding: &String,
        offset_from_last: isize,
    ) -> Option<Option<MatchSMAction>> {
        match self.last_binding {
            Some(ref last) => {
                if !self.registry.contains_key(binding) {
                    // a first-time binding will always match,
                    // as there is no older position to compare it to.
                    Some(Some(MatchSMAction::NewBinding {
                        offset_from_last,
                        name: binding.clone(),
                    }))
                } else {
                    // with a known last for reference, the offset
                    // can be checked for consistency with the previously
                    // recorded offset.
                    if self.registry[binding][last] == offset_from_last {
                        // success, but nothing to do.
                        Some(None)
                    } else {
                        None
                    }
                }
            }
            None => {
                // with no known last, a first-time binding will always
                // match, as it's certain that there is no registry of the
                // binding itself, nor is a registry of any other binding to
                // compare its solidity.
                Some(Some(MatchSMAction::NewBinding {
                    offset_from_last,
                    name: binding.clone(),
                }))
            }
        }
    }

    /// literal instructions are checked directly against the source
    fn match_instruction(&self, instruction: BFCommand) -> Option<MatchSMAction> {
        self.instructions
            .get(self.offset)
            .filter(|&&i| i == instruction)
            .map(|_| MatchSMAction::AdvanceInput { amount: 1 })
    }

    /// calculate an offset from the current source. The first direction instruction
    /// dictates what direction is the offset going, and the rest
    /// will be matched according to that.
    fn calculate_offset(&self) -> (isize, Option<MatchSMAction>) {
        let mut local_offset = 0;
        if let Some(direction) = self
            .instructions
            .get(self.offset)
            .filter(|i| matches!(i, BFCommand::Left | BFCommand::Right))
        {
            local_offset += 1;
            while self
                .instructions
                .get(self.offset + local_offset)
                .filter(|&i| i == direction)
                .is_some()
            {
                local_offset += 1;
            }
            (
                local_offset as isize
                    * match direction {
                        BFCommand::Left => -1,
                        BFCommand::Right => 1,
                        _ => unreachable!(),
                    },
                Some(MatchSMAction::AdvanceInput {
                    amount: local_offset,
                }),
            )
        } else {
            (0, None)
        }
    }
}

/// Models the different state mutation actions
/// that the state machine has available. Lets you control exactly when the state is modified
#[derive(Debug)]
enum MatchSMAction {
    /// a new binding was discovered.
    NewBinding {
        offset_from_last: isize,
        name: String,
    },
    AdvanceInput {
        amount: usize,
    },
    SetLastBinding {
        name: String,
    },
    Chain(Vec<MatchSMAction>),
}

impl MatchSMAction {
    pub fn chain_with(self, other: Self) -> Self {
        match (self, other) {
            (Self::Chain(mut a_chain), Self::Chain(mut b_chain)) => Self::Chain({
                a_chain.reserve(b_chain.len());
                let drain = b_chain.drain(..);
                a_chain.extend(drain);
                a_chain
            }),
            (Self::Chain(mut chain), other) | (other, Self::Chain(mut chain)) => Self::Chain({
                chain.push(other);
                chain
            }),
            (a, b) => Self::Chain(vec![a, b]),
        }
    }

    pub fn optional_chain(self, other: Option<Self>) -> Self {
        if let Some(other) = other {
            self.chain_with(other)
        } else {
            self
        }
    }

    pub fn chain_optionals(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (Some(a), b) | (b, Some(a)) => Some(a.optional_chain(b)),
            _ => None,
        }
    }
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
