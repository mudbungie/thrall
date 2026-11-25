//! **The jobs a pushing read has handed this host** (yog's `docs/REMOTE.md`
//! §5.8): each on its own thread, each posting on its own connection, and the
//! list of them that the next read states as `holding`.
//!
//! **There is no limit and no backlog.** A job is started the moment its frame
//! is read, whatever else is running; what this host has not started is still
//! on the server's queue, where every seat can read it. That is §5.8's
//! *"start every pushed job at once, on its own thread, with no limit"* and
//! *"hold no backlog"*, and it is the whole reason a pushing host is never
//! busy.
//!
//! **`holding` is computed, never kept.** It is every job whose thread is
//! still running, and a job's thread ends only once its final post has been
//! answered — landed or refused — or the run was dropped on a post that could
//! not be made. So the list a reconnect states is read off the threads
//! themselves, at the moment the read is opened, and there is no second record
//! of what is in flight that could disagree with them. A job that finishes
//! while the read is reconnecting is safe either way round (REMOTE §5.8):
//! listed and finished is ignored by the server, unlisted and finished has
//! nothing to hand out.
//!
//! **No lock** (DESIGN §4's `src/state.rs` row stays unbuilt). The read's own
//! thread is the only one that touches this list; a job hands back nothing but
//! the fact that its thread ended, which `JoinHandle::is_finished` already
//! answers, and the one flag the jobs read — that the channel is over — is a
//! scalar.
//!
//! **What two jobs do to each other is the caller's business.** Two jobs aimed
//! at one working directory write it at once; the server's asker chose both,
//! and this host neither serializes nor refuses them (DESIGN §3.13).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

use super::held;
use super::hold::Ending;
use super::redial::{FIRST, next};
use super::{Handoff, Notice, Pause};
use crate::channel::Channel;
use crate::config::Local;
use crate::invocation::Invocation;

/// Every job this channel has been pushed and is still running, by id.
pub(crate) struct Pushed {
    running: Vec<(String, JoinHandle<()>)>,
    /// Set when the channel is over: a job holding a capture the wire
    /// swallowed stops posting it again, as the serial loop drops one on an
    /// `Over` (`Held::carried_by`).
    over: Arc<AtomicBool>,
    notice: Notice,
    pause: Pause,
}

impl Pushed {
    /// Nothing running yet. A job says what happens to it through `notice`,
    /// and waits through `pause` before posting a swallowed capture again.
    pub(crate) fn new(notice: &Notice, pause: &Pause) -> Self {
        Self {
            running: Vec::new(),
            over: Arc::new(AtomicBool::new(false)),
            notice: Arc::clone(notice),
            pause: Arc::clone(pause),
        }
    }

    /// **What this host holds** (REMOTE §5.8's `holding`): every job whose
    /// final post the server has not answered, in the order they were pushed.
    /// A job that has finished is forgotten here, which is the only place one
    /// leaves the list.
    pub(crate) fn holding(&mut self) -> Vec<String> {
        self.running.retain(|(_, job)| !job.is_finished());
        self.running.iter().map(|(id, _)| id.clone()).collect()
    }

    /// **Start one pushed job, now, on its own thread** — unless it is already
    /// running here, because §5.8 says *"a pushed id already in `holding` is
    /// not started twice"*.
    ///
    /// The job posts on a clone of the channel, so its connection is its own
    /// and never the read's.
    pub(crate) fn start(
        &mut self,
        channel: &Channel,
        set: &[Local],
        handoff: Handoff,
        invocation: Invocation,
    ) {
        if self.holding().contains(&invocation.id) {
            return;
        }
        let id = invocation.id.clone();
        let mut channel = channel.clone();
        let set = set.to_vec();
        let (over, notice, pause) = (
            Arc::clone(&self.over),
            Arc::clone(&self.notice),
            Arc::clone(&self.pause),
        );
        let job = std::thread::spawn(move || {
            job(
                &mut channel,
                &set,
                handoff,
                &invocation,
                &over,
                &notice,
                &pause,
            );
        });
        self.running.push((id, job));
    }

    /// **The channel is over: wait for every job it started.** A process whose
    /// channels have all stopped exits, and one that exited with jobs running
    /// would leave their children unread and their captures unposted. A job
    /// still running finishes and posts; a job holding a swallowed capture
    /// stops trying at its next wait.
    pub(crate) fn settle(self) {
        self.over.store(true, Ordering::SeqCst);
        for (_, job) in self.running {
            let _ = job.join();
        }
    }
}

/// **One pushed job, from the hand-off to the answered post** — the serial
/// loop's own run (`held::run`), on a connection of its own.
///
/// Three ways out, and the thread ending is what takes the id out of
/// `holding` in every one of them:
///
/// - the final post was answered, landed or refused — the ordinary end;
/// - a `partial` post could not be made: the executor stopped the child, the
///   run is dropped and nothing more is posted for it, exactly as the serial
///   loop drops it; this host says so;
/// - **the wire swallowed the final post**: the capture is held, and posted
///   again after the redial's own waits until the server answers it. The id
///   stays in `holding` the whole time, so the server leaves the slot alone
///   (REMOTE §5.6, ruling 1's note for a pushing host).
fn job(
    channel: &mut Channel,
    set: &[Local],
    handoff: Handoff,
    invocation: &Invocation,
    over: &AtomicBool,
    notice: &Notice,
    pause: &Pause,
) {
    let id = &invocation.id;
    let mut held = match held::run(channel, set, handoff, invocation) {
        Ok(()) => return,
        Err((failed, None)) => {
            return notice(&format!(
                "{}. The run of {id} was stopped and nothing more is posted for it.",
                failed.said()
            ));
        }
        Err((failed, Some(held))) => {
            notice(&format!(
                "{}. The capture of {id} is held and posted again until the server answers it.",
                failed.said()
            ));
            held
        }
    };
    let mut series = FIRST;
    loop {
        let (wait, then) = next(series, false, false);
        series = then;
        pause(wait);
        if over.load(Ordering::SeqCst) {
            return;
        }
        let Some(Ending::Again {
            held: Some(again), ..
        }) = held.post(channel)
        else {
            return;
        };
        held = again;
    }
}
