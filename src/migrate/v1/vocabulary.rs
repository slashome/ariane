//! The status and priority words of the legacy prose headers, and what they
//! mean in the version-1 vocabulary.
//!
//! The legacy items wrote their status and priority in free French prose
//! ("⏳ à faire", "✅ LIVRÉE", "🟡 moyenne"). These tables map the observed words
//! onto the fixed values of SPEC.md: a status category, a reflexion status, or a
//! priority level. Each entry also says whether the mapping is a judgment call
//! (`inferred`), which is shown to the reviewer.
//!
//! Emoji are dropped before matching and never decide anything on their own:
//! 🔴 means "high priority" in some files and "in progress" in others.

use crate::item::Kind;
use crate::node::Category;

use super::prose::StatusValue;

/// A word found in a table. `T` is the type of value the table maps to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<T> {
    /// The value the word maps to.
    pub value: T,
    /// Whether the mapping is a judgment call rather than a plain translation.
    pub inferred: bool,
    /// Whether the whole text was the word, or the word followed by an explanation.
    pub exact: bool,
}

/// The result of reading a legacy value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading<T> {
    /// A known word starts the text.
    Found(Match<T>),
    /// Nothing but emoji or punctuation: nothing to decide on.
    EmojiOnly,
    /// No known word; carries the normalized text.
    Unknown(String),
}

/// Status words of tasks, stories and backlog items, with their category and whether the mapping is inferred.
const TASK_STATUSES: &[(&str, Category, bool)] = &[
    ("commencée puis enterrée", Category::Dropped, true),
    ("commencé puis enterré", Category::Dropped, true),
    ("à faire", Category::Todo, false),
    ("a faire", Category::Todo, false),
    ("todo", Category::Todo, false),
    ("ouvert", Category::Todo, false),
    ("ouverte", Category::Todo, false),
    ("prêt", Category::Todo, true),
    ("prête", Category::Todo, true),
    ("qualifié", Category::Todo, true),
    ("qualifiée", Category::Todo, true),
    ("à clarifier", Category::Backlog, true),
    ("à cadrer", Category::Backlog, true),
    ("à trancher", Category::Backlog, true),
    ("à réfléchir", Category::Backlog, true),
    ("à arbitrer", Category::Backlog, true),
    ("en attente", Category::Backlog, true),
    ("backlog", Category::Backlog, false),
    ("optionnel", Category::Backlog, true),
    ("optionnelle", Category::Backlog, true),
    ("r&d", Category::Backlog, true),
    ("maturation perso", Category::Backlog, true),
    ("en veille", Category::Backlog, true),
    ("reporté", Category::Backlog, true),
    ("reportée", Category::Backlog, true),
    ("en cours", Category::InProgress, false),
    ("engagé", Category::InProgress, true),
    ("engagée", Category::InProgress, true),
    ("commencé", Category::InProgress, true),
    ("commencée", Category::InProgress, true),
    ("à tester", Category::InProgress, true),
    ("bloqué", Category::Blocked, false),
    ("bloquée", Category::Blocked, false),
    ("livré", Category::Done, false),
    ("livrée", Category::Done, false),
    ("fait", Category::Done, false),
    ("faite", Category::Done, false),
    ("terminé", Category::Done, false),
    ("terminée", Category::Done, false),
    ("clos", Category::Done, false),
    ("close", Category::Done, false),
    ("done", Category::Done, false),
    ("résolu", Category::Done, false),
    ("résolue", Category::Done, false),
    ("archivé", Category::Done, true),
    ("archivée", Category::Done, true),
    ("migré", Category::Done, true),
    ("migrée", Category::Done, true),
    ("validé", Category::Done, true),
    ("validée", Category::Done, true),
    ("tranché", Category::Done, true),
    ("tranchée", Category::Done, true),
    ("enterré", Category::Dropped, false),
    ("enterrée", Category::Dropped, false),
    ("abandonné", Category::Dropped, false),
    ("abandonnée", Category::Dropped, false),
    ("annulé", Category::Dropped, false),
    ("annulée", Category::Dropped, false),
    ("obsolète", Category::Dropped, true),
];

