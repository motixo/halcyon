//! Sleep until actual work is due; completed jobs set their next deadline.
//!
//! Deadlines are monotonic `Instant`s, but the monotonic clock stops while
//! the computer is suspended. A long sleep therefore also remembers the
//! wall-clock moment it stands for and naps at most `NAP` at a time, waking
//! as soon as either clock reaches the deadline: after a resume, a wake that
//! was due during suspend arrives within `NAP`, not late by the time spent
//! asleep. Short sleeps are unaffected.
//!
//! A wake reports how much earlier than its monotonic deadline it came
//! (zero unless the wall clock got there first). The receiver must treat
//! its monotonic deadlines as that much closer, or a handler comparing
//! `Instant`s would find nothing due and the same wake would never be armed
//! again (see `Cards::resumed`).
use cosmic::iced::{Subscription, futures::stream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const NAP: Duration = Duration::from_secs(20);

/// Wakes at `deadline`, or at the wall-clock moment it stood for when it
/// was armed; yields the monotonic time still left then.
pub fn wake_at(deadline: Instant) -> Subscription<Duration> {
    Subscription::run_with(deadline, |deadline| {
        let deadline = *deadline;
        stream::once(sleep_until(deadline))
    })
}

async fn sleep_until(deadline: Instant) -> Duration {
    let wall = SystemTime::now() + deadline.saturating_duration_since(Instant::now());
    nap_until(
        || Instant::now() >= deadline || SystemTime::now() >= wall,
        || deadline.saturating_duration_since(Instant::now()),
        NAP,
    )
    .await;
    deadline.saturating_duration_since(Instant::now())
}

/// Naps (at most `nap` each) until `due` says either clock has reached the
/// deadline; `left` is the monotonic time still to go.
async fn nap_until(due: impl Fn() -> bool, left: impl Fn() -> Duration, nap: Duration) {
    while !due() {
        tokio::time::sleep(left().min(nap)).await;
    }
}

pub fn next_clock_tick(now: Instant, interval: Duration) -> Instant {
    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now + until_boundary(wall, interval)
}

fn until_boundary(wall: Duration, interval: Duration) -> Duration {
    let interval_ms = interval.as_millis().max(1);
    Duration::from_millis((interval_ms - wall.as_millis() % interval_ms) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minute_clock_updates_on_the_minute_instead_of_a_minute_after_launch() {
        assert_eq!(
            until_boundary(Duration::from_millis(75_250), Duration::from_secs(60)),
            Duration::from_millis(44_750)
        );
        assert_eq!(
            until_boundary(Duration::from_secs(120), Duration::from_secs(60)),
            Duration::from_secs(60)
        );
    }

    /// Without a suspend a wake sleeps to its deadline. When the monotonic
    /// clock stood still while the wall clock passed the deadline (a
    /// suspend), it comes after one nap instead of the remaining hour.
    #[tokio::test]
    async fn a_long_wake_is_not_late_by_the_time_spent_suspended() {
        use std::cell::Cell;
        let nap = Duration::from_millis(10);
        let start = Instant::now();
        let deadline = start + Duration::from_millis(35);
        nap_until(
            || Instant::now() >= deadline,
            || deadline.saturating_duration_since(Instant::now()),
            nap,
        )
        .await;
        assert!(start.elapsed() >= Duration::from_millis(35));
        let checks = Cell::new(0);
        let woke = tokio::time::timeout(
            Duration::from_secs(2),
            nap_until(
                || {
                    checks.set(checks.get() + 1);
                    // The wall clock passed the deadline during the first nap.
                    checks.get() > 1
                },
                || Duration::from_secs(3600),
                nap,
            ),
        )
        .await;
        assert!(woke.is_ok());
        assert_eq!(checks.get(), 2);
    }
}
