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
//! [`json`] the strict reads both spend (bl-05fe); the [`gestures`] a foot may
//! send with the [`run`] loop that spends them, over the
//! [`invocation`](crate::invocation) nouns the routing leg carries (bl-a2ea);
//! and the [`exec`]utor behind the loop's hand-off, with the [`serve`] read
//! that starts the whole thing from a data root [`paths`] names (bl-4cda).
//!
//! **thrall is complete as a foot at that point** — advertise, wait, execute,
//! answer — and what remains is what it deliberately is not: it holds no world,
//! it never listens, it never asks and it never acts.

pub mod channel;
pub mod cli;
pub mod config;
pub mod exec;
pub mod gestures;
pub mod invocation;
pub mod json;
pub mod paths;
pub mod run;
pub mod serve;
pub mod tools;

/// **The spawn boundary** — every child process is built and forked here.
pub(crate) mod spawn;

/// **The confined `unsafe` file** — every raw process effect in the crate.
pub(crate) mod sys;

/// Scaffolding the suite shares. Never compiled into a released binary.
#[cfg(test)]
pub(crate) mod test_support;
