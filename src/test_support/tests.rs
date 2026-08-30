//! The scaffolding's own guarantees, since everything else leans on them.

use super::{Scratch, fork_lock};

/// Two scratch directories are two directories, and each disappears with the
/// value that made it.
#[test]
fn a_scratch_is_unique_and_is_removed_when_it_drops() {
    let first = Scratch::new();
    let second = Scratch::new();
    assert_ne!(first.path(), second.path());
    assert!(first.path().is_dir());
    let path = first.join("inside");
    std::fs::write(&path, b"something").expect("write");
    let held = first.path().to_path_buf();
    drop(first);
    assert!(!held.exists(), "the scratch outlived its value");
    assert!(second.path().is_dir(), "and took nothing else with it");
}

/// The fork lock is takeable, and a poisoned one is still takeable — a
/// panicking test must not become a suite that cannot fork.
#[test]
fn the_fork_lock_survives_a_poisoning() {
    let poison = std::thread::spawn(|| {
        let _held = fork_lock();
        panic!("a test that died holding it");
    });
    assert!(poison.join().is_err(), "the thread panicked");
    let _taken = fork_lock();
}
