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
//! The loop (`run.rs`, E4 of 2 nov) is the only one that does the work: it
//! looks at the steps in order, does one small operation of the first one
//! incomplete, keeps it, and starts again from the first step. There is no
//! barrier and no other thread.

pub mod run;
pub mod steps;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use serde::Serialize;

#[allow(unused_imports)]
pub use steps::{About, Class, Count, Level, Scope, Step, STEPS};

use super::DocBase;

/// What a count of the process takes into account besides the scope.
#[derive(Debug, Clone, Default)]
pub struct Looking {
    /// The steps a person asked for, in the scope of the ask: their
    /// questions go out even with the automatic reading off.
    pub allowed: Vec<Step>,
    /// Steps set aside as a whole now (the round past its cap waits for a
    /// person, or a step that could not go on): their pending elements are
    /// counted as set aside, and they do not hold the steps after.
    pub aside: BTreeSet<Step>,
    /// Stop at the first incomplete step.
    pub first_only: bool,
}

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
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn step(&self, step: Step) -> Option<&StepState> {
        self.steps.iter().find(|s| s.about.step == step)
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn pending(&self, step: Step) -> usize {
        self.step(step).map(|s| s.count.pending).unwrap_or(0)
    }
}

impl DocBase {
    /// How the process stands over `scope`: what each step has left and
    /// the first one incomplete. Reads only. With `first_only`, it stops at
    /// the first incomplete step (what the loop of A2 needs to choose its
    /// next operation, and what a person's ask waits for).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn process_state(&self, scope: &Scope, first_only: bool) -> Result<ProcessState, String> {
        self.process_state_looking(scope, &Looking { first_only, ..Looking::default() })
    }

    /// The same, with what the loop takes into account (`Looking`).
    pub fn process_state_looking(&self, scope: &Scope, looking: &Looking) -> Result<ProcessState, String> {
        let first_only = looking.first_only;
        let reading_on = self.background_reading()?;
        let conn = self.conn();
        let held = steps::asking_held(&conn, reading_on)?;
        // A person's ask goes out with the reading off; never past an
        // engine paused, nor without an engine.
        let held_for = |step: Step| match held {
            Some(why) if looking.allowed.contains(&step) && why == "la lectura automática está apagada" => {
                if super::services::ask_engine(&conn).ok().flatten().is_none() {
                    Some("falta elegir el motor de IA")
                } else {
                    steps::asking_held(&conn, true).ok().flatten()
                }
            }
            other => other,
        };
        let mut state = ProcessState { steps: Vec::new(), first_incomplete: None, unknown: Vec::new() };
        for step in STEPS {
            let class = steps::class_now(&conn, step)?;
            let mut count = steps::count_step(&conn, step, scope)?;
            let mut step_held = if class == Class::Asks { held_for(step) } else { None };
            if looking.aside.contains(&step) {
                step_held = Some("apartado por ahora");
            }
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
