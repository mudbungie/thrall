//! **The child's three pipes, pumped without blocking** (bl-6c14; DESIGN
//! §3.5).
//!
//! **A pipe outlives the process that was given it.** A read ends when *every*
//! writer has closed, and a tool that backgrounds a helper hands that helper
//! the same stdout and stderr write ends it holds itself — so a drain that
//! waits for the pipe waits for a stranger this box never started and cannot
//! bound. The executor's deadline governs the process thrall forked; before
//! this file, nothing at all governed the read.
//!
//! **So the drain asks what is there rather than waiting for the end.** Every
//! descriptor is put into non-blocking mode (`sys::nonblocking`) and read until
//! the kernel says there is nothing more *right now*. Once the tool has exited,
//! everything it wrote is already in the pipe — `write(2)` completed before
//! `exit(2)` did — so one such pass is the whole of what the capture is owed,
//! and a write end a helper still holds adds nothing to the invocation but
//! bytes nobody asked for.
//!
//! **It replaced three threads with one loop, and that is the point.** The
//! drains and the input feed used to be threads precisely because each of them
//! could block; none of them can now, so the executor's poll — the one that was
//! already watching the child — pumps all three. A capture is bounded by the
//! deadline because there is nothing left that is not inside that loop.
//!
//! **Nothing here changes what the child sees.** `pipe(2)` gives the two ends
//! separate file descriptions, so `O_NONBLOCK` on this end is invisible on the
//! other: the tool's own writes block and succeed exactly as they always did.

use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::process::Child;
use std::time::Instant;

/// How much is moved per read. A pipe holds 64 KiB on Linux, so this empties a
/// full one in a handful of calls and costs one page when it is empty.
const CHUNK: usize = 16 * 1024;

/// One pipe being read, and what it has yielded so far. `None` is a pipe that
/// is finished — every writer closed, or it stopped being readable — and a
/// finished pipe is never asked again.
struct Sink {
    pipe: Option<Box<dyn Read>>,
    bytes: Vec<u8>,
}

impl Sink {
    /// Read everything available right now. `true` when something moved, which
    /// is what tells the loop above not to sleep yet.
    fn pump(&mut self) -> bool {
        let mut moved = false;
        let mut buf = [0u8; CHUNK];
        while let Some(pipe) = self.pipe.as_mut() {
            match pipe.read(&mut buf) {
                Ok(n) if n > 0 => {
                    self.bytes
                        .extend_from_slice(buf.get(..n).unwrap_or_default());
                    moved = true;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                // End of file, or a pipe that cannot be read at all: one arm,
                // because both are the same fact — there is nothing more here.
                _ => self.pipe = None,
            }
        }
        moved
    }
}

/// A child's stdin, stdout and stderr, pumped from the caller's own loop.
pub(crate) struct Pipes {
    stdin: Option<std::process::ChildStdin>,
    payload: Vec<u8>,
    written: usize,
    out: Sink,
    err: Sink,
}

impl Pipes {
    /// Take the child's three pipes and make every one of them non-blocking.
    ///
    /// `payload` is the invocation's input, which goes down stdin as the loop
    /// runs; the pipe is **closed** the moment it is all written, because a
    /// tool that reads to end of file needs that close to be its end of file.
    pub(crate) fn new(child: &mut Child, payload: Vec<u8>) -> Self {
        Self {
            stdin: unblocked(child.stdin.take()),
            payload,
            written: 0,
            out: Sink {
                pipe: reading(child.stdout.take()),
                bytes: Vec::new(),
            },
            err: Sink {
                pipe: reading(child.stderr.take()),
                bytes: Vec::new(),
            },
        }
    }

    /// One pass over all three. `true` when anything moved in either direction.
    pub(crate) fn pump(&mut self) -> bool {
        let fed = self.feed();
        let out = self.out.pump();
        let err = self.err.pump();
        fed || out || err
    }

    /// **The last read a capture is owed**, spent once the tool is gone.
    ///
    /// It keeps pumping while bytes keep arriving, and `until` is the wall on
    /// the one case where they never stop: a helper writing into the same pipe
    /// as fast as this can read it. The tool's own output is in the pipe
    /// already and comes back in the first pass; the wall is there so a
    /// stranger cannot extend an invocation that has ended.
    pub(crate) fn settle(&mut self, until: Instant) {
        while self.pump() && Instant::now() < until {}
    }

    /// What the two pipes held: stdout, then stderr.
    pub(crate) fn take(self) -> (Vec<u8>, Vec<u8>) {
        (self.out.bytes, self.err.bytes)
    }

    /// Push what is left of the input, and close stdin when there is no more.
    fn feed(&mut self) -> bool {
        let Some(pipe) = self.stdin.as_mut() else {
            return false;
        };
        let mut moved = false;
        loop {
            let rest = self.payload.get(self.written..).unwrap_or_default();
            if rest.is_empty() {
                break;
            }
            match pipe.write(rest) {
                Ok(n) if n > 0 => {
                    self.written += n;
                    moved = true;
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => return moved,
                // Written out, or a tool that closed its input and will never
                // read the rest: either way this end has nothing left to do.
                _ => break,
            }
        }
        self.stdin = None;
        moved
    }
}

/// A child pipe, made non-blocking in place.
fn unblocked<T: AsRawFd>(pipe: Option<T>) -> Option<T> {
    if let Some(pipe) = pipe.as_ref() {
        crate::sys::nonblocking(pipe.as_raw_fd());
    }
    pipe
}

/// The same, boxed as the one reader shape a [`Sink`] holds — so stdout and
/// stderr, which are two types, are one thing to the loop.
fn reading<T: AsRawFd + Read + 'static>(pipe: Option<T>) -> Option<Box<dyn Read>> {
    unblocked(pipe).map(|pipe| Box::new(pipe) as Box<dyn Read>)
}
