//! The closing date of an item, `closed` (ADR-0027, SPEC.md §1.3).
//!
//! Every writer of a status applies [`settle`]: entering a status of the
//! `done` category writes today's date, leaving that category removes it, and
//! moving between two `done` statuses keeps it. Nothing back-fills the key:
//! an item finished before it existed is only [`approximate`]d from the
//! history git keeps, and that date is shown as approximate, never written.

use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml_serde::{Mapping, Value};

use crate::history::Transition;
use crate::item::{self, Kind};
use crate::node::Category;

/// The header key.
pub const KEY: &str = "closed";

/// Tells whether an item of `kind` carries a closing date: tasks, stories and
/// epics, whose statuses come from the node's workflow.
pub fn applies_to(kind: Kind) -> bool {
    matches!(kind, Kind::Task | Kind::Story | Kind::Epic)
}

/// Today's date, `YYYY-MM-DD`, on the machine's clock and in its time zone,
/// as `date` gives it; in UTC when `date` cannot run.
pub fn today() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|date| crate::schema::is_date(date))
        .unwrap_or_else(utc_today)
}

/// Today's date in UTC, from the system clock.
fn utc_today() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    civil(i64::try_from(seconds / 86_400).unwrap_or_default())
}

/// The `YYYY-MM-DD` date `days` days after 1970-01-01, in the proleptic
/// Gregorian calendar (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> String {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Applies the rule of ADR-0027 to `header` after a change of status from the
/// category `before` to the category `after`: entering `done` writes `today`,
/// leaving it removes `closed`, staying in it keeps the date. An unknown
/// category after the change leaves the header alone.
pub fn settle(
    header: &mut Mapping,
    before: Option<Category>,
    after: Option<Category>,
    today: &str,
) {
    match after {
        Some(Category::Done) if before != Some(Category::Done) => {
            item::set_key(header, KEY, Some(Value::String(today.to_owned())));
        }
        Some(Category::Done) | None => {}
        Some(_) => item::set_key(header, KEY, None),
    }
}

/// The approximate closing date of an item from the statuses git saw it go
/// through, oldest first: the date of its last entry into a status `is_done`
/// accepts. `None` when git never saw it in such a status, or saw it leave.
pub fn approximate(transitions: &[Transition], is_done: impl Fn(&str) -> bool) -> Option<&str> {
    let mut entered = None;
    let mut was_done = false;
    for transition in transitions {
        let done = transition.status.as_deref().is_some_and(&is_done);
        if done && !was_done {
            entered = Some(transition.date.as_str());
        }
        was_done = done;
    }
    entered.filter(|_| was_done)
}

/// Unit tests of the closing rule and of its approximation.
#[cfg(test)]
mod tests {
    use super::*;

    fn closed(header: &Mapping) -> Option<&str> {
        header.get(KEY).and_then(Value::as_str)
    }

    fn header(text: &str) -> Mapping {
        yaml_serde::from_str(text).unwrap()
    }

    #[test]
    fn entering_done_writes_today_after_created_leaving_removes_it_and_done_to_done_keeps_it() {
        let mut task = header("id: T1\nstatus: done\ncreated: 2026-01-02\nrelease: '1.0'\n");
        settle(
            &mut task,
            Some(Category::InProgress),
            Some(Category::Done),
            "2026-11-12",
        );
        assert_eq!(closed(&task), Some("2026-11-12"));
        let keys: Vec<&str> = task.keys().filter_map(Value::as_str).collect();
        assert_eq!(keys, ["id", "status", "created", "closed", "release"]);
        settle(
            &mut task,
            Some(Category::Done),
            Some(Category::Done),
            "2026-12-01",
        );
        assert_eq!(closed(&task), Some("2026-11-12"));
        settle(&mut task, Some(Category::Done), None, "2026-12-01");
        assert_eq!(closed(&task), Some("2026-11-12"));
        settle(
            &mut task,
            Some(Category::Done),
            Some(Category::Todo),
            "2026-12-01",
        );
        assert_eq!(closed(&task), None);
        settle(
            &mut task,
            Some(Category::Todo),
            Some(Category::Dropped),
            "2026-12-01",
        );
        assert_eq!(closed(&task), None);
    }

    #[test]
    fn reclosing_keeps_only_the_last_date() {
        let mut task = header("id: T1\nstatus: done\nclosed: 2026-01-01\n");
        settle(
            &mut task,
            Some(Category::InProgress),
            Some(Category::Done),
            "2026-11-12",
        );
        assert_eq!(closed(&task), Some("2026-11-12"));
    }

    fn transitions(steps: &[(&str, &str)]) -> Vec<Transition> {
        steps
            .iter()
            .map(|(date, status)| Transition {
                date: (*date).to_owned(),
                status: Some((*status).to_owned()),
            })
            .collect()
    }

    #[test]
    fn the_approximation_is_the_last_entry_into_done_while_still_done() {
        let done = |status: &str| matches!(status, "done" | "valide");
        let steps = transitions(&[
            ("2026-01-01", "todo"),
            ("2026-02-01", "done"),
            ("2026-03-01", "valide"),
        ]);
        assert_eq!(approximate(&steps, done), Some("2026-02-01"));
        let reopened = transitions(&[
            ("2026-01-01", "done"),
            ("2026-02-01", "todo"),
            ("2026-03-01", "done"),
        ]);
        assert_eq!(approximate(&reopened, done), Some("2026-03-01"));
        let left = transitions(&[("2026-01-01", "done"), ("2026-02-01", "todo")]);
        assert_eq!(approximate(&left, done), None);
        assert_eq!(approximate(&[], done), None);
    }

    #[test]
    fn days_since_the_epoch_become_calendar_dates() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(11_016), "2000-02-29");
        assert_eq!(civil(20_769), "2026-11-12");
        assert!(crate::schema::is_date(&today()));
    }
}
