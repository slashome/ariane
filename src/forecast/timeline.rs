//! Turns hours of focused work counted from now into moments of the calendar.
//!
//! The scheduler counts in focused hours from now, the same with or without a
//! calendar. A [`Timeline`] lays the carrier's weekly blocks end to end from
//! now on, in the calendar's time zone, skipping days off, each block worth its
//! length times `focus`. A focused hour `h` is then a moment inside one block:
//! focused hours accrue evenly over the block's wall-clock time.

use jiff::civil::Date;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

use super::input::Capacity;

/// How many days ahead the blocks are laid out before the timeline gives up:
/// work further than that has no date.
const HORIZON_DAYS: i64 = 3660;

/// One block of one day, with the focused hours counted from now at its ends.
#[derive(Debug, Clone, Copy)]
struct Interval {
    /// When the block starts, or now for a block already started.
    start: Timestamp,
    /// When it ends.
    end: Timestamp,
    /// Focused hours from now at `start`.
    from: f64,
    /// Focused hours from now at `end`.
    to: f64,
}

/// The part of a stretch of work that falls in one block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Piece {
    /// Focused hours from now where it starts.
    pub from: f64,
    /// Focused hours from now where it ends.
    pub to: f64,
    /// When it starts.
    pub start: Timestamp,
    /// When it ends.
    pub end: Timestamp,
}

/// The carrier's blocks laid out from now on, extended on demand.
#[derive(Debug, Clone)]
pub struct Timeline {
    /// The calendar's time zone.
    zone: TimeZone,
    /// What the blocks are.
    capacity: Capacity,
    /// The moment hours are counted from.
    now: Timestamp,
    /// The blocks laid out so far.
    intervals: Vec<Interval>,
    /// The next day to lay out.
    next_day: Date,
    /// Days laid out so far.
    days: i64,
}

impl Timeline {
    /// The timeline of `capacity` from `now`; `None` when its time zone is
    /// unknown or it has no hours at all.
    pub fn new(capacity: &Capacity, now: Timestamp) -> Option<Timeline> {
        let zone = TimeZone::get(&capacity.timezone).ok()?;
        let has_hours = capacity
            .blocks
            .iter()
            .any(|block| block.to > block.from && block.weekdays.count() > 0);
        if !has_hours || capacity.focus <= 0.0 {
            return None;
        }
        let next_day = now.to_zoned(zone.clone()).date();
        Some(Timeline {
            zone,
            capacity: capacity.clone(),
            now,
            intervals: Vec::new(),
            next_day,
            days: 0,
        })
    }

    /// The time zone of the calendar.
    pub fn zone(&self) -> &TimeZone {
        &self.zone
    }

    /// The moment the work starting at focused hour `hours` starts: a block
    /// ending exactly there gives way to the next one.
    pub fn start_at(&mut self, hours: f64) -> Option<Timestamp> {
        self.cover(hours, true)?;
        let interval = self.intervals.iter().find(|interval| interval.to > hours)?;
        Some(self.inside(interval, hours))
    }

    /// The moment the work ending at focused hour `hours` ends: the block
    /// ending there, not the start of the next one.
    pub fn end_at(&mut self, hours: f64) -> Option<Timestamp> {
        self.cover(hours, false)?;
        let interval = self
            .intervals
            .iter()
            .find(|interval| interval.to >= hours)?;
        Some(self.inside(interval, hours))
    }

    /// The pieces of the work from focused hour `from` to `to`, one per block
    /// it spans: the focused hours each piece starts and ends at, and its
    /// wall-clock moments.
    pub fn pieces(&mut self, from: f64, to: f64) -> Option<Vec<Piece>> {
        if to <= from {
            return Some(Vec::new());
        }
        self.cover(to, false)?;
        let spans: Vec<(f64, f64, Interval)> = self
            .intervals
            .iter()
            .filter(|interval| interval.to > from && interval.from < to)
            .map(|interval| (from.max(interval.from), to.min(interval.to), *interval))
            .collect();
        Some(
            spans
                .into_iter()
                .map(|(first, last, interval)| Piece {
                    from: first,
                    to: last,
                    start: self.inside(&interval, first),
                    end: self.inside(&interval, last),
                })
                .collect(),
        )
    }

    /// The moment of focused hour `hours` inside `interval`.
    fn inside(&self, interval: &Interval, hours: f64) -> Timestamp {
        let seconds = ((hours - interval.from).max(0.0) / self.capacity.focus * 3600.0).round();
        let moment = interval
            .start
            .checked_add((seconds as i64).seconds())
            .unwrap_or(interval.end);
        moment.min(interval.end)
    }

    /// Lays out days until the blocks reach focused hour `hours`, strictly
    /// past it when `strict`; `None` past the horizon.
    fn cover(&mut self, hours: f64, strict: bool) -> Option<()> {
        let reached = |timeline: &Timeline| {
            timeline.intervals.last().is_some_and(|last| {
                if strict {
                    last.to > hours
                } else {
                    last.to >= hours
                }
            })
        };
        while !reached(self) {
            if self.days >= HORIZON_DAYS {
                return None;
            }
            self.lay_out_day();
        }
        Some(())
    }

    /// Adds the blocks of the next day, unless it is a day off.
    fn lay_out_day(&mut self) {
        let day = self.next_day;
        self.next_day = day.tomorrow().unwrap_or(day);
        self.days += 1;
        if self
            .capacity
            .days_off
            .iter()
            .any(|off| off.from <= day && day <= off.to)
        {
            return;
        }
        let weekday = usize::from(day.weekday().to_monday_zero_offset().unsigned_abs());
        let mut blocks: Vec<(u16, u16)> = self
            .capacity
            .blocks
            .iter()
            .filter(|block| block.weekdays.holds(weekday) && block.to > block.from)
            .map(|block| (block.from.minutes(), block.to.minutes()))
            .collect();
        blocks.sort_unstable();
        for (from, to) in blocks {
            let (Some(start), Some(end)) = (self.moment(day, from), self.moment(day, to)) else {
                continue;
            };
            let start = start.max(self.now);
            if end <= start {
                continue;
            }
            let from = self.intervals.last().map_or(0.0, |last| last.to);
            let length = (end.as_second() - start.as_second()) as f64 / 3600.0;
            self.intervals.push(Interval {
                start,
                end,
                from,
                to: from + length * self.capacity.focus,
            });
        }
    }

    /// The moment `minutes` after midnight on `day`, `24:00` being the next
    /// midnight.
    fn moment(&self, day: Date, minutes: u16) -> Option<Timestamp> {
        let (day, minutes) = if minutes >= 24 * 60 {
            (day.tomorrow().ok()?, minutes - 24 * 60)
        } else {
            (day, minutes)
        };
        let hour = i8::try_from(minutes / 60).ok()?;
        let minute = i8::try_from(minutes % 60).ok()?;
        day.at(hour, minute, 0, 0)
            .to_zoned(self.zone.clone())
            .ok()
            .map(|zoned| zoned.timestamp())
    }
}
