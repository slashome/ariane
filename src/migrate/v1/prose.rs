use yaml_serde::Value;

use crate::item::Kind;
use crate::node::Category;

use super::vocabulary::{self, Reading};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusValue {
    Category(Category),
    Open,
    Settled,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Extraction {
    pub status: Option<StatusValue>,
    pub fields: Vec<(String, Value)>,
    pub inferred: Vec<String>,
    pub leftovers: Vec<String>,
    pub body: String,
}

pub fn extract(kind: Kind, body: &str) -> Extraction {
    let lines: Vec<&str> = body.split_inclusive('\n').collect();
    let Some(region) = header_region(&lines) else {
        return unchanged(body);
    };
    let mut reader = Reader::new(kind);
    let mut edits: Vec<Edit> = vec![Edit::Keep; lines.len()];
    for index in region.clone() {
        let line = lines[index];
        let text = line.trim_end_matches(['\r', '\n']);
        edits[index] = if let Some(quoted) = text.strip_prefix('>') {
            reader.quote_line(quoted)
        } else if is_bullet(text) {
            reader.bullet_line(text)
        } else {
            Edit::Keep
        };
    }
    if reader.is_empty() {
        return unchanged(body);
    }
    reader.finish(rebuild(&lines, &edits, region))
}

fn unchanged(body: &str) -> Extraction {
    Extraction {
        body: body.to_owned(),
        ..Extraction::default()
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Edit {
    Keep,
    Replace(String),
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    Banner,
    Field,
}

struct Reader {
    kind: Kind,
    status: Option<(StatusValue, Source, bool)>,
    priority: Option<(&'static str, bool)>,
    created: Option<String>,
    external: Vec<String>,
    leftovers: Vec<String>,
}

impl Reader {
    fn new(kind: Kind) -> Reader {
        Reader {
            kind,
            status: None,
            priority: None,
            created: None,
            external: Vec::new(),
            leftovers: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.status.is_none()
            && self.priority.is_none()
            && self.created.is_none()
            && self.external.is_empty()
            && self.leftovers.is_empty()
    }

    fn bullet_line(&mut self, text: &str) -> Edit {
        let pairs = bullet_pairs(text);
        if pairs.is_empty() {
            return Edit::Keep;
        }
        let kept: Vec<&(String, String)> = pairs
            .iter()
            .filter(|(key, value)| !self.field(key, value))
            .collect();
        if kept.len() == pairs.len() {
            return Edit::Keep;
        }
        if kept.is_empty() {
            return Edit::Remove;
        }
        let rebuilt = kept
            .iter()
            .map(|(key, value)| format!("**{key}** : {value}"))
            .collect::<Vec<_>>()
            .join(" · ");
        Edit::Replace(format!("- {rebuilt}"))
    }

    fn quote_line(&mut self, quoted: &str) -> Edit {
        let content = quoted.trim();
        if content.is_empty() {
            return Edit::Keep;
        }
        if is_banner(content) {
            self.banner(content);
            return Edit::Keep;
        }
        let pieces = split_segments(content);
        let mut consumed = vec![false; pieces.len()];
        for (index, piece) in pieces.iter().enumerate() {
            consumed[index] = self.segment(&piece.text);
        }
        if !consumed.iter().any(|done| *done) {
            return Edit::Keep;
        }
        let kept: Vec<&Piece> = pieces
            .iter()
            .zip(&consumed)
            .filter(|(_, done)| !**done)
            .map(|(piece, _)| piece)
            .collect();
        if kept.is_empty() {
            return Edit::Remove;
        }
        let rebuilt = join_pieces(&kept);
        if !rebuilt.matches("**").count().is_multiple_of(2) {
            return Edit::Keep;
        }
        Edit::Replace(format!("> {rebuilt}"))
    }

    fn segment(&mut self, text: &str) -> bool {
        let plain = text.replace("**", "").replace('`', "");
        let plain = plain.trim();
        let lowered = plain.to_lowercase();
        if let Some(marker) = CREATED_MARKERS
            .iter()
            .find(|marker| lowered.starts_with(**marker))
        {
            return self.created_segment(plain, marker.chars().count());
        }
        if let Some((key, value)) = plain.split_once(':') {
            let key = key.trim();
            if !key.is_empty() && key.chars().count() <= 24 {
                return self.field(key, value.trim());
            }
        }
        self.monday_mentions(plain);
        false
    }

    fn created_segment(&mut self, plain: &str, marker_chars: usize) -> bool {
        let rest: String = plain.chars().skip(marker_chars).collect();
        let Some(date) = find_date(&rest) else {
            return false;
        };
        self.created.get_or_insert(date.clone());
        let remainder = rest.replacen(&date, "", 1);
        remainder
            .trim_matches(|c: char| c.is_whitespace() || ":.,".contains(c))
            .is_empty()
    }

    fn field(&mut self, key: &str, value: &str) -> bool {
        let name = vocabulary::normalize(key);
        let name = name.trim_matches(|c: char| c == ':' || c.is_whitespace());
        match name {
            "état" | "etat" | "statut" | "status" => self.status_field(value),
            "priorité" | "priorite" | "priority" => self.priority_field(value),
            "monday" => self.monday_field(value),
            "créée" | "créé" | "créée le" | "créé le" | "date" => {
                self.created_segment(value, 0)
            }
            "estimation" => {
                self.leftovers.push(format!(
                    "estimation {value}: story points are not a size, set `size` by hand"
                ));
                false
            }
            "dépend de" | "depend de" | "parent" | "story parente" => {
                self.leftovers.push(format!(
                    "{name}: {value} cannot be resolved to an item id automatically"
                ));
                false
            }
            _ => {
                self.monday_mentions(value);
                false
            }
        }
    }

    fn status_field(&mut self, value: &str) -> bool {
        if self.kind == Kind::Ressource {
            self.leftovers
                .push("a status was written on a ressource and is ignored".to_owned());
            return false;
        }
        match vocabulary::status(self.kind, value) {
            Reading::Found(found) => {
                self.set_status(found.value, Source::Field, found.inferred);
                found.exact
            }
            Reading::EmojiOnly => {
                self.leftovers.push(format!(
                    "status {value:?} is an emoji alone, which is ambiguous"
                ));
                false
            }
            Reading::Unknown(text) => {
                self.leftovers
                    .push(format!("status {text:?} matches no known value"));
                false
            }
        }
    }

    fn priority_field(&mut self, value: &str) -> bool {
        match vocabulary::priority(value) {
            Reading::Found(found) => {
                if self.priority.is_none() {
                    self.priority = Some((found.value, found.inferred));
                }
                found.exact
            }
            Reading::EmojiOnly => {
                self.leftovers.push(format!(
                    "priority {value:?} is an emoji alone, which is ambiguous"
                ));
                false
            }
            Reading::Unknown(text) => {
                self.leftovers.push(format!(
                    "priority {text:?} has no equivalent in low/medium/high"
                ));
                false
            }
        }
    }

    fn monday_field(&mut self, value: &str) -> bool {
        let ids = ticket_ids(value);
        if ids.is_empty() {
            return false;
        }
        for id in ids {
            self.add_external(id);
        }
        let without_link = value
            .replace(|c: char| c.is_ascii_digit() || "[]()".contains(c), "")
            .replace("https://", "");
        without_link
            .split_whitespace()
            .all(|word| word.contains(".monday.com") || word.contains('/'))
    }

    fn monday_mentions(&mut self, text: &str) {
        if !text.to_lowercase().contains("monday") {
            return;
        }
        for id in ticket_ids(text) {
            self.add_external(id);
        }
    }

    fn add_external(&mut self, id: String) {
        let value = format!("monday:{id}");
        if !self.external.contains(&value) {
            self.external.push(value);
        }
    }

    fn banner(&mut self, content: &str) {
        let lowered = vocabulary::normalize(content);
        let category = if lowered.contains("abandonn") || lowered.contains("enterr") {
            Category::Dropped
        } else if lowered.contains("terminé")
            || lowered.contains("archivé")
            || lowered.contains("livré")
        {
            Category::Done
        } else {
            return;
        };
        let value = match self.kind {
            Kind::Reflexion => StatusValue::Settled,
            Kind::Ressource => return,
            _ => StatusValue::Category(category),
        };
        self.set_status(value, Source::Banner, false);
    }

    fn set_status(&mut self, value: StatusValue, source: Source, inferred: bool) {
        match self.status {
            None => self.status = Some((value, source, inferred)),
            Some((current, current_source, _)) if current != value => {
                if source == Source::Banner && current_source == Source::Field {
                    self.status = Some((value, source, inferred));
                }
                self.leftovers.push(
                    "two statuses disagree; the archive banner wins over the older status line"
                        .to_owned(),
                );
            }
            Some(_) => {}
        }
    }

    fn finish(self, body: String) -> Extraction {
        let mut fields = Vec::new();
        let mut inferred = Vec::new();
        if let Some(created) = self.created {
            fields.push(("created".to_owned(), Value::String(created)));
        }
        if let Some((priority, guessed)) = self.priority {
            fields.push(("priority".to_owned(), Value::String(priority.to_owned())));
            if guessed {
                inferred.push("priority".to_owned());
            }
        }
        if !self.external.is_empty() {
            let list = self.external.into_iter().map(Value::String).collect();
            fields.push(("external".to_owned(), Value::Sequence(list)));
        }
        let status = self.status.map(|(value, _, guessed)| {
            if guessed {
                inferred.push("status".to_owned());
            }
            value
        });
        Extraction {
            status,
            fields,
            inferred,
            leftovers: self.leftovers,
            body,
        }
    }
}

const CREATED_MARKERS: [&str; 8] = [
    "créée le",
    "créé le",
    "créée",
    "créé",
    "ouverte le",
    "écrite le",
    "versée le",
    "extrait le",
];

#[derive(Debug, Clone, PartialEq)]
struct Piece {
    separator: &'static str,
    text: String,
}

fn split_segments(content: &str) -> Vec<Piece> {
    let mut pieces = Vec::new();
    let mut separator = "";
    let mut start = 0;
    let mut index = 0;
    let bytes = content.as_bytes();
    while index < content.len() {
        let rest = &content[index..];
        let found = [" — ", " · "]
            .into_iter()
            .find(|candidate| rest.starts_with(candidate))
            .map(|candidate| (candidate, candidate.len(), 0))
            .or_else(|| sentence_end(rest).then_some((" ", 2, 1)));
        if let Some((next, width, keep)) = found {
            pieces.push(Piece {
                separator,
                text: content[start..index + keep].to_owned(),
            });
            separator = next;
            index += width;
            start = index;
            continue;
        }
        index += 1;
        while index < bytes.len() && !content.is_char_boundary(index) {
            index += 1;
        }
    }
    pieces.push(Piece {
        separator,
        text: content[start..].to_owned(),
    });
    pieces.retain(|piece| !piece.text.trim().is_empty());
    pieces
}

fn sentence_end(rest: &str) -> bool {
    let Some(after) = rest.strip_prefix(". ") else {
        return false;
    };
    after
        .chars()
        .next()
        .is_some_and(|c| c.is_uppercase() || c == '*')
}

fn join_pieces(pieces: &[&Piece]) -> String {
    let mut text = String::new();
    for (index, piece) in pieces.iter().enumerate() {
        if index > 0 {
            text.push_str(piece.separator);
        }
        text.push_str(piece.text.trim());
    }
    text
}

fn header_region(lines: &[&str]) -> Option<std::ops::Range<usize>> {
    let title = lines
        .iter()
        .position(|line| line.starts_with("# "))
        .or_else(|| lines.iter().position(|line| !line.trim().is_empty()))?;
    let start = title + 1;
    let mut end = start;
    let mut seen = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        let text = line.trim_end_matches(['\r', '\n']);
        if text.trim().is_empty() {
            continue;
        }
        if text.starts_with('>') || is_bullet(text) {
            seen = true;
            end = index + 1;
        } else {
            break;
        }
    }
    seen.then_some(start..end)
}

fn is_bullet(text: &str) -> bool {
    text.strip_prefix("- **").is_some_and(|rest| {
        rest.split_once("**").is_some_and(|(_, after)| {
            after.trim_start().starts_with(':') || after.trim_start().is_empty()
        })
    }) || text
        .strip_prefix("- **")
        .is_some_and(|rest| rest.split_once(":**").is_some())
}

fn bullet_pairs(text: &str) -> Vec<(String, String)> {
    let Some(rest) = text.strip_prefix("- ") else {
        return Vec::new();
    };
    rest.split(" · **")
        .enumerate()
        .filter_map(|(index, part)| {
            let part = if index == 0 {
                part.strip_prefix("**")?
            } else {
                part
            };
            let (key, value) = part.split_once("**")?;
            let key = key.trim().trim_end_matches(':').trim().to_owned();
            let value = value.trim().trim_start_matches(':').trim().to_owned();
            (!key.is_empty()).then_some((key, value))
        })
        .collect()
}

fn is_banner(content: &str) -> bool {
    let upper = content.to_uppercase();
    content.starts_with('✅')
        || content.starts_with('❌')
        || (content.contains("**") && (content.contains("ARCHIVÉ") || content.contains("TERMINÉ")))
        || upper.starts_with("**ARCHIVÉ")
}

fn find_date(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    chars.windows(10).find_map(|window| {
        let candidate: String = window.iter().collect();
        is_date(&candidate).then_some(candidate)
    })
}

fn is_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return false;
    }
    let digits = |range: std::ops::Range<usize>| bytes[range].iter().all(u8::is_ascii_digit);
    if !(digits(0..4) && digits(5..7) && digits(8..10)) {
        return false;
    }
    let month: u32 = text[5..7].parse().unwrap_or(0);
    let day: u32 = text[8..10].parse().unwrap_or(0);
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

fn ticket_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            run.push(c);
            continue;
        }
        if run.len() >= 9 && !ids.contains(&run) {
            ids.push(run.clone());
        }
        run.clear();
    }
    ids
}

