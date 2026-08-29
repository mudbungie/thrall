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
//! This is the founding skeleton (bl-349f): a version-capable binary and
//! nothing else. No wire code exists yet.

pub mod cli;
