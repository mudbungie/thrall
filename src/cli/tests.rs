//! **The command line, read back as values** (split from `cli.rs` by bl-e104,
//! at the cap): every argv this binary understands, and every refusal it
//! makes, asserted as the `Decided` it produces.
//!
//! It is a sibling file rather than an inline module for the ordinary reason —
//! the 300-line cap counts inline tests — and the seam is the one the file's
//! own comment already draws: `cli.rs` is the decision, this is the reading
//! back of it.

use super::{Decided, FAILED, REFUSED, Verdict, run, usage, version};

/// Build the argument vector the way `main` does, from string literals.
fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_string()).collect()
}

/// What a run said, for the arguments that decide to say something.
fn said(words: &[&str]) -> Verdict {
    match run(argv(words)) {
        Decided::Say(verdict) => verdict,
        _ => panic!("{words:?} decided to do something rather than say something"),
    }
}

#[test]
fn version_names_the_crate_and_its_version() {
    assert_eq!(version(), format!("thrall {}", env!("CARGO_PKG_VERSION")));
}

#[test]
fn usage_leads_with_the_version_line_then_says_what_thrall_is() {
    let text = usage();
    assert!(
        text.starts_with(&version()),
        "usage did not lead with the version: {text}"
    );
    assert!(
        text.contains("thrall is the foot"),
        "usage did not say what thrall is"
    );
    assert!(
        text.contains("never speaks first"),
        "usage dropped the dial-in invariant"
    );
}

/// The usage names the verb, and names the two files an operator has to
/// put on the box by hand — because a foot arrives on a machine whose
/// operator did not necessarily choose to install it.
#[test]
fn usage_names_the_verb_and_what_it_reads() {
    let text = usage();
    assert!(text.contains("thrall run"), "{text}");
    assert!(text.contains("tools.json"), "{text}");
    assert!(text.contains("wire/workspaces/"), "{text}");
    assert!(text.contains("dialled again"), "{text}");
}

#[test]
fn ok_carries_the_success_code() {
    let v = Verdict::ok("said".to_string());
    assert_eq!(v.code, 0);
    assert_eq!(v.text, "said");
}

#[test]
fn every_refusal_names_what_it_refused_and_still_teaches() {
    // The prefix and the usage are the constructor's, not the call
    // site's — so this holds for a refusal nobody has written yet.
    let v = Verdict::refused("that is not a verb".to_string());
    assert_eq!(v.code, REFUSED);
    assert_eq!(v.text, format!("thrall: that is not a verb\n\n{}", usage()));
}

/// **A failure carries no usage**, and that is the difference: a refusal is
/// about what the caller typed, a failure is about this box or the far end,
/// where a usage line is noise in front of the sentence that matters.
#[test]
fn a_failure_says_only_what_happened() {
    let v = Verdict::failed("this box holds no channel".to_string());
    assert_eq!(v.code, FAILED);
    assert_eq!(v.text, "thrall: this box holds no channel");
    assert!(!v.text.contains("usage:"), "{}", v.text);
}

#[test]
fn both_version_spellings_print_the_version_and_succeed() {
    for spelling in ["--version", "-V"] {
        let v = said(&[spelling]);
        assert_eq!(v.code, 0, "{spelling} did not succeed");
        assert_eq!(v.text, version(), "{spelling} printed something else");
    }
}

#[test]
fn both_help_spellings_print_the_usage_and_succeed() {
    for spelling in ["--help", "-h"] {
        let v = said(&[spelling]);
        assert_eq!(v.code, 0, "{spelling} did not succeed");
        assert_eq!(v.text, usage(), "{spelling} printed something else");
    }
}

/// **The example document is a thing to say** (bl-bb7d), which is what lets a
/// box that has only the binary write its own `tools.json`: it is a compiled-in
/// constant, so the decision is a value here and the entry point performs no
/// read. The bytes it prints are held to the shipped file in
/// `config/tests/example.rs` — this asserts the argv surface, not the document.
#[test]
fn the_example_document_is_printed_and_succeeds() {
    let v = said(&["--example-tools"]);
    assert_eq!(v.code, 0);
    assert!(v.text.starts_with('['), "not a JSON document: {}", v.text);
    assert!(
        usage().contains("--example-tools"),
        "the usage no longer names the flag, so nobody finds it"
    );
}

