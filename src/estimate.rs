//! Estimates in hours of focused work (SPEC.md §1.5, ADR-0025).
//!
//! An item keeps its `size`, `XS` to `XL`; the node says what each size costs.
//! The `[estimates]` table of a `node.toml` gives the hours of the five sizes,
//! the `default` size of an item without one, the `test_ratio` of testing added
//! after each piece of work, and who tests (`tester`). It is inherited key by
//! key, the nearest declaration winning, over the values of ADR-0025.

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

use crate::schema::SIZES;

/// Hours of `XS`, `S`, `M`, `L` and `XL` when no node declares them.
pub const DEFAULT_HOURS: [f64; 5] = [2.0, 4.0, 12.0, 24.0, 48.0];
/// The size of an unsized item when no node declares one.
pub const DEFAULT_SIZE: &str = "M";
/// The share of testing added after each piece of work when no node declares one.
pub const DEFAULT_TEST_RATIO: f64 = 0.25;

/// Who tests a piece of work once it is done (ADR-0025 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tester {
    /// The owner tests: the testing consumes the owner's hours.
    #[default]
    Owner,
    /// Someone else validates: the testing is a delay, not the owner's hours.
    Others,
}

impl Tester {
    /// The value written in `node.toml` and JSON.
    pub fn id(self) -> &'static str {
        match self {
            Tester::Owner => "owner",
            Tester::Others => "others",
        }
    }
}

/// Every value `tester` accepts.
pub const TESTERS: [Tester; 2] = [Tester::Owner, Tester::Others];

/// The `[estimates]` table of one `node.toml`, as written: every key optional,
/// each one overriding the inherited value. `deny_unknown_fields` refuses a key
/// that is neither one of the five sizes nor a known key, such as `XXL`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Declaration {
    /// Hours of an `XS`.
    #[serde(rename = "XS", default, deserialize_with = "hours")]
    pub xs: Option<f64>,
    /// Hours of an `S`.
    #[serde(rename = "S", default, deserialize_with = "hours")]
    pub s: Option<f64>,
    /// Hours of an `M`.
    #[serde(rename = "M", default, deserialize_with = "hours")]
    pub m: Option<f64>,
    /// Hours of an `L`.
    #[serde(rename = "L", default, deserialize_with = "hours")]
    pub l: Option<f64>,
    /// Hours of an `XL`.
    #[serde(rename = "XL", default, deserialize_with = "hours")]
    pub xl: Option<f64>,
    /// The size an item without `size` counts as.
    #[serde(default, deserialize_with = "size")]
    pub default: Option<String>,
    /// Hours of testing added per hour of work.
    #[serde(default, deserialize_with = "ratio")]
    pub test_ratio: Option<f64>,
    /// Who tests.
    #[serde(default)]
    pub tester: Option<Tester>,
}

impl Declaration {
    /// The hours this table declares for the five sizes, smallest first.
    fn hours(&self) -> [Option<f64>; 5] {
        [self.xs, self.s, self.m, self.l, self.xl]
    }
}

/// Reads a number of TOML, integer or float, refusing anything else.
fn number<'de, D: Deserializer<'de>>(deserializer: D, key: &str) -> Result<f64, D::Error> {
    match toml::Value::deserialize(deserializer)? {
        toml::Value::Integer(value) => Ok(value as f64),
        toml::Value::Float(value) if value.is_finite() => Ok(value),
        other => Err(D::Error::custom(format!(
            "`{key} = {other}` is not a number"
        ))),
    }
}

/// Reads the hours of a size: a number above zero.
fn hours<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    let value = number(deserializer, "hours")?;
    if value <= 0.0 {
        return Err(D::Error::custom(format!(
            "hours must be above zero, not {value}"
        )));
    }
    Ok(Some(value))
}

/// Reads `test_ratio`: a number, zero or above.
fn ratio<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f64>, D::Error> {
    let value = number(deserializer, "test_ratio")?;
    if value < 0.0 {
        return Err(D::Error::custom(format!(
            "`test_ratio = {value}` is negative"
        )));
    }
    Ok(Some(value))
}

/// Reads `default`: one of the five sizes.
fn size<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    let text = String::deserialize(deserializer)?;
    if known_size(&text).is_none() {
        return Err(D::Error::custom(format!(
            "`default = \"{text}\"` is not one of the sizes {}",
            SIZES.join(", ")
        )));
    }
    Ok(Some(text))
}

/// The size `text` names, as the schema spells it; `None` when it is not one
/// of the five.
pub fn known_size(text: &str) -> Option<&'static str> {
    SIZES.iter().find(|size| **size == text).copied()
}

/// What sizes cost on one node, once the `[estimates]` tables of its ancestors
/// are combined key by key.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Pricing {
    /// Hours of an `XS`.
    #[serde(rename = "XS")]
    pub xs: f64,
    /// Hours of an `S`.
    #[serde(rename = "S")]
    pub s: f64,
    /// Hours of an `M`.
    #[serde(rename = "M")]
    pub m: f64,
    /// Hours of an `L`.
    #[serde(rename = "L")]
    pub l: f64,
    /// Hours of an `XL`.
    #[serde(rename = "XL")]
    pub xl: f64,
    /// The size an item without `size` counts as.
    pub default: &'static str,
    /// Hours of testing added per hour of work.
    pub test_ratio: f64,
    /// Who tests.
    pub tester: Tester,
}

impl Default for Pricing {
    /// The values of ADR-0025, for a node with no `[estimates]` above it.
    fn default() -> Pricing {
        let [xs, s, m, l, xl] = DEFAULT_HOURS;
        Pricing {
            xs,
            s,
            m,
            l,
            xl,
            default: DEFAULT_SIZE,
            test_ratio: DEFAULT_TEST_RATIO,
            tester: Tester::Owner,
        }
    }
}

impl Pricing {
    /// Overrides the keys `declaration` gives, keeping the others.
    pub fn apply(&mut self, declaration: &Declaration) {
        let declared = declaration.hours();
        let slots = [
            &mut self.xs,
            &mut self.s,
            &mut self.m,
            &mut self.l,
            &mut self.xl,
        ];
        for (slot, value) in slots.into_iter().zip(declared) {
            if let Some(value) = value {
                *slot = value;
            }
        }
        if let Some(default) = declaration.default.as_deref().and_then(known_size) {
            self.default = default;
        }
        if let Some(ratio) = declaration.test_ratio {
            self.test_ratio = ratio;
        }
        if let Some(tester) = declaration.tester {
            self.tester = tester;
        }
    }

    /// The hours of `size`, `None` when it is not one of the five sizes.
    pub fn hours_of(&self, size: &str) -> Option<f64> {
        match size {
            "XS" => Some(self.xs),
            "S" => Some(self.s),
            "M" => Some(self.m),
            "L" => Some(self.l),
            "XL" => Some(self.xl),
            _ => None,
        }
    }

    /// The hours of the default size.
    pub fn default_hours(&self) -> f64 {
        self.hours_of(self.default).unwrap_or(self.m)
    }
}
