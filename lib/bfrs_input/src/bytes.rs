//! Common input methods to obtain iterators of characters.

use std::io;

// NOTE: not for the scope of this project but could
// be a good idea to make it allocator generic as well
// inside an OS or RT application.
/// Allows getting byte by byte from an input,
/// using a buffer to minimize IO calls while
/// maintaining a low memory cost. Allocates
/// only at `new` or `with_capacity`. Will **never**
/// try to extend itself.
pub struct BufferedBytes<R>
where
    R: io::Read,
{
    buffer: Buffered<R>,
    eof: bool,
}

impl<R> BufferedBytes<R>
where
    R: io::Read,
{
    /// Allocates a buffer with a specified capacity
    pub fn with_capacity(cap: usize, reader: R) -> Self {
        Self {
            buffer: Buffered::with_capacity(cap, reader),
            eof: false,
        }
    }
    // Allocates a buffer of 1 Kb (1/4 page)
    pub fn new(reader: R) -> Self {
        Self::with_capacity(1024, reader)
    }
}

impl<R> std::iter::FusedIterator for BufferedBytes<R> where R: io::Read {}

impl<R> Iterator for BufferedBytes<R>
where
    R: io::Read,
{
    type Item = io::Result<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.eof {
            None
        } else {
            let res = self.buffer.next_byte();
            if matches!(res, Ok(None) | Err(_)) {
                self.eof = true;
            }
            res.transpose()
        }
    }
}

/// A structure with a buffer to
/// obtain byte-by-byte. Won't implement
/// [`io::Read`] nor [`io::BufRead`] as its
/// purpose is not to be a generic reader.
struct Buffered<R>
where
    R: io::Read,
{
    buffer: Vec<u8>,
    reader: R,
}

impl<R> Buffered<R>
where
    R: io::Read,
{
    /// Allocates a buffer with a specified capacity
    pub fn with_capacity(cap: usize, reader: R) -> Self {
        Self {
            buffer: Vec::with_capacity(cap),
            reader,
        }
    }

    pub fn next_byte(&mut self) -> io::Result<Option<u8>> {
        match self.buffer.pop() {
            Some(byte) => Ok(Some(byte)),
            None => {
                if self.read_buffer()? {
                    self.next_byte()
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn read_buffer(&mut self) -> io::Result<bool> {
        // set the buffer length as full capacity.
        unsafe { self.buffer.set_len(self.buffer.capacity()) };
        // read from the reader
        let read_len = self.reader.read(self.buffer.as_mut_slice())?;
        // set the buffer length to what was read
        // SAFETY: the buffer length is initially set to its capacity to let
        // the reader write correctly. The reader returns how many bytes did
        // it read (initialize) into the buffer, so the length of the buffer
        // upon exiting the function is always set correctly to what is currently
        // initialized.
        unsafe { self.buffer.set_len(read_len) };
        self.buffer.reverse();
        Ok(read_len > 0)
    }
}
