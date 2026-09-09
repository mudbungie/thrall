//! **The two replays yog's `docs/REMOTE.md` §3.2 asks a consumer for**, and the
//! fixture that proves the first of them is alive.

use serde_json::{Value, json};

use super::{REPLIES, editions, project, reword};
use crate::corpus::ledger::{self, FLOOR};
use crate::corpus::{ADVERTISED, QUEUED, REFUSAL};
use crate::gestures::{self, Reply};

/// A token no build has heard of — a word, in a place a word can be.
const UNHEARD: &str = "fhtagn";

/// One corpus frame as a value.
fn frame(text: &str) -> Value {
    serde_json::from_str(text).expect("a corpus frame is JSON")
}

/// **The projection replay.** Every reply shape, at every edition an engine of
/// this major can be at, with every key stamped above that edition deleted:
/// nothing refuses.
#[test]
fn every_reply_shape_decodes_at_every_edition_an_engine_can_be_at() {
    let mut replayed = 0;
    for (name, frames) in REPLIES {
        let signature = ledger::signature(name).expect("a shape this foot speaks");
        for edition in editions() {
            for text in frames {
                let projected = project(&frame(text), signature, edition);
                assert!(
                    gestures::decode(&projected).is_ok(),
                    "{name} at edition {edition} refused {projected}"
                );
                replayed += 1;
            }
        }
    }
    assert!(replayed > 0, "the replay walked no frame at all");
}

/// **The editions are a range and the floor is in it.** An empty range would
/// make the replay above a loop that never runs — green forever, and about
/// nothing.
#[test]
fn the_replay_always_has_at_least_the_floor_to_replay_at() {
    let editions = editions();
    assert_eq!(editions.first(), Some(&FLOOR));
    assert_eq!(editions.last(), Some(&FLOOR.max(ledger::edition())));
}

/// **The projector bites, and here is the red it will give.** No path this foot
/// reads is stamped above the floor yet, so the replay above deletes nothing
/// and could pass on a projector that did nothing at all. This is the other
/// direction: a FIXTURE signature in which `wrote` arrived after the floor —
/// the shape of the first post-floor field a foot shape gains — projected at
/// the floor, which is an engine too old to write it.
///
/// The key goes, and the decoder refuses, because `wrote` is required
/// (`gestures::Reply::Advertised`). That refusal is the whole point: the day
/// yog stamps a foot-facing path above the floor, this replay goes red until
/// the reader gives it the default §3.2 rule 2 asks for.
#[test]
fn a_key_stamped_above_the_floor_is_deleted_and_the_strict_reader_says_so() {
    const AFTER_THE_FLOOR: [(&str, u32); 4] = [
        ("/kind:string", 8),
        ("/ok:bool", 8),
        ("/wrote:bool", 19),
        (":object", 8),
    ];
    let projected = project(&frame(ADVERTISED[1]), &AFTER_THE_FLOOR, FLOOR);
    assert_eq!(projected, json!({"kind": "advertised", "ok": true}));
    assert_eq!(
        gestures::decode(&projected),
        Err("reply: missing field \"wrote\"".to_owned())
    );
}

/// **The word-mutation replay.** Every string-typed path the ledger stamps,
/// other than `kind`, carrying a token no build has heard of: nothing refuses.
///
/// `kind` is the exception §3.2 rule 4 names — a reader asks only what it
/// paints, so an unknown kind means an ask started being answered differently,
/// which is a major by definition.
#[test]
fn an_unheard_of_word_refuses_nothing_but_the_kind() {
    let mut reworded = 0;
    for (name, frames) in REPLIES {
        let signature = ledger::signature(name).expect("a shape this foot speaks");
        for path in ledger::paths_of_type(signature, "string") {
            if path == "/kind" {
                continue;
            }
            let Some(mutated) = frames
                .iter()
                .find_map(|text| reword(&frame(text), path, UNHEARD))
            else {
                panic!("{name}: the ledger stamps {path} and no vendored frame carries it");
            };
            assert!(
                gestures::decode(&mutated).is_ok(),
                "{name} refused an unheard-of word at {path}: {mutated}"
            );
            reworded += 1;
        }
    }
    assert!(reworded > 0, "the ledger stamps no word-shaped path");
}

/// **The `kind` this foot has no use for is still refused by name**, which is
/// the strictness rule 4 keeps: the mutation replay skips `kind` because
/// mutating it is supposed to fail.
#[test]
fn the_kind_stays_strict_under_the_same_mutation() {
    let mutated = reword(&frame(ADVERTISED[0]), "/kind", UNHEARD).expect("a kind to reword");
    assert_eq!(
        gestures::decode(&mutated),
        Err(format!("reply: unusable kind {UNHEARD:?}"))
    );
}

/// **A path no frame carries is answered `None`, not silently skipped.** That
/// is what lets the mutation replay above report a stale vendor: the refusal
/// shape has no `kind` at all, which is the one shape a refusal may wear.
#[test]
fn a_path_the_frame_does_not_carry_is_not_a_mutation() {
    assert_eq!(reword(&frame(REFUSAL), "/kind", UNHEARD), None);
}

/// **A post-floor key inside a row is deleted in every row, and a reader that
/// defaults it goes on** — REMOTE §3.2 rule 2 in the one place this crate
/// already keeps it. `cwd` is optional and its absence means *the subject's own
/// location*, which is exactly "the fact before the field existed"; so a
/// fixture signature stamping it after the floor projects to the frame an older
/// engine would have written, and the decoder answers `None` rather than
/// refusing. It is also the walk this replay would otherwise never take: the
/// path descends through an object into an array and deletes in each element.
#[test]
fn a_post_floor_key_on_a_row_is_deleted_and_a_defaulting_reader_goes_on() {
    const AFTER_THE_FLOOR: [(&str, u32); 4] = [
        ("/kind:string", 2),
        ("/ok:bool", 2),
        ("/rows/[]/cwd:string", 19),
        ("/rows/[]/tool:string", 2),
    ];
    let projected = project(&frame(QUEUED[2]), &AFTER_THE_FLOOR, FLOOR);
    assert_eq!(projected["rows"][0].get("cwd"), None);
    let Ok(Ok(Reply::Invocations(rows))) = gestures::decode(&projected) else {
        panic!("a row an older engine wrote still decodes");
    };
    assert_eq!(rows[0].cwd, None);
}

/// **A path the frame's shape does not admit changes nothing**, which is what
/// the mutation replay's report of a stale vendor rests on: three ways for a
/// walk to run out of frame — the frame itself, an array step where there is no
/// array, and an object step where there is no object.
#[test]
fn a_path_the_frames_shape_does_not_admit_changes_nothing() {
    for path in ["", "/kind/[]", "/rows/[]/tool/deeper"] {
        assert_eq!(reword(&frame(QUEUED[1]), path, UNHEARD), None, "{path}");
    }
}
