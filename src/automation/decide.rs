//! What automation should do now: a pure function of the settings, the
//! saved runtime state, the clock and the connected displays.
//!
//! Policy, in one place:
//! - **Events, not enforcement.** A rule acts when its time arrives or its
//!   display change happens. In between, nothing is re-applied, so manual
//!   changes stay until the next switch.
//! - **Once per occurrence.** Scheduled wall times up to the `watermark` are
//!   handled; each one applies at most once, across restarts, suspend, clock
//!   and time-zone changes. A display change is the difference from the
//!   last confirmed set of displays, so it happens once.
//! - **Catch-up without a backlog.** After sleep, a restart or a clock jump
//!   only the most recent missed scheduled time counts, and only if nobody
//!   changed the desktop after it (then the manual change is kept).
//! - **Priority.** The latest scheduled time wins; at the same time, or for
//!   one display change, the rule higher in the list wins. A scheduled
//!   switch and a display change found together are applied oldest first,
//!   once each (once in total when they name the same profile).
//! - **Fresh starts.** Times and display changes from before the feature or
//!   a rule was turned on, edited or resumed never count; neither does
//!   anything during a pause. *Skip next* skips only that one scheduled
//!   switch; display rules keep working.
//! - **Newest wins.** A switch waiting for Settings drafts is replaced by
//!   any newer switch, so it can never run over that one later.
//! - **What counts as a manual change** is what the desktop can see: see
//!   `runtime::manual_at`.
use super::clock::{self, wall};
use super::rules::{Automation, DisplayWhen, ProfileRef, Rule, Trigger};
use chrono::{DateTime, Duration, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

pub const STATE_FORMAT: &str = "desktop-cards-automation-state";
pub const STATE_VERSION: u32 = 1;
/// History lines kept.
pub const RECENT: usize = 6;
/// A watermark further ahead than this means the clock was set back (not
/// just a time-zone change); scheduling then restarts from now.
const CLOCK_BACK_HOURS: i64 = 27;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cause {
    /// The local wall time of the occurrence (`2026-10-18T19:00:00`).
    Schedule(String),
    Display(DisplayWhen),
}

/// A switch that should happen.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Due {
    pub rule: String,
    pub profile: ProfileRef,
    pub name: String,
    pub cause: Cause,
    /// The moment it became due.
    pub at: i64,
    /// Identity of the rule/activation that produced this occurrence.
    /// Older history remains readable; unversioned waiting work is retired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<Revision>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Revision {
    activation: i64,
    rule_since: i64,
    trigger: Trigger,
}

impl Revision {
    pub fn new(automation: &Automation, rule: &Rule) -> Self {
        Self {
            activation: automation.since,
            rule_since: rule.since,
            trigger: rule.when.clone(),
        }
    }
}

impl Due {
    /// Validate both persisted waits and queued jobs against current rules.
    /// Trigger identity also fences edits made within the same Unix second.
    pub fn is_current(&self, automation: &Automation, now: i64) -> bool {
        automation.enabled
            && automation.paused_at(now).is_none()
            && self.at >= automation.since
            && automation.rule(&self.rule).is_some_and(|rule| {
                rule.enabled
                    && rule.profile == self.profile
                    && self.at >= rule.since
                    && self.revision.as_ref() == Some(&Revision::new(automation, rule))
            })
            && !automation.paused.is_some_and(|pause| match self.cause {
                Cause::Schedule(_) => pause.stops_schedule(self.at),
                Cause::Display(_) => !pause.skip && pause.covers(self.at),
            })
    }
    /// "19:00" or "AOC 24G2W1G5 connected".
    pub fn reason(&self, h24: bool) -> String {
        match &self.cause {
            Cause::Schedule(local) => wall::parse(local)
                .and_then(|at| {
                    super::rules::Time::new(
                        chrono::Timelike::hour(&at),
                        chrono::Timelike::minute(&at),
                    )
                })
                .map(|time| time.format(h24))
                .unwrap_or_default(),
            Cause::Display(when) => when.happened(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum Effect {
    /// Applied; parts left out because they couldn't be applied.
    Applied {
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        skipped: Vec<String>,
    },
    /// The desktop already looked like this; nothing was written.
    AlreadySet,
    /// Someone changed the desktop after the switch was due; kept.
    KeptManual {
        manual_at: i64,
    },
    /// Settings has unapplied changes to these parts; tried again when
    /// they are applied or discarded.
    Waiting {
        parts: Vec<String>,
    },
    Failed {
        reason: String,
    },
    /// It waited, and a newer switch came before it could run.
    Replaced {
        by: String,
    },
}

/// One line of history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    #[serde(
        default,
        deserialize_with = "read_transaction",
        skip_serializing_if = "Option::is_none"
    )]
    pub transaction: Option<String>,
    pub at: i64,
    pub due: Due,
    pub effect: Effect,
}

/// The displays last confirmed connected, and when.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observed {
    pub names: Vec<String>,
    pub at: i64,
}