fn rebuild(lines: &[&str], edits: &[Edit], region: std::ops::Range<usize>) -> String {
    let head = lines[..region.start].concat();
    let mut middle: Vec<String> = region
        .clone()
        .filter_map(|index| {
            let line = lines[index];
            let ending = &line[line.trim_end_matches(['\r', '\n']).len()..];
            match &edits[index] {
                Edit::Keep => Some(line.to_owned()),
                Edit::Remove => None,
                Edit::Replace(text) => Some(format!("{text}{ending}")),
            }
        })
        .collect();
    let tail = lines[region.end..].concat();
    tidy(&mut middle);
    while middle.last().is_some_and(|line| line.trim().is_empty())
        && (tail.is_empty() || tail.starts_with('\n') || tail.starts_with("\r\n"))
    {
        middle.pop();
    }
    format!("{head}{}{tail}", middle.concat())
}

fn tidy(lines: &mut Vec<String>) {
    let is_quote_text = |line: &str| line.starts_with('>') && line.trim() != ">";
    let mut kept: Vec<String> = Vec::with_capacity(lines.len());
    for index in 0..lines.len() {
        let line = &lines[index];
        if line.trim() == ">" {
            let before = kept.last().is_some_and(|previous| is_quote_text(previous));
            let after = lines[index + 1..]
                .iter()
                .find(|next| next.trim() != ">")
                .is_some_and(|next| is_quote_text(next));
            if !(before && after) {
                continue;
            }
        }
        if line.trim().is_empty()
            && kept
                .last()
                .is_some_and(|previous| previous.trim().is_empty())
        {
            continue;
        }
        kept.push(line.clone());
    }
    *lines = kept;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field<'a>(extraction: &'a Extraction, key: &str) -> Option<&'a Value> {
        extraction
            .fields
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    fn text_field(extraction: &Extraction, key: &str) -> Option<String> {
        field(extraction, key).and_then(|value| value.as_str().map(str::to_owned))
    }

    fn external(extraction: &Extraction) -> Vec<String> {
        field(extraction, "external")
            .and_then(Value::as_sequence)
            .map(|list| {
                list.iter()
                    .filter_map(|value| value.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn a_bullet_header_yields_status_and_priority_and_keeps_the_other_bullets() {
        let body = "# Ranger les boîtes\n\n- **Catégorie** : rangement\n- **Priorité** : 🟡 moyenne\n- **Cible** : le grenier\n- **État** : ouvert\n\n## Suite\n\nTexte.\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(
            extraction.status,
            Some(StatusValue::Category(Category::Todo))
        );
        assert_eq!(
            text_field(&extraction, "priority").as_deref(),
            Some("medium")
        );
        assert_eq!(
            extraction.body,
            "# Ranger les boîtes\n\n- **Catégorie** : rangement\n- **Cible** : le grenier\n\n## Suite\n\nTexte.\n"
        );
    }

    #[test]
    fn a_bullet_whose_value_carries_an_explanation_stays_in_the_body() {
        let body = "# Ranger\n\n- **Priorité** : 🟢 basse — rien ne presse\n- **État** : en attente (personne de libre)\n\nTexte.\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(text_field(&extraction, "priority").as_deref(), Some("low"));
        assert_eq!(
            extraction.status,
            Some(StatusValue::Category(Category::Backlog))
        );
        assert_eq!(extraction.inferred, vec!["status".to_owned()]);
        assert_eq!(extraction.body, body);
    }

    #[test]
    fn a_story_with_ticket_bullets_yields_its_ticket_and_notes_the_estimation() {
        let body = "# Story 9 — Repeindre\n\n- **Monday** : [1000000001](https://example.monday.com/boards/1/pulses/1000000001)\n- **Epic** : Travaux\n- **Sprint** : 12 · **Estimation** : 5 · **Type** : Fonctionnalité\n- **Owner** : Une Personne\n- **Statut** : ✅ **LIVRÉE** — fini le 2030-02-01\n\n## Contexte\n";
        let extraction = extract(Kind::Story, body);
        assert_eq!(external(&extraction), vec!["monday:1000000001"]);
        assert_eq!(
            extraction.status,
            Some(StatusValue::Category(Category::Done))
        );
        assert!(
            extraction
                .leftovers
                .iter()
                .any(|note| note.contains("estimation 5"))
        );
        assert!(!extraction.body.contains("**Monday**"));
        assert!(
            extraction
                .body
                .contains("**Sprint** : 12 · **Estimation** : 5")
        );
        assert!(extraction.body.contains("**Owner** : Une Personne"));
    }

    #[test]
    fn a_created_quote_with_a_dash_separator_is_read_and_removed() {
        let body = "# Tâche\n\n> Créée : 2030-03-04 — statut : ⏳ à faire\n\n## Détail\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(
            text_field(&extraction, "created").as_deref(),
            Some("2030-03-04")
        );
        assert_eq!(
            extraction.status,
            Some(StatusValue::Category(Category::Todo))
        );
        assert_eq!(extraction.body, "# Tâche\n\n## Détail\n");
    }

    #[test]
    fn a_created_quote_with_a_dot_separator_keeps_its_other_facts() {
        let body =
            "# Tâche\n\n> Créé : 2030-03-04 · Monday : à créer · Branche : `feat/x`\n\nTexte.\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(
            text_field(&extraction, "created").as_deref(),
            Some("2030-03-04")
        );
        assert!(external(&extraction).is_empty());
        assert_eq!(
            extraction.body,
            "# Tâche\n\n> Monday : à créer · Branche : `feat/x`\n\nTexte.\n"
        );
    }

    #[test]
    fn a_priority_quote_to_be_framed_has_no_priority_and_a_leftover() {
        let body =
            "# Doc\n\n> Priorité : à cadrer.\n> Origine : demandée le 2030-01-02.\n\n## Décision\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(text_field(&extraction, "priority"), None);
        assert!(
            extraction
                .leftovers
                .iter()
                .any(|note| note.contains("à cadrer"))
        );
        assert_eq!(extraction.body, body);
    }

    #[test]
    fn an_origin_quote_alone_gives_no_status_and_an_extraction_date() {
        let body = "# Note\n\n> Origine : une relecture (2030-01-05).\n\n## Constat\n";
        let extraction = extract(Kind::Reflexion, body);
        assert_eq!(extraction.status, None);
        assert_eq!(extraction.body, body);
    }

    #[test]
    fn a_reflexion_quote_yields_its_opening_date_and_open_status() {
        let body =
            "# Une question\n\n> Ouverte le 2030-04-05. **Statut : 🔴 ouverte**\n\n## Contexte\n";
        let extraction = extract(Kind::Reflexion, body);
        assert_eq!(
            text_field(&extraction, "created").as_deref(),
            Some("2030-04-05")
        );
        assert_eq!(extraction.status, Some(StatusValue::Open));
        assert_eq!(extraction.body, "# Une question\n\n## Contexte\n");
    }

    #[test]
    fn a_ressource_quote_yields_its_date_and_no_status() {
        let body = "# Un site\n\n> Versée le 2030-05-06. **Ressource, pas une décision** : à lire.\n\n## Ce que c'est\n";
        let extraction = extract(Kind::Ressource, body);
        assert_eq!(
            text_field(&extraction, "created").as_deref(),
            Some("2030-05-06")
        );
        assert_eq!(extraction.status, None);
        assert_eq!(
            extraction.body,
            "# Un site\n\n> **Ressource, pas une décision** : à lire.\n\n## Ce que c'est\n"
        );
    }

    #[test]
    fn an_archive_banner_wins_over_an_older_status_line() {
        let body = "# Lot 3\n\n> ✅ **TERMINÉ ET ARCHIVÉ** (2030-06-07) — remplacé par une décision.\n\n> Créée : 2030-06-01 — statut : ⏳ à faire\n> Suite du lot 2, ticket monday `1000000002`\n\n## Issue\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(
            extraction.status,
            Some(StatusValue::Category(Category::Done))
        );
        assert_eq!(
            text_field(&extraction, "created").as_deref(),
            Some("2030-06-01")
        );
        assert_eq!(external(&extraction), vec!["monday:1000000002"]);
        assert!(
            extraction
                .leftovers
                .iter()
                .any(|note| note.contains("archive banner wins"))
        );
        assert!(extraction.body.contains("TERMINÉ ET ARCHIVÉ"));
        assert!(!extraction.body.contains("statut"));
        assert!(extraction.body.contains("ticket monday `1000000002`"));
    }

    #[test]
    fn an_emoji_alone_sets_no_status() {
        let body = "# Tâche\n\n- **État** : 🔴\n\nTexte.\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(extraction.status, None);
        assert!(
            extraction
                .leftovers
                .iter()
                .any(|note| note.contains("emoji alone"))
        );
        assert_eq!(extraction.body, body);
    }

    #[test]
    fn several_ticket_forms_collapse_into_one_external_list() {
        let body = "# Tâche\n\n- **Monday** : 1000000003\n- **Lien** : voir le ticket monday https://example.monday.com/boards/9/pulses/1000000004\n\nTexte.\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(
            external(&extraction),
            vec!["monday:1000000003", "monday:1000000004"]
        );
        assert!(!extraction.body.contains("**Monday** : 1000000003"));
        assert!(extraction.body.contains("**Lien**"));
    }

    #[test]
    fn dependencies_are_left_to_a_human() {
        let body = "# Tâche\n\n- **Dépend de** : la tâche des boîtes\n\nTexte.\n";
        let extraction = extract(Kind::Task, body);
        assert!(
            extraction
                .leftovers
                .iter()
                .any(|note| note.contains("cannot be resolved"))
        );
        assert_eq!(extraction.body, body);
    }

    #[test]
    fn a_body_without_a_known_header_is_returned_untouched() {
        let body = "# Titre\n\nUn paragraphe ordinaire.\n\n- une liste\n";
        let extraction = extract(Kind::Task, body);
        assert_eq!(extraction, unchanged(body));
    }

    #[test]
    fn the_rest_of_the_body_is_preserved_byte_for_byte() {
        let tail =
            "## Partie\n\n> Une citation plus bas, statut : à faire.\n\n- **Gras** : non lu\n";
        let body = format!("# T\n\n- **État** : fait\n\n{tail}");
        let extraction = extract(Kind::Task, &body);
        assert_eq!(
            extraction.status,
            Some(StatusValue::Category(Category::Done))
        );
        assert_eq!(extraction.body, format!("# T\n\n{tail}"));
    }
}