/// Status words of reflexions: open, trial ("en essai") or settled.
const REFLEXION_STATUSES: &[(&str, StatusValue, bool)] = &[
    ("partiellement tranchée", StatusValue::Open, true),
    ("partiellement tranché", StatusValue::Open, true),
    ("ouverte", StatusValue::Open, false),
    ("ouvert", StatusValue::Open, false),
    ("en essai", StatusValue::Trial, false),
    ("à l'essai", StatusValue::Trial, false),
    ("a l'essai", StatusValue::Trial, false),
    ("à l’essai", StatusValue::Trial, false),
    ("r&d", StatusValue::Open, true),
    ("en cours", StatusValue::Open, false),
    ("à trancher", StatusValue::Open, false),
    ("à instruire", StatusValue::Open, false),
    ("tranchée", StatusValue::Settled, false),
    ("tranché", StatusValue::Settled, false),
    ("décidée", StatusValue::Settled, false),
    ("décidé", StatusValue::Settled, false),
    ("close", StatusValue::Settled, false),
    ("clos", StatusValue::Settled, false),
    ("abandonnée", StatusValue::Settled, true),
    ("abandonné", StatusValue::Settled, true),
];

/// Priority words and their level: low, medium or high.
const PRIORITIES: &[(&str, &str, bool)] = &[
    ("moyenne-haute", "medium", true),
    ("moyenne haute", "medium", true),
    ("basse", "low", false),
    ("faible", "low", false),
    ("low", "low", false),
    ("moyenne", "medium", false),
    ("medium", "medium", false),
    ("haute", "high", false),
    ("élevée", "high", false),
    ("high", "high", false),
    ("absolue", "high", true),
];

/// Reads a legacy status for an item of `kind`: reflexions use their own table,
/// every other kind the task table. An "open" backlog item stays in the backlog
/// rather than becoming a planned task.
pub fn status(kind: Kind, raw: &str) -> Reading<StatusValue> {
    match kind {
        Kind::Reflexion => lookup(raw, REFLEXION_STATUSES),
        _ => match lookup(raw, TASK_STATUSES) {
            Reading::Found(found) => Reading::Found(Match {
                value: StatusValue::Category(backlog_when_open(kind, found.value)),
                inferred: found.inferred,
                exact: found.exact,
            }),
            Reading::EmojiOnly => Reading::EmojiOnly,
            Reading::Unknown(text) => Reading::Unknown(text),
        },
    }
}

/// Reads a legacy priority.
pub fn priority(raw: &str) -> Reading<&'static str> {
    lookup(raw, PRIORITIES)
}

/// Lower-cases the text, drops emoji and other symbols, and collapses the whitespace, so that words compare equal however they were decorated.
pub fn normalize(raw: &str) -> String {
    let kept: String = raw
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || "&'’-/.,;:()«»".contains(*c))
        .collect();
    kept.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Maps an open status to the backlog category for a backlog item.
fn backlog_when_open(kind: Kind, category: Category) -> Category {
    match (kind, category) {
        (Kind::Backlog, Category::Todo) => Category::Backlog,
        _ => category,
    }
}