/// Runtime state written by the desktop (`automation-state.json`). Never
/// part of a profile or an export.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub format: String,
    pub version: u32,
    /// Scheduled wall times up to here are handled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watermark: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub displays: Option<Observed>,
    /// The Undo ID of the last change automation made, to tell it apart
    /// from changes people make.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<String>,
    /// A switch waiting for unapplied Settings changes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending: Option<Due>,
    /// Newest first.
    #[serde(default)]
    pub recent: Vec<Outcome>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            format: STATE_FORMAT.into(),
            version: STATE_VERSION,
            watermark: None,
            displays: None,
            applied: None,
            pending: None,
            recent: Vec::new(),
        }
    }
}

impl State {
    /// A damaged or unknown state file starts fresh (nothing replays: a
    /// missing watermark begins at the next evaluation's "now").
    pub fn parse(bytes: &[u8]) -> Self {
        serde_json::from_slice::<Self>(bytes)
            .ok()
            .filter(|state| state.format == STATE_FORMAT && state.version == STATE_VERSION)
            .map(|mut state| {
                state.recent.truncate(RECENT);
                state
            })
            .unwrap_or_default()
    }
    pub fn watermark(&self) -> Option<NaiveDateTime> {
        self.watermark.as_deref().and_then(wall::parse)
    }
    pub fn record(&mut self, outcome: Outcome) {
        self.recent.insert(0, outcome);
        self.recent.truncate(RECENT);
    }
    pub fn last(&self) -> Option<&Outcome> {
        self.recent.first()
    }
    /// A newer switch `by` replaces one still waiting (recorded as such).
    /// Returns whether one was waiting.
    pub fn replace_pending(&mut self, by: &Due, now: i64) -> bool {
        let Some(pending) = self.pending.take() else {
            return false;
        };
        if pending != *by {
            self.record(Outcome {
                transaction: None,
                at: now,
                due: pending,
                effect: Effect::Replaced {
                    by: by.name.clone(),
                },
            });
        }
        true
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Decision {
    /// Switches to make now, oldest first.
    pub due: Vec<Due>,
    /// Switches not made because of a later manual change (already in the
    /// state's history).
    pub kept: usize,
    /// The state changed in a way worth saving.
    pub persist: bool,
}

/// Decides what is due at `now` and advances `state`. `displays` is the
/// current, settled set of connected displays (`None`: not watched or not
/// known). `manual_at` returns when someone other than automation last
/// changed the desktop through Halcyon; it is asked only when a
/// missed switch is about to be caught up.
pub fn decide<Tz: TimeZone>(
    automation: &Automation,
    state: &mut State,
    tz: &Tz,
    now: DateTime<Utc>,
    displays: Option<&[String]>,
    manual_at: impl Fn() -> Option<DateTime<Utc>>,
) -> Decision {
    let mut decision = Decision::default();
    let now_unix = clock::unix(now);
    let now_local = clock::local(tz, now);
    let active = automation.enabled && automation.paused_at(now_unix).is_none();
    let counts = |since: i64, at: i64| {
        at >= automation.since
            && at >= since
            && !automation
                .paused
                .is_some_and(|pause| pause.stops_schedule(at))
    };

    // Scheduled times since the watermark.
    let watermark = match state.watermark() {
        Some(mark) if mark <= now_local + Duration::hours(CLOCK_BACK_HOURS) => Some(mark),
        Some(_) => {
            // The clock went back by more than a time-zone change: start
            // again from now instead of waiting days for the old mark.
            decision.persist = true;
            None
        }
        None => None,
    };
    if let Some(mark) = watermark.filter(|_| automation.enabled) {
        let mut best: Option<(NaiveDateTime, usize)> = None;
        for (index, rule) in automation.rules.iter().enumerate() {
            let Trigger::Schedule { time, days } = &rule.when else {
                continue;
            };
            if !rule.enabled {
                continue;
            }
            if let Some(at) = clock::latest(*time, *days, mark, now_local)
                && best.is_none_or(|(known, _)| at > known)
            {
                best = Some((at, index));
            }
        }
        if let Some((local, index)) = best {
            // Something was passed: the advanced watermark must survive a
            // restart, whatever happens with it.
            decision.persist = true;
            let rule = &automation.rules[index];
            let at = clock::unix(clock::instant(tz, local));
            if active && counts(rule.since, at) {
                let due = Due {
                    revision: Some(Revision::new(automation, rule)),
                    rule: rule.id.clone(),
                    profile: rule.profile.clone(),
                    name: rule.name.clone(),
                    cause: Cause::Schedule(wall::format(local)),
                    at,
                };
                match manual_at()
                    .map(clock::unix)
                    .filter(|manual| kept_manual(*manual, at))
                {
                    Some(manual) => {
                        state.record(Outcome {
                            transaction: None,
                            at: now_unix,
                            due,
                            effect: Effect::KeptManual { manual_at: manual },
                        });
                        decision.kept += 1;
                    }
                    None => decision.due.push(due),
                }
            }
        }
    }
    let mark = watermark.map_or(now_local, |mark| mark.max(now_local));
    if state.watermark() != Some(mark) {
        if state.watermark.is_none() || watermark.is_none() {
            decision.persist = true;
        }
        state.watermark = Some(wall::format(mark));
    }

    // Display changes since the last confirmed set.
    if let Some(current) = displays {
        let mut current = current.to_vec();
        current.sort();
        current.dedup();
        match &state.displays {
            Some(before) if before.names != current => {
                for rule in automation.active() {
                    let Trigger::Display(when) = &rule.when else {
                        continue;
                    };
                    // The set before must have been seen after the rule
                    // and the feature were turned on.
                    let seen = before.at >= automation.since && before.at >= rule.since;
                    if active && seen && when.fired(&before.names, &current) {
                        decision.due.push(Due {
                            revision: Some(Revision::new(automation, rule)),
                            rule: rule.id.clone(),
                            profile: rule.profile.clone(),
                            name: rule.name.clone(),
                            cause: Cause::Display(when.clone()),
                            at: now_unix,
                        });
                        break;
                    }
                }
                decision.persist = true;
            }
            Some(_) => {}
            None => decision.persist = true,
        }
        state.displays = Some(Observed {
            names: current,
            at: now_unix,
        });
    }

    // One apply when both name the same profile; the newer one is kept.
    if decision.due.len() == 2 && decision.due[0].profile == decision.due[1].profile {
        decision.due.remove(0);
    }
    // A newer switch replaces one still waiting for Settings; nothing waits
    // while automation is off or paused, or for a rule that's gone.
    if let Some(newer) = decision.due.last() {
        decision.persist |= state.replace_pending(newer, now_unix);
    } else if !active
        || state
            .pending
            .as_ref()
            .is_some_and(|pending| !pending.is_current(automation, now_unix))
    {
        decision.persist |= state.pending.take().is_some();
    }
    decision
}

/// Whether a switch that waited is still wanted: not if someone changed the
/// desktop after it became due. Applying the held drafts is such a change
/// (manual choices win); discarding them is not, so the switch runs.
pub fn still_due(due: &Due, manual_at: Option<DateTime<Utc>>, now: i64) -> Result<(), Outcome> {
    match manual_at
        .map(clock::unix)
        .filter(|manual| kept_manual(*manual, due.at))
    {
        Some(manual) => Err(Outcome {
            transaction: None,
            at: now,
            due: due.clone(),
            effect: Effect::KeptManual { manual_at: manual },
        }),
        None => Ok(()),
    }
}

/// Whether a manual change at `manual` wins over a switch due at `due`
/// (both whole Unix seconds). The same second counts as after: a choice
/// saved as a switch fell due is kept rather than overwritten at once.
fn kept_manual(manual: i64, due: i64) -> bool {
    manual >= due
}

/// The next moment a scheduled rule switches, after `now` and the
/// watermark, outside any pause: its local wall time and rule index.
pub fn next_switch<Tz: TimeZone>(
    automation: &Automation,
    state: &State,
    tz: &Tz,
    now: DateTime<Utc>,
) -> Option<(NaiveDateTime, usize)> {
    if !automation.enabled {
        return None;
    }
    let mut after = clock::local(tz, now);
    if let Some(mark) = state.watermark().filter(|mark| *mark > after) {
        if mark <= after + Duration::hours(CLOCK_BACK_HOURS) {
            after = mark;
        }
    }
    // Look past a pause: occurrences inside it are skipped.
    for _ in 0..64 {
        let mut best: Option<(NaiveDateTime, usize)> = None;
        for (index, rule) in automation.rules.iter().enumerate() {
            let Trigger::Schedule { time, days } = &rule.when else {
                continue;
            };
            if !rule.enabled {
                continue;
            }
            if let Some(at) = clock::next(*time, *days, after)
                && best.is_none_or(|(known, _)| at < known)
            {
                best = Some((at, index));
            }
        }
        let (local, index) = best?;
        let at = clock::unix(clock::instant(tz, local));
        match automation.paused.filter(|pause| pause.stops_schedule(at)) {
            Some(pause) => match pause.until {
                Some(_) => after = local,
                None => return None,
            },
            None => return Some((local, index)),
        }
    }
    None
}

/// When the desktop should look again: the next scheduled switch, but at
/// most a minute away so clock and time-zone changes are noticed.
pub fn next_check<Tz: TimeZone>(
    automation: &Automation,
    state: &State,
    tz: &Tz,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    if !automation.running() {
        return None;
    }
    let minute = now + Duration::seconds(60);
    let switch = next_switch(automation, state, tz, now)
        .map(|(local, _)| clock::instant(tz, local))
        .filter(|at| *at > now);
    let pause_end = automation
        .paused
        .and_then(|pause| pause.until)
        .map(clock::from_unix)
        .filter(|at| *at > now);
    Some(
        [Some(minute), switch, pause_end]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(minute),
    )
}

fn read_transaction<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(value
        .as_str()
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 96
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        })
        .map(str::to_owned))
}