/// **The one verb decides to serve, and says nothing.** Serving is not a
/// sentence, so it is not a verdict — which is what keeps this file a pure
/// function and the entry point a performer.
#[test]
fn the_verb_decides_to_serve() {
    assert!(matches!(run(argv(&["run"])), Decided::Serve));
}

/// **The bridge is a decision and not a sentence**, for serving's reason
/// exactly: it reads this process's stdin and spawns a server, which are
/// effects. What is decided here — and read back here as a value — is the
/// split of one argv into a tool and the server that carries it.
#[test]
fn the_bridge_takes_the_tool_before_the_dashes_and_the_server_after() {
    match run(argv(&["mcp", "fetch", "--", "uvx", "mcp-server-fetch"])) {
        Decided::Bridge { tool, server } => {
            assert_eq!(tool, "fetch");
            assert_eq!(
                server,
                vec!["uvx".to_string(), "mcp-server-fetch".to_string()]
            );
        }
        _ => panic!("mcp did not decide to bridge"),
    }
}

/// **`pin` is the operator's half of the same verb**, and it reads no input —
/// which is why it is a decision of its own rather than a bridge with a
/// reserved tool name: the entry point must know not to wait on a stdin that
/// a human at a terminal is never going to write.
#[test]
fn pin_is_the_operator_verb_and_carries_only_the_server() {
    match run(argv(&["mcp", "pin", "--", "uvx", "mcp-server-fetch"])) {
        Decided::Pin { server } => {
            assert_eq!(
                server,
                vec!["uvx".to_string(), "mcp-server-fetch".to_string()]
            );
        }
        _ => panic!("mcp pin did not decide to pin"),
    }
}

/// And so a server's tool actually CALLED `pin` cannot be bridged under that
/// name — the usage says so, and renaming it is an edit the paste already
/// invites (DESIGN §6.3: a prefix is the operator's edit).
#[test]
fn a_tool_named_pin_is_the_operator_verb_and_the_usage_says_so() {
    assert!(matches!(
        run(argv(&["mcp", "pin", "--", "a-server"])),
        Decided::Pin { .. }
    ));
    assert!(
        usage().contains("`pin` is therefore a tool"),
        "the usage does not warn that the name is taken: {}",
        usage()
    );
}

/// **Every incomplete spelling of it refuses, and teaches the whole
/// shape.** A bridge with no server to spawn is the one an operator writes
/// into `tools.json` and never sees fail until a model calls it.
#[test]
fn an_mcp_without_a_tool_and_a_server_argv_refuses_and_shows_the_shape() {
    for words in [
        vec!["mcp"],
        vec!["mcp", "fetch"],
        vec!["mcp", "fetch", "--"],
        vec!["mcp", "--", "uvx"],
    ] {
        let v = said(&words);
        assert_eq!(v.code, REFUSED, "{words:?}");
        assert!(
            v.text.contains("thrall mcp fetch -- uvx mcp-server-fetch"),
            "{words:?}: {}",
            v.text
        );
    }
}

#[test]
fn a_bare_invocation_refuses_and_names_the_verb() {
    let v = said(&[]);
    assert_eq!(v.code, REFUSED);
    assert!(v.text.contains("`thrall run` is the verb"), "{}", v.text);
    assert!(
        v.text.contains("usage: thrall"),
        "a refusal must still teach: {}",
        v.text
    );
}

#[test]
fn an_unrecognised_argument_refuses_and_quotes_every_word_of_it() {
    let v = said(&["seat", "--ws", "Example"]);
    assert_eq!(v.code, REFUSED);
    assert!(
        v.text.contains("unrecognised argument: seat --ws Example"),
        "the refusal did not name what it refused: {}",
        v.text
    );
    assert!(v.text.contains("usage: thrall"), "{}", v.text);
}

#[test]
fn a_recognised_word_with_extra_words_is_not_recognised() {
    // The match is on the WHOLE argument list, not on a first word, so a
    // word that would succeed alone refuses when something rides behind
    // it — rather than silently ignoring the rest.
    for extra in [["--version", "--now"], ["run", "--now"]] {
        let v = said(&extra);
        assert_eq!(v.code, REFUSED);
        assert!(
            v.text
                .contains(&format!("unrecognised argument: {}", extra.join(" "))),
            "{}",
            v.text
        );
    }
}
