//! The banner of `ariane --help`: the interface's logo drawn in text.
//!
//! The spool is Lucide's (ISC), as in the interface's header and favicon,
//! rasterized from its SVG paths into half blocks (`▀ ▄ █`), each character
//! being two square pixels stacked. The word beside it is drawn in square
//! pixels of two full blocks. The spool takes the content's theme color
//! (`theme.primary` of `settings.toml`), the default one outside a content
//! repository; `clap` drops the colors when the output is not a terminal or
//! `NO_COLOR` is set.

use crate::settings::DEFAULT_COLOR;

/// The spool, eleven lines.
const SPOOL: [&str; 11] = [
    " ▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄",
    "██▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀██",
    "▀██▄           ▄▄██▀",
    " ▀▀██▄  ▄▄▄█████▀▀▀",
    "    ████▀▀▀▀  ██",
    "    ██        ██",
    "    ██  ▄▄▄▄████",
    " ▄▄▄█████▀▀▀  ▀██▄▄",
    "▄██▀▀           ▀██▄",
    "██▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄▄██",
    " ▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀▀",
];

/// “Ariane”, set beside the spool from its third line.
const WORD: [&str; 7] = [
    "  ██████              ██",
    "██      ██",
    "██      ██  ██  ████  ██    ████    ██████      ████",
    "██████████  ████      ██        ██  ██    ██  ██    ██",
    "██      ██  ██        ██    ██████  ██    ██  ████████",
    "██      ██  ██        ██  ██    ██  ██    ██  ██",
    "██      ██  ██        ██    ██████  ██    ██    ██████",
];

/// The line of the spool where the word starts.
const WORD_LINE: usize = 2;

/// The tagline, beside the last line of the spool.
const TAGLINE: &str = "Ariadne's thread through your projects";

/// The width the spool takes, and the space after it.
const SPOOL_WIDTH: usize = 20;
const GAP: &str = "   ";

/// The banner, the spool in `color` (`#rrggbb`; the default color when it is
/// not one), the word in bold and the tagline dimmed.
pub fn banner(color: &str) -> String {
    let (red, green, blue) = rgb(color)
        .or_else(|| rgb(DEFAULT_COLOR))
        .unwrap_or_default();
    let spool = format!("\x1b[1;38;2;{red};{green};{blue}m");
    let reset = "\x1b[0m";
    SPOOL
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let pad = " ".repeat(SPOOL_WIDTH - line.chars().count());
            let beside = match index.checked_sub(WORD_LINE).and_then(|row| WORD.get(row)) {
                Some(word) => format!("\x1b[1m{word}{reset}"),
                None if index == SPOOL.len() - 1 => format!("\x1b[2m{TAGLINE}{reset}"),
                None => String::new(),
            };
            format!("{spool}{line}{reset}{pad}{GAP}{beside}")
                .trim_end_matches(' ')
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The red, green and blue of a color written `#rrggbb`.
fn rgb(color: &str) -> Option<(u8, u8, u8)> {
    let hex = color.strip_prefix('#').filter(|hex| hex.len() == 6)?;
    let channel = |at: usize| u8::from_str_radix(hex.get(at..at + 2)?, 16).ok();
    Some((channel(0)?, channel(2)?, channel(4)?))
}

/// The theme color of the content repository found from here, the default
/// color when there is none or its settings cannot be read.
pub fn theme_color() -> String {
    let explicit = std::env::var_os("ARIANE_REPO").map(std::path::PathBuf::from);
    crate::repo::discover(explicit.as_deref())
        .and_then(|location| crate::settings::load(&location.root))
        .map(|settings| settings.theme.primary)
        .unwrap_or_else(|_| DEFAULT_COLOR.to_owned())
}

/// Unit tests of the banner.
#[cfg(test)]
mod tests {
    use super::*;

    /// The banner as a terminal without colors shows it.
    fn plain(text: &str) -> String {
        let mut plain = String::new();
        let mut escape = false;
        for character in text.chars() {
            match (escape, character) {
                (false, '\x1b') => escape = true,
                (true, 'm') => escape = false,
                (true, _) => {}
                (false, character) => plain.push(character),
            }
        }
        plain
    }

    #[test]
    fn the_banner_fits_in_eighty_columns_with_the_word_beside_the_spool() {
        let banner = plain(&banner("#3760c9"));
        let lines: Vec<&str> = banner.lines().collect();
        assert_eq!(lines.len(), SPOOL.len());
        assert!(lines.iter().all(|line| line.chars().count() <= 80));
        assert!(lines[6].starts_with("    ██  ▄▄▄▄████"));
        assert!(lines[6].ends_with("██    ██  ████████"));
        assert!(lines[10].ends_with(TAGLINE));
        assert!(lines.iter().all(|line| !line.ends_with(' ')));
    }

    #[test]
    fn the_spool_takes_the_theme_color_or_the_default_one() {
        assert!(banner("#AA0010").contains("\x1b[1;38;2;170;0;16m"));
        assert!(banner("red").contains("\x1b[1;38;2;55;96;201m"));
        assert_eq!(rgb("#3760c9"), Some((55, 96, 201)));
        assert_eq!(rgb("#3760c"), None);
        assert_eq!(rgb("#zz60c9"), None);
    }
}
