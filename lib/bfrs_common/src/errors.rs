use super::Position;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ErrorWithPosition<K> {
    pub kind: K,
    pub position: Position,
}

impl<K> Error for ErrorWithPosition<K>
where
    K: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

impl<K> fmt::Display for ErrorWithPosition<K>
where
    K: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.position, self.kind)
    }
}
