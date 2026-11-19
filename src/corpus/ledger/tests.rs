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

/// The floor is the server's fact too, so it is stated twice for the same
/// reason and held equal here.
#[test]
fn the_floor_this_build_defaults_to_is_the_floor_it_vendored() {
    assert_eq!(hello::FLOOR, FLOOR);
}

/// **The first addition past the floor on a foot's shapes is the streamed
/// form** (REMOTE §5.7): `partial`, on the post and on its receipt, and the
/// edition this foot waits for before it writes one is that stamp.
///
/// It is pinned as a list rather than as *"some paths"* because each new
/// member is a decision: a post-floor key on a reply must be defaulted by its
/// reader, and one on a request must be gated on the server's edition. A
/// re-vendor that adds an eighth fails here and names it.
///
/// **The seventh is `superseded`, on the read's answer** (REMOTE §5.1, edition
/// 29) — a reply key, so its reader defaults it: absent is an ordinary hold
/// (`gestures::decode`).
#[test]
fn the_only_paths_stamped_above_the_floor_are_the_streamed_form_s_and_superseded() {
    let above: Vec<(&str, &str, u32)> = SHAPES
        .iter()
        .flat_map(|shape| {
            shape
                .signature
                .iter()
                .map(|&(entry, since)| (shape.name, entry, since))
        })
        .filter(|&(_, _, since)| since > FLOOR)
        .collect();
    let streamed = |shape| {
        [
            (shape, "/partial/stderr:string", hello::STREAMS),
            (shape, "/partial/stdout:string", hello::STREAMS),
            (shape, "/partial:object", hello::STREAMS),
        ]
    };
    let superseded = [("reply/invocations", "/superseded:bool", hello::EDITION)];
    assert_eq!(
        above,
        [
            streamed("request/complete").as_slice(),
            &superseded,
            &streamed("reply/routed"),
        ]
        .concat()
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
