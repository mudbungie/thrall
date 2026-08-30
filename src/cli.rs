//! The command line, as a pure function.
//!
//! `run` takes the arguments and hands back a [`Verdict`] — an exit code and
//! the one thing the run has to say. It touches no process state: no argv, no
//! streams, no exit. That is the whole reason `src/main.rs` can be the one
//! file excluded from the coverage floor (`tarpaulin.toml`) without excluding
//! any decision: every decision is here, and every decision is a value a test
//! can read back.

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

usage: thrall [--version | --help]

  -V, --version   print the name and version
  -h, --help      print this

No verb runs yet. What is built is the channel — mTLS, the version preface, the
framing, and the foot-grade check on this box's own leaf — the operator's tool
document, and the loop that presents it, waits for work and posts the captures
back. What is missing is the executor behind the loop's hand-off, and the verb
that starts it. See docs/DESIGN.md for the role and the module map, and yog's
docs/REMOTE.md for the protocol thrall implements against.",
        version()
    )
}

/// Decide what one invocation does. `args` is argv **without** the program
/// name.
pub fn run(args: Vec<String>) -> Verdict {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--version" | "-V"] => Verdict::ok(version()),
        ["--help" | "-h"] => Verdict::ok(usage()),
        [] => Verdict::refused("nothing to do — no verb starts the loop yet".to_string()),
        other => Verdict::refused(format!("unrecognised argument: {}", other.join(" "))),
    }
}

#[cfg(test)]
mod tests {
    use super::{REFUSED, Verdict, run, usage, version};

    /// Build the argument vector the way `main` does, from string literals.
    fn argv(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| (*w).to_string()).collect()
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

    #[test]
    fn both_version_spellings_print_the_version_and_succeed() {
        for spelling in ["--version", "-V"] {
            let v = run(argv(&[spelling]));
            assert_eq!(v.code, 0, "{spelling} did not succeed");
            assert_eq!(v.text, version(), "{spelling} printed something else");
        }
    }

    #[test]
    fn both_help_spellings_print_the_usage_and_succeed() {
        for spelling in ["--help", "-h"] {
            let v = run(argv(&[spelling]));
            assert_eq!(v.code, 0, "{spelling} did not succeed");
            assert_eq!(v.text, usage(), "{spelling} printed something else");
        }
    }

    #[test]
    fn a_bare_invocation_refuses_and_says_why() {
        let v = run(argv(&[]));
        assert_eq!(v.code, REFUSED);
        assert!(v.text.contains("no verb starts the loop yet"), "{}", v.text);
        assert!(
            v.text.contains("usage: thrall"),
            "a refusal must still teach: {}",
            v.text
        );
    }

    #[test]
    fn an_unrecognised_argument_refuses_and_quotes_every_word_of_it() {
        let v = run(argv(&["seat", "--ws", "Example"]));
        assert_eq!(v.code, REFUSED);
        assert!(
            v.text.contains("unrecognised argument: seat --ws Example"),
            "the refusal did not name what it refused: {}",
            v.text
        );
        assert!(v.text.contains("usage: thrall"), "{}", v.text);
    }

    #[test]
    fn a_recognised_flag_with_extra_words_is_not_recognised() {
        // The match is on the WHOLE argument list, not on a first word, so a
        // flag that would succeed alone refuses when something rides behind
        // it — rather than silently ignoring the rest.
        let v = run(argv(&["--version", "--now"]));
        assert_eq!(v.code, REFUSED);
        assert!(
            v.text.contains("unrecognised argument: --version --now"),
            "{}",
            v.text
        );
    }
}
