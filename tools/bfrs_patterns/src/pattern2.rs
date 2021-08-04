//! New way of managing patterns

use bfrs_common::errors as bfrs_errors;
use bfrs_common::{BFCommand, Position};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct PatternScope {
    pub bindings: HashMap<String, usize>,
    pub pattern: Pattern,
}

#[derive(Debug)]
pub enum Pattern {
    /// A single instruction
    Instruction(BFCommand),
    /// A single binding
    Binding(usize),
    /// A group built of patterns within the same scope
    Group(Vec<Pattern>),
}

impl Pattern {
    pub fn extend(self, other: Self) -> Self {
        match (self, other) {
            (Self::Group(mut a), Self::Group(mut b)) => {
                let drain = b.drain(..);
                a.extend(drain);
                Self::Group(a)
            }
            (a, b) => Self::Group(vec![a, b]),
        }
    }
    pub fn extend_optionally(a: Option<Self>, b: Self) -> Self {
        match a {
            None => b,
            Some(a) => a.extend(b),
        }
    }
}

// NOTE: will have to refactor this to
// a structure and state management enums
// so the parser can be streamlined
pub fn parse_pattern(src: &str) -> ParseResult<Option<PatternScope>> {
    let mut current_pos = Position::default();
    let src: Vec<_> = src.chars().collect();
    let mut offset_i = 0;
    let mut bindings = HashMap::new();
    let mut pattern = None;
    while let Some(&ch) = src.get(offset_i) {
        if ch.is_ascii() {
            if let Some(instr) = BFCommand::from_u8(ch as u8) {
                offset_i += 1;
                pattern = Some(Pattern::extend_optionally(
                    pattern,
                    Pattern::Instruction(instr),
                ));
            }
        // as long as you don't interfere with any instruction, you can name your
        // shit whatever you want.
        } else if !(ch.is_whitespace() || BFCommand::from_u8(ch as u8).is_some()) {
            let mut str = String::new();
            str.push(ch);
            offset_i += 1;
            while let Some(&ch) = src
                .get(offset_i)
                .filter(|&&ch| !(ch.is_whitespace() || BFCommand::from_u8(ch as u8).is_some()))
            {
                str.push(ch);
                offset_i += 1;
            }
            let len = bindings.len();
            let num = *bindings.entry(str).or_insert(len);
            pattern = Some(Pattern::extend_optionally(pattern, Pattern::Binding(num)));
            continue;
        } else if !ch.is_whitespace() {
            return Err(bfrs_errors::ErrorWithPosition {
                kind: ParseError::UnknownChar { bad_char: ch },
                position: current_pos,
            });
        }
        current_pos.advance_char(ch);
    }
    Ok(pattern.map(|pattern| PatternScope { bindings, pattern }))
}

type ParseResult<T> = Result<T, bfrs_errors::ErrorWithPosition<ParseError>>;

#[derive(Debug)]
pub enum ParseError {
    UnknownChar { bad_char: char },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UnknownChar { bad_char } => {
                write!(f, "Unknown character in source: {:?}", bad_char)
            }
        }
    }
}

impl Error for ParseError {}
