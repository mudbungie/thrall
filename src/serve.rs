//! **What `thrall run` does**: read this box, and serve every channel it holds
//! until they have all stopped.
//!
//! Three reads and one loop, and the order is the point. **The config first**,
//! because a box with nothing to offer has nothing to do and no reason to dial
//! anything. **Then the channels**, because a box with nothing to dial has
//! nothing to offer them to. Only then the loop, which does not return while
//! any channel is up.
//!
//! **There is no success exit, so none is spelled.** Every answer here is a
//! [`Verdict::failed`]: either this box could not start, or every channel it
//! held has stopped and the sentences that stopped them are the answer. A foot
//! that returned zero would be saying it had finished, and a foot is never
//! finished — it is either serving or stopped.
//!
//! **And it never reconnects** (DESIGN §2). Restart policy belongs to the
//! supervision the operator's machine already has, and inventing one here would
//! be thrall deciding how a box it does not administer runs a program. The
//! sentence it exits with is what that supervisor's log will carry.

use std::path::Path;

use crate::channel::entries;
use crate::channel::material::{ADDRESS, ANCHORS, CHAIN, KEY, REMEDY};
use crate::cli::Verdict;
use crate::{config, exec, run};

/// Serve the box rooted at `root`, saying anything a channel raises while it
/// is still serving through `notice`.
///
/// **The sink is a parameter for the same reason the executor is** (`run`): a
/// serving foot's notices go to this process's stderr, which is an effect and
/// not a value, so the effect is `src/main.rs`'s and every test here reads the
/// notices back instead.
pub fn serve(root: &Path, notice: &run::Notice) -> Verdict {
    let set = match config::read(&config::path(root)) {
        Ok(set) => set,
        Err(reason) => return Verdict::failed(reason),
    };
    let held = entries::read_dir(&entries::dir(root));
    if held.is_empty() {
        return Verdict::failed(unprovisioned(root));
    }
    Verdict::failed(run::fan(held, set, exec::handoff, notice).join("\n"))
}

/// What a box with a tool document and no channel is told.
///
/// It names the shape rather than a command, because there is no command: the
/// material is minted on the box that holds the operator's CA and carried here
/// by hand (REMOTE §1.4). A remedy thrall could run would be a bootstrap flow,
/// and a foot that could provision itself over the wire would be a foot any
/// wire could provision.
fn unprovisioned(root: &Path) -> String {
    format!(
        "this box holds no channel. One channel is one directory under {}, \
         holding {ANCHORS}, {CHAIN}, {KEY} and {ADDRESS} — {REMEDY}",
        entries::dir(root).display()
    )
}

#[cfg(test)]
mod tests;
