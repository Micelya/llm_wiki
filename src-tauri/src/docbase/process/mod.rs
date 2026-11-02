//! The process (A1 of 2 nov): the steps a file and its documents go
//! through, in order, each with its condition of being complete, told the
//! same way in all of them (`steps.rs`).
//!
//! It is always the same process, whatever starts it: the first load, a
//! new folder, the watcher, a person who asks. A person is one more start
//! (change B of 2 nov): what a person asks leaves incomplete the steps it
//! touches, in the scope it touches, and the same loop takes them, first
//! within their step: it waits only for the operation going on and the
//! steps before of its own document (decision 2 of 2 nov; `Scope`). The
//! exceptions already known change only how one element of a step is
//! done, never the order: asking the model to read a document that cannot
//! be read, or an old manuscript (the step of the scans: for that document
//! it asks, whatever the configuration says), and asking the model where
//! the documents of one PDF start (the step of the question of the cut:
//! also the doubtful steps the app does not ask by itself). What a person
//! does on the screen without touching the process is none of this.
//!
//! This part only tells the steps and what each has left: who does the
//! work does not change yet (A2 puts one loop in place of the threads and
//! the barrier).

pub mod steps;
#[cfg(test)]
mod tests;

use serde::Serialize;

pub use steps::{About, Class, Count, Level, Scope, Step, STEPS};

use super::DocBase;

/// How one step stands.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StepState {
    pub about: About,
    /// Its class now: the reading of the scans asks with an engine in the
    /// cloud.
    pub class: Class,
    pub count: Count,
    /// Why its questions are set aside now, if they are.
    pub held: Option<&'static str>,
}

impl StepState {
    /// Nothing of it is pending (what is set aside does not hold it).
    pub fn complete(&self) -> bool {
        self.count.pending == 0
    }
}

/// How the process stands, over the whole base or a scope.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessState {
    /// The steps counted, in order: all 18, or up to the first incomplete
    /// one (`first_only`).
    pub steps: Vec<StepState>,
    /// The first step with something pending: the one the process works on.
    pub first_incomplete: Option<Step>,
    /// The steps before it, or all when everything is complete, that
    /// cannot say by themselves whether something is left (D-452): a pass
    /// tells.
    pub unknown: Vec<Step>,
}

impl ProcessState {
    pub fn step(&self, step: Step) -> Option<&StepState> {
        self.steps.iter().find(|s| s.about.step == step)
    }

    pub fn pending(&self, step: Step) -> usize {
        self.step(step).map(|s| s.count.pending).unwrap_or(0)
    }
}

impl DocBase {
    /// How the process stands over `scope`: what each step has left and
    /// the first one incomplete. Reads only. With `first_only`, it stops at
    /// the first incomplete step (what the loop of A2 needs to choose its
    /// next operation, and what a person's ask waits for).
    pub fn process_state(&self, scope: &Scope, first_only: bool) -> Result<ProcessState, String> {
        let reading_on = self.background_reading()?;
        let conn = self.conn();
        let held = steps::asking_held(&conn, reading_on)?;
        let mut state = ProcessState { steps: Vec::new(), first_incomplete: None, unknown: Vec::new() };
        for step in STEPS {
            let class = steps::class_now(&conn, step)?;
            let mut count = steps::count_step(&conn, step, scope)?;
            let step_held = if class == Class::Asks { held } else { None };
            if step_held.is_some() {
                // Questions that cannot go out now wait, set aside; what
                // the step closes without asking is still pending.
                let asks = count.pending - count.without_asking.min(count.pending);
                count.pending -= asks;
                count.set_aside += asks;
            }
            let step_state = StepState { about: step.about(), class, count, held: step_held };
            let incomplete = !step_state.complete();
            if !count.known && state.first_incomplete.is_none() {
                state.unknown.push(step);
            }
            state.steps.push(step_state);
            if incomplete && state.first_incomplete.is_none() {
                state.first_incomplete = Some(step);
                if first_only {
                    break;
                }
            }
        }
        Ok(state)
    }
}
