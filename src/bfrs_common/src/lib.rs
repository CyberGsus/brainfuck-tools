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
