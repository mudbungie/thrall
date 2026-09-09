//! **What the vendored ledger has to be true of**, and the two facts this
//! build states on the wire out of it.

use super::{DEPRECATED, FLOOR, SHAPES, edition, path_of, paths_of_type, signature};
use crate::channel::hello;

/// **The edition this build states is the edition it vendored.**
/// [`hello::EDITION`] is what goes on the wire; [`edition`] is computed from
/// the ledger copied out of yog. Two sources, held equal here — the same
/// discipline the protocol pin keeps, and for the same reason: a constant that
/// wrote both ends of the comparison would agree at any value (bl-e0f0).
#[test]
fn the_edition_this_build_states_is_the_newest_stamp_it_vendored() {
    assert_eq!(
        hello::EDITION,
        edition(),
        "this foot states edition {}, its vendored ledger's newest stamp is {} — \
         re-vendor src/corpus/ledger/shapes.rs and move the constant with it",
        hello::EDITION,
        edition(),
    );
}

/// The floor is the engine's fact too, so it is stated twice for the same
/// reason and held equal here.
#[test]
fn the_floor_this_build_defaults_to_is_the_floor_it_vendored() {
    assert_eq!(hello::FLOOR, FLOOR);
}

/// **Every path this foot reads is at or below the floor**, which is why every
/// reader here may require its fields — and this is the beat that says so out
/// loud rather than leaving it a coincidence.
///
/// The day a re-vendor brings a foot-facing path stamped above the floor, this
/// goes red first, and what it asks for is REMOTE §3.2 rule 2: that reader
/// gives the key a default, and the default is the fact before the field
/// existed.
#[test]
fn no_path_a_foot_reads_is_stamped_above_the_floor() {
    let above: Vec<&str> = SHAPES
        .iter()
        .flat_map(|shape| shape.signature.iter())
        .filter(|&&(_, since)| since > FLOOR)
        .map(|&(entry, _)| entry)
        .collect();
    assert!(
        above.is_empty(),
        "post-floor paths a reader must now default rather than require: {above:?}"
    );
}

/// **Nothing this foot reads is on yog's deprecation list.** The list is empty
/// today; the window rule (§3.2) is that a path is listed in a published
/// release and removed no sooner than three releases later, so a name arriving
/// here is a prompt with time on it rather than a break.
#[test]
fn nothing_this_foot_reads_has_been_listed_for_removal() {
    let going: Vec<&str> = SHAPES
        .iter()
        .flat_map(|shape| shape.signature.iter())
        .map(|&(entry, _)| entry)
        .filter(|entry| DEPRECATED.contains(entry))
        .collect();
    assert!(
        going.is_empty(),
        "deprecated paths this foot reads: {going:?}"
    );
}

/// The seven shapes are the three gestures and the four answers, by name.
#[test]
fn the_ledger_holds_the_seven_shapes_a_foot_speaks() {
    let names: Vec<&str> = SHAPES.iter().map(|shape| shape.name).collect();
    assert_eq!(
        names,
        [
            "request/advertise",
            "request/complete",
            "request/invocations",
            "reply/advertised",
            "reply/invocations",
            "reply/refusal",
            "reply/routed",
        ]
    );
    assert!(
        signature("reply/follow").is_none(),
        "a seat's shape is a parity fact, not a vendored one"
    );
}

/// A path is read off the entry by cutting the type, and the type is read off
/// the same entry by matching it — one spelling, two questions.
#[test]
fn an_entry_carries_its_path_and_its_type() {
    let queued = signature("reply/invocations").expect("a shape this foot speaks");
    assert_eq!(path_of("/rows/[]/tool:string"), "/rows/[]/tool");
    assert_eq!(path_of(":object"), "");
    assert!(paths_of_type(queued, "string").contains(&"/rows/[]/tool"));
    assert!(!paths_of_type(queued, "string").contains(&"/rows"));
}
