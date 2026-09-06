//! **The one ending a further dial cannot improve** (bl-5d62): the two ends do
//! not speak one protocol version.
//!
//! It is a fourth party, beside the three `ending` tells apart. The wire has no
//! opinion of either binary. The engine refusing and the engine answering
//! something unreadable are both this engine's reading of this box. A version
//! mismatch is neither: it is a fact about the two *binaries*, stated by yog's
//! `docs/REMOTE.md` §3 as having no negotiation at all and carrying its own
//! remedy — *"upgrade the older component"* — so nothing but a new binary on one
//! of the two boxes can change it, and every dial until then buys the same
//! handshake and the same sentence.
//!
//! On the tree before this ball it was classified as the wire: an engine
//! upgraded ahead of its feet was dialled forever, writing that sentence into
//! its operator's journal a minute at a time for the life of the process, while
//! the sentence itself named a remedy the loop then declined to wait for.

use super::super::hold::{Ending, hold};
use super::super::redial::redial;
use super::{advertised, aside, echo, entry_at, set};
use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::test_support::engine::Engine;
use crate::test_support::{Notices, Scratch, Waits, mint};

/// An engine standing one version ahead of this foot, answering `dials` times
/// if anything ever gets past its preface. More than one, so a loop that took
/// the skew for the wire would find something to talk to on its second dial —
/// which is what the tests below assert never happens.
fn ahead(dir: &std::path::Path, dials: usize) -> Engine {
    mint::material(dir);
    Engine::start(
        dir,
        crate::corpus::PROTOCOL + 1,
        vec![vec![advertised()]; dials],
    )
}

/// **The conversation is over, not dropped.** It is met at the first leg,
/// because the preface is confirmed before a frame of any answer is decoded —
/// but the class is the failure's and not the leg's, so it would end the
/// channel wherever it arrived.
#[test]
fn a_version_the_two_ends_do_not_share_is_over_and_not_the_wire() {
    let scratch = Scratch::new();
    let _engine = ahead(scratch.path(), 1);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    let Ending::Over(said) = hold(&channel, &set(), echo, &aside(), None) else {
        panic!("a version this foot cannot speak is not worth another dial");
    };
    assert!(
        said.contains(&format!(
            "the engine speaks {}",
            crate::corpus::PROTOCOL + 1
        )),
        "{said}"
    );
    assert!(said.contains("upgrade the older component"), "{said}");
}

/// **And the lifetime above it stops where it stands**: one sentence, no wait,
/// no second dial. The process then exits naming it, which is the state an
/// operator has to act on anyway — and it is what the README already promises
/// for this case, *"a foot that cannot be a foot at all exits and says why"*.
///
/// The absent redial suffix is also the end of the doubled full stop the ball
/// filed beside this: the refusal ends in `.` and `waiting` appended a second
/// one. Nothing is trimmed — the sentence simply stops going through the
/// wording that assumed another dial was coming.
///
/// **The recorded waits are the proof that only one dial was made**, and not
/// the engine's own record of what it heard. A refused foot drops the stream
/// where it stands, so whether the far end finished reading the two frames it
/// was sent is a race this end has already won and is nothing about the
/// lifetime under test (it flaked exactly there, under tarpaulin). `redial`
/// pauses before every dial after the first, so an empty series is a second
/// dial that never happened.
#[test]
fn it_is_neither_waited_out_nor_dialled_again() {
    let scratch = Scratch::new();
    let _engine = ahead(scratch.path(), 2);
    let (waits, pause) = Waits::new();
    let (notices, sink) = Notices::new();
    let said = redial(&entry_at(scratch.path()), &set(), echo, &sink, &pause);
    assert!(said.contains("upgrade the older component"), "{said}");
    assert_eq!(waits.heard(), [], "it waited to dial an engine again");
    assert!(
        !said.contains("Dialling this engine again"),
        "a fact only a new binary can change is not something to wait on: {said}"
    );
    assert!(
        !said.contains(".."),
        "the redial suffix used to append a second full stop: {said}"
    );
    assert!(notices.heard().is_empty(), "nothing said in flight");
}
