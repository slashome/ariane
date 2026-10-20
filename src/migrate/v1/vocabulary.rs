use crate::item::Kind;
use crate::node::Category;

use super::prose::StatusValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match<T> {
    pub value: T,
    pub inferred: bool,
    pub exact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reading<T> {
    Found(Match<T>),
    EmojiOnly,
    Unknown(String),
}

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

const REFLEXION_STATUSES: &[(&str, StatusValue, bool)] = &[
    ("partiellement tranchée", StatusValue::Open, true),
    ("partiellement tranché", StatusValue::Open, true),
    ("ouverte", StatusValue::Open, false),
    ("ouvert", StatusValue::Open, false),
    ("en essai", StatusValue::Open, true),
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

pub fn priority(raw: &str) -> Reading<&'static str> {
    lookup(raw, PRIORITIES)
}

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

fn backlog_when_open(kind: Kind, category: Category) -> Category {
    match (kind, category) {
        (Kind::Backlog, Category::Todo) => Category::Backlog,
        _ => category,
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

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
