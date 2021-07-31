#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BFCommand {
    BeginLoop = b'[',
    EndLoop = b']',
    Print = b'.',
    Read = b',',
    Increment = b'+',
    Decrement = b'-',
    Right = b'>',
    Left = b'<',
}

impl BFCommand {
    pub fn from_u8(byte: u8) -> Option<Self> {
        Some(match byte {
            b'[' => Self::BeginLoop,
            b']' => Self::EndLoop,
            b'.' => Self::Print,
            b',' => Self::Read,
            b'+' => Self::Increment,
            b'-' => Self::Decrement,
            b'>' => Self::Right,
            b'<' => Self::Left,
            _ => return None,
        })
    }
}

use std::fmt;
impl fmt::Display for BFCommand {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", *self as u8 as char)
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub column: usize,
    pub line: usize,
}

impl Default for Position {
    fn default() -> Self {
        Self { column: 1, line: 1 }
    }
}

impl Position {
    #[inline]
    pub fn advance_char(&mut self, ch: char) {
        if ch == '\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
    }

    #[inline]
    pub fn advance_col(&mut self) {
        self.column += 1;
    }
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}
