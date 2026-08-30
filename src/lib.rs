//! thrall — the **foot**: a tool-execution client for a yog server.
//!
//! A thrall holds an operator-issued, foot-grade certificate and dials in to
//! an engine it does not administer. Its entire wire surface is three acts:
//! advertise what this box offers, wait on its mailbox for work, post the
//! captures back. It asks nothing, it acts on nothing, and it never listens —
//! the engine never speaks first.
//!
//! `docs/DESIGN.md` states the role and the inherited invariants. yog's
//! `docs/REMOTE.md` is the protocol authority thrall implements against; where
//! this crate and that document disagree, one of them is a bug.
//!
//! **What is built** (DESIGN §4): the command line; the [`channel`] — the mTLS
//! dial, the version preface, the framing, the foot-grade check on this box's
//! own leaf, and the entries an operator filed (bl-a4a5); and the [`config`]
//! this box offers from, with [`tools`] the element it advertises and
//! [`json`] the strict reads both spend (bl-05fe); and the [`gestures`] a foot
//! may send with the [`run`] loop that spends them, over the
//! [`invocation`](crate::invocation) nouns the routing leg carries (bl-a2ea).
//! The executor behind the loop's hand-off is the ball after it.

pub mod channel;
pub mod cli;
pub mod config;
pub mod gestures;
pub mod invocation;
pub mod json;
pub mod run;
pub mod tools;

/// **The spawn boundary** — every child process is built and forked here.
/// `cfg(test)` while its only tenant is the suite's stand-in for the operator's
/// certificate mint; bl-4cda drops the attribute when the executor lands.
#[cfg(test)]
pub(crate) mod spawn;

/// Scaffolding the suite shares. Never compiled into a released binary.
#[cfg(test)]
pub(crate) mod test_support;