/// Looks the normalized text up in `table`, longest words first, so that
/// "partiellement tranchée" wins over "tranchée". A word matches the whole
/// text, or its start when followed by a separator ("haute — à faire vite").
///
/// `T: Copy` means the values can be copied out of the table instead of
/// borrowed.
fn lookup<T: Copy>(raw: &str, table: &[(&str, T, bool)]) -> Reading<T> {
    let text = normalize(raw);
    let text = text.trim_matches(|c: char| c == '.' || c == ':' || c.is_whitespace());
    if text.is_empty() {
        return if raw.trim().is_empty() {
            Reading::Unknown(String::new())
        } else {
            Reading::EmojiOnly
        };
    }
    let mut entries: Vec<&(&str, T, bool)> = table.iter().collect();
    entries.sort_by_key(|(term, _, _)| std::cmp::Reverse(term.chars().count()));
    for (term, value, inferred) in entries {
        if text == *term {
            return Reading::Found(Match {
                value: *value,
                inferred: *inferred,
                exact: true,
            });
        }
        if text
            .strip_prefix(term)
            .is_some_and(|rest| rest.starts_with(|c: char| !c.is_alphanumeric() && c != '-'))
        {
            return Reading::Found(Match {
                value: *value,
                inferred: *inferred,
                exact: false,
            });
        }
    }
    Reading::Unknown(text.to_owned())
}

/// Unit tests of the vocabulary tables.
#[cfg(test)]
mod tests {
    use super::*;

    /// The match of a reading, failing the test when nothing was found.
    fn found<T: Copy>(reading: Reading<T>) -> Match<T> {
        match reading {
            Reading::Found(found) => found,
            _ => panic!("expected a match"),
        }
    }

    #[test]
    fn a_bare_term_is_an_exact_match() {
        let found = found(status(Kind::Task, "⏳ à faire"));
        assert_eq!(found.value, StatusValue::Category(Category::Todo));
        assert!(found.exact);
    }

    #[test]
    fn a_term_followed_by_an_explanation_matches_but_is_not_exact() {
        let found = found(status(
            Kind::Task,
            "✅ **LIVRÉE** — ticket clos le 2030-01-01",
        ));
        assert_eq!(found.value, StatusValue::Category(Category::Done));
        assert!(!found.exact);
    }

    #[test]
    fn the_longest_term_wins() {
        let found = found(status(Kind::Task, "🟠 commencée puis enterrée"));
        assert_eq!(found.value, StatusValue::Category(Category::Dropped));
    }

    #[test]
    fn an_open_backlog_item_stays_in_the_backlog() {
        let found = found(status(Kind::Backlog, "ouvert"));
        assert_eq!(found.value, StatusValue::Category(Category::Backlog));
    }

    #[test]
    fn reflexions_read_open_or_settled() {
        assert_eq!(
            found(status(Kind::Reflexion, "🟡 ouverte")).value,
            StatusValue::Open
        );
        let partial = found(status(
            Kind::Reflexion,
            "partiellement tranchée le 2030-01-01",
        ));
        assert_eq!(partial.value, StatusValue::Open);
        assert!(partial.inferred);
    }

    #[test]
    fn a_reflexion_on_trial_reads_trial() {
        let trial = found(status(Kind::Reflexion, "🟡 en essai — la forme tient"));
        assert_eq!(trial.value, StatusValue::Trial);
        assert!(!trial.inferred);
        assert_eq!(
            found(status(Kind::Reflexion, "à l'essai")).value,
            StatusValue::Trial
        );
    }

    #[test]
    fn an_emoji_alone_decides_nothing() {
        assert_eq!(status(Kind::Task, "🔴"), Reading::EmojiOnly);
        assert_eq!(priority("🔴"), Reading::EmojiOnly);
    }

    #[test]
    fn a_word_prefix_is_not_a_term() {
        assert!(matches!(
            status(Kind::Task, "faits divers"),
            Reading::Unknown(_)
        ));
    }

    #[test]
    fn priorities_map_to_three_levels() {
        assert_eq!(found(priority("🟢 basse")).value, "low");
        assert_eq!(found(priority("🟡 moyenne")).value, "medium");
        let mid_high = found(priority("🟠 moyenne-haute — raison"));
        assert_eq!(mid_high.value, "medium");
        assert!(mid_high.inferred && !mid_high.exact);
        assert_eq!(found(priority("🔴 ABSOLUE")).value, "high");
        assert!(matches!(priority("à cadrer"), Reading::Unknown(_)));
    }
}
