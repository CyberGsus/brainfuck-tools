//! New way of managing patterns

use bfrs_common::errors as bfrs_errors;
use bfrs_common::{BFCommand, Position};
use bimap::BiMap;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct PatternScope {
    pub bindings: BiMap<usize, String>,
    pub patterns: Vec<Pattern>,
}

#[derive(Debug)]
pub enum Pattern {
    /// A single instruction
    Instruction(BFCommand),
    /// A single binding
    Binding {
        index: usize,
        /// A strict binding match ends with `!` and requires
        /// to have an offset with the last binding encountered more than zero.
        strict: bool,
    },
}

// NOTE: will have to refactor this to
// a structure and state management enums
// so the parser can be streamlined
pub fn parse_pattern(src: &str) -> ParseResult<PatternScope> {
    let mut current_pos = Position::default();
    let src: Vec<_> = src.chars().collect();
    let mut offset_i = 0;
    let mut bindings = BiMap::new();
    let mut patterns = Vec::new();
    while let Some(&ch) = src.get(offset_i) {
        if ch.is_ascii() {
            if let Some(instr) = BFCommand::from_u8(ch as u8) {
                current_pos.advance_char(ch);
                offset_i += 1;
                patterns.push(Pattern::Instruction(instr));
                continue;
            }
        }
        // as long as you don't interfere with any instruction, you can name your
        // shit whatever you want.
        if ch.is_alphabetic() {
            let mut str = String::new();
            str.push(ch);
            current_pos.advance_char(ch);
            offset_i += 1;
            while let Some(&ch) = src.get(offset_i).filter(|&&ch| ch.is_alphanumeric()) {
                str.push(ch);
                current_pos.advance_char(ch);
                offset_i += 1;
            }
            let strict = if matches!(src.get(offset_i), Some(&'!')) {
                offset_i += 1;
                current_pos.advance_char('!');
                true
            } else {
                false
            };
            let index = if let Some(i) = bindings.get_by_right(&str) {
                *i
            } else {
                let len = bindings.len();
                bindings.insert(len, str);
                len
            };
            patterns.push(Pattern::Binding { index, strict });
            continue;
        } else if !ch.is_whitespace() {
            return Err(bfrs_errors::ErrorWithPosition {
                kind: ParseError::UnknownChar { bad_char: ch },
                position: current_pos,
            });
        }
        current_pos.advance_char(ch);
        offset_i += 1;
    }
    Ok(PatternScope { bindings, patterns })
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
