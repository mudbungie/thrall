//! The command line, as a pure function.
//!
//! `run` takes the arguments and hands back a [`Decided`] — either a
//! [`Verdict`] to say, or the one thing this binary does that cannot be a
//! sentence. It touches no process state: no argv, no environment, no streams,
//! no exit. That is the whole reason `src/main.rs` can be the one file excluded
//! from the coverage floor (`tarpaulin.toml`) without excluding any decision:
//! every decision is here, and every decision is a value a test can read back.
//!
//! **Serving is an outcome and not a `Verdict`, and that is what keeps this
//! file pure.** A verdict is text and an exit code; serving is dialling
//! engines, spawning children and blocking until every channel has stopped. So
//! the decision *"this argv means serve"* is made here and tested here, and the
//! doing of it is the entry point's — which is what that file is excluded for.

/// What one invocation decided: an exit code and the text that explains it.
///
/// The text goes to stdout on success and to stderr otherwise — the split is
/// the caller's, because the code already says which one it is and storing the
/// stream beside it would be the same fact twice.
pub struct Verdict {
    /// The process exit code. `0` is the only success.
    pub code: u8,
    /// Everything this run has to say, without a trailing newline.
    pub text: String,
}

/// The exit code for every refusal. One code, because thrall's refusals are
/// all the same kind of event — "that is not something this binary does" — and
/// a taxonomy of exit codes would be a promise to keep them stable.
const REFUSED: u8 = 2;

/// The exit code for a run that was understood and did not finish: no config,
/// no channel, or every channel stopped. One code for the same reason.
const FAILED: u8 = 1;

impl Verdict {
    /// A successful run and what it printed.
    pub fn ok(text: String) -> Self {
        Self { code: 0, text }
    }

    /// A refusal, from the sentence naming what was refused.
    ///
    /// The prefix and the usage are appended HERE rather than at each call
    /// site, so "a refusal always says what it refused *and* what the caller
    /// could have typed instead" is structural rather than remembered: a
    /// refusal added later cannot forget it. A bare non-zero exit teaches
    /// nobody anything.
    pub fn refused(what: String) -> Self {
        Self {
            code: REFUSED,
            text: format!("thrall: {what}\n\n{}", usage()),
        }
    }

    /// A run that did what it was asked and could not finish it.
    ///
    /// It carries **no usage**, and that is the difference from a refusal: a
    /// refusal is about what the caller typed, so the alternatives are the
    /// useful thing to say next; a failure is about this box or the far end,
    /// where a usage line is noise in front of the sentence that matters.
    pub fn failed(what: String) -> Self {
        Self {
            code: FAILED,
            text: format!("thrall: {what}"),
        }
    }
}

/// What one invocation decided to do.
pub enum Decided {
    /// Say this, and exit. Every flag and every refusal is one of these.
    Say(Verdict),
    /// Serve every channel this box holds, until they have all stopped. The
    /// entry point performs it, because it is the only thing here that is not
    /// a value.
    Serve,
}

/// The crate's name and version, as the `--version` line.
pub fn version() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// The usage text. It states what thrall is before it states what to type,
/// because a foot arrives on a machine whose operator did not necessarily
/// choose to install it.
pub fn usage() -> String {
    format!(
        "{}

thrall is the foot: a tool-execution client for a yog server. It dials in,
advertises what this box offers, waits for work, and posts the captures back.
It never listens and it never speaks first.

usage: thrall run
       thrall [--version | --help]

  run             serve every channel this box is provisioned for: present
                  what it offers, wait for work, run it, post the captures
                  back. It does not return while a channel is up. A channel
                  that drops is dialled again, with a backoff that settles;
                  a channel that cannot be served at all is an exit naming
                  it, and restarting the process belongs to this machine's
                  own supervision.
  -V, --version   print the name and version
  -h, --help      print this

What it reads, both provisioned by hand and never by thrall: the tool document
at <data root>/tools.json, and one channel per directory under <data
root>/wire/workspaces/. The data root is $XDG_DATA_HOME/thrall, or
$HOME/.local/share/thrall.

See docs/DESIGN.md for the role and the module map, and yog's docs/REMOTE.md
for the protocol thrall implements against.",
        version()
    )
}

/// Decide what one invocation does. `args` is argv **without** the program
/// name.
pub fn run(args: Vec<String>) -> Decided {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["run"] => Decided::Serve,
        ["--version" | "-V"] => Decided::Say(Verdict::ok(version())),
        ["--help" | "-h"] => Decided::Say(Verdict::ok(usage())),
        [] => Decided::Say(Verdict::refused(
            "nothing to do — `thrall run` is the verb".to_string(),
        )),
        other => Decided::Say(Verdict::refused(format!(
            "unrecognised argument: {}",
            other.join(" ")
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::{Decided, FAILED, REFUSED, Verdict, run, usage, version};

    /// Build the argument vector the way `main` does, from string literals.
    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
    }

    /// What a run said, for the arguments that decide to say something.
    fn said(words: &[&str]) -> Verdict {
        match run(argv(words)) {
            Decided::Say(verdict) => verdict,
            Decided::Serve => panic!("{words:?} decided to serve"),
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

    /// **The one verb decides to serve, and says nothing.** Serving is not a
    /// sentence, so it is not a verdict — which is what keeps this file a pure
    /// function and the entry point a performer.
    #[test]
    fn the_verb_decides_to_serve() {
        assert!(matches!(run(argv(&["run"])), Decided::Serve));
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
}
