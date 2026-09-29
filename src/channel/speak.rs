//! **What a roving channel says of its held line** (bl-3958; DESIGN §3.11):
//! the sink the loop hands down, and the two things only the channel sees —
//! a held line lost, and a ping discarded off it. The lines themselves are
//! `rendezvous::say`'s, where the one rule they share is kept.

use super::{Channel, Failure};
use crate::rendezvous::say::{self, Kind, Say};
use crate::run::Notice;

impl Channel {
    /// **Where the ladder's notices go**: the sink the loop above hands down,
    /// so a channel's rungs are said under its name. An entry that does not
    /// rove climbs one rung and says nothing.
    pub(crate) fn speak_to(&mut self, notice: &Notice) {
        if let Some(roving) = self.roving.as_mut() {
            roving.say = Say::to(notice);
        }
    }

    /// Say `line`, where there is a roving half to say it.
    pub(super) fn say(&mut self, kind: Kind, line: String) {
        if let Some(roving) = self.roving.as_mut() {
            roving.say.say(kind, line);
        }
    }

    /// The wire failed on `leg` — said as a held line lost where the stream
    /// was punched.
    pub(super) fn lost(&mut self, leg: &str, e: &std::io::Error, punched: bool) -> Failure {
        if punched {
            self.say(Kind::Held, say::held_dropped(leg, e));
        }
        Failure::Wire(self.failed(leg, e))
    }
}
