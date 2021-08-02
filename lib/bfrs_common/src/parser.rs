use super::{BFCommand, Position};
use std::error::Error;
use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, IOParserErr>;

pub fn parse<I>(input: I) -> BFParserIter<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    BFParser::new(input).into_iter()
}

pub fn parse_starting_at<I>(input: I, start_pos: Position) -> BFParserIter<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    BFParser::starting_at(input, start_pos).into_iter()
}

// note: maybe move this to a more generic thing?
#[derive(Debug, Clone, Copy)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub position: Position,
}

#[derive(Debug, Clone, Copy)]
pub enum ParseErrorKind {
    MissingLB,
    MissingRB(Position),
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::MissingLB => write!(f, "Unmatched loop closing"),
            Self::MissingRB(last_lb) => {
                write!(f, "Unclosed loop: last opening was found at {}", last_lb)
            }
        }
    }
}

impl Error for ParseErrorKind {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.position, self.kind)
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

struct BFParser<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    input: std::iter::Fuse<I>,
    current_position: Position,
    loop_backlog: Vec<Position>,
}

#[derive(Debug)]
pub enum IOParserErr {
    IO(io::Error),
    Parser(ParseError),
}

impl fmt::Display for IOParserErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IO(e) => write!(f, "an IO error occurred while trying to read bytes: {}", e),
            Self::Parser(p) => write!(f, "parse error: {}", p),
        }
    }
}

impl Error for IOParserErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(match self {
            Self::IO(e) => e,
            Self::Parser(e) => e,
        })
    }
}

impl<I> BFParser<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    #[inline]
    /// Starts parsing, setting the initial position to `start_pos`
    fn starting_at(input: I, start_pos: Position) -> Self {
        Self {
            input: Iterator::fuse(input),
            current_position: start_pos,
            loop_backlog: Vec::new(),
        }
    }
    #[inline]
    fn new(input: I) -> Self {
        Self::starting_at(input, Position::default())
    }
    fn next_instruction(&mut self) -> Result<Option<BFCommand>> {
        // clippy can't distinguish that there is an early return, and that
        // the iterator is not to be consumed on call.
        #[allow(clippy::while_let_on_iterator)]
        while let Some(next_byte) = self.input.next() {
            let byte = next_byte.map_err(IOParserErr::IO)?;
            let instruction = BFCommand::from_u8(byte);

            if let Some(instr) = instruction {
                // make sure we're matching loops correctly.
                match instr {
                    BFCommand::BeginLoop => self.loop_backlog.push(self.current_position),
                    BFCommand::EndLoop => {
                        if self.loop_backlog.pop().is_none() {
                            return Err(self.error(ParseErrorKind::MissingLB));
                        }
                    }
                    _ => (),
                }
            }

            if byte.is_ascii() {
                self.current_position.advance_char(byte as char)
            } else {
                self.current_position.advance_col()
            }

            if instruction.is_some() {
                return Ok(instruction);
            }
        }

        // on EOF, there should be no dangling loops
        if let Some(lb_pos) = self.loop_backlog.pop() {
            Err(self.error(ParseErrorKind::MissingRB(lb_pos)))
        } else {
            Ok(None)
        }
    }
    #[inline]
    fn error(&self, kind: ParseErrorKind) -> IOParserErr {
        IOParserErr::Parser(ParseError {
            kind,
            position: self.current_position,
        })
    }
}

pub struct BFParserIter<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    parser: BFParser<I>,
    finished: bool,
}

impl<I> Iterator for BFParserIter<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    type Item = Result<BFCommand>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            None
        } else {
            let res = self.parser.next_instruction();
            if matches!(res, Ok(None) | Err(_)) {
                self.finished = true;
            }
            res.transpose()
        }
    }
}

impl<I> std::iter::FusedIterator for BFParserIter<I> where I: Iterator<Item = io::Result<u8>> {}

impl<I> IntoIterator for BFParser<I>
where
    I: Iterator<Item = io::Result<u8>>,
{
    type IntoIter = BFParserIter<I>;
    type Item = <Self::IntoIter as Iterator>::Item;
    fn into_iter(self) -> Self::IntoIter {
        BFParserIter {
            parser: self,
            finished: false,
        }
    }
}
