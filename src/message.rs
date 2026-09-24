//! The `PREFIX|field|field` text messages our games pass between rules and UI.
//!
//! Rules send status to the UI with `send_lua_ui_msg`; the UI sends input to
//! the rules with `send_lua_rules_msg`. Fields are either positional
//! (`MOVE|12.5|300`) or `key=value` pairs (`STATUS|health=40|gameEnd=0`).
//! Parsing is strict about numbers: a non-finite float never reaches synced
//! code.

use alloc::string::String;
use core::cell::RefCell;
use core::fmt::{Display, Write};

/// The positional fields of `message` after `prefix|`, or `None` when the
/// prefix doesn't match.
pub fn fields<'a>(message: &'a str, prefix: &str) -> Option<Fields<'a>> {
    let rest = message.strip_prefix(prefix)?;
    let rest = match rest.strip_prefix('|') {
        Some(rest) => rest,
        None if rest.is_empty() => rest,
        None => return None,
    };
    Some(Fields {
        rest: (!rest.is_empty()).then_some(rest),
    })
}

/// The `key=value` fields of `message` after `prefix|`. Fields without `=`
/// are skipped.
pub fn key_values<'a>(
    message: &'a str,
    prefix: &str,
) -> Option<impl Iterator<Item = (&'a str, &'a str)>> {
    let fields = fields(message, prefix)?;
    Some(fields.filter_map(|field| field.split_once('=')))
}

/// Positional message fields; each accessor consumes one.
pub struct Fields<'a> {
    rest: Option<&'a str>,
}

impl<'a> Iterator for Fields<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest?;
        match rest.split_once('|') {
            Some((field, tail)) => {
                self.rest = Some(tail);
                Some(field)
            }
            None => {
                self.rest = None;
                Some(rest)
            }
        }
    }
}

impl<'a> Fields<'a> {
    /// A finite float.
    pub fn f32(&mut self) -> Option<f32> {
        self.next()?
            .parse::<f32>()
            .ok()
            .filter(|value| value.is_finite())
    }

    pub fn i32(&mut self) -> Option<i32> {
        self.next()?.parse().ok()
    }

    /// `1`/`0` (any other non-empty value counts as `true`).
    pub fn flag(&mut self) -> Option<bool> {
        let field = self.next()?;
        (!field.is_empty()).then(|| field != "0")
    }

    /// Two finite floats, typically a map position `x|z`.
    pub fn xz(&mut self) -> Option<(f32, f32)> {
        Some((self.f32()?, self.f32()?))
    }

    /// Three finite floats.
    pub fn xyz(&mut self) -> Option<(f32, f32, f32)> {
        Some((self.f32()?, self.f32()?, self.f32()?))
    }

    /// Everything not consumed yet, `|` included, as one field: for a free
    /// text last field such as a story line.
    pub fn rest(&mut self) -> Option<&'a str> {
        self.rest.take()
    }

    /// True when every field has been consumed.
    pub fn is_done(&self) -> bool {
        self.rest.is_none()
    }
}

/// Builds a message: `Encoder::new("STATUS").kv("health", 40).finish()`.
pub struct Encoder(String);

impl Encoder {
    pub fn new(prefix: &str) -> Self {
        Self(String::from(prefix))
    }

    /// Append a positional field.
    pub fn field(mut self, value: impl Display) -> Self {
        let _ = write!(self.0, "|{value}");
        self
    }

    /// Append a `key=value` field.
    pub fn kv(mut self, key: &str, value: impl Display) -> Self {
        let _ = write!(self.0, "|{key}={value}");
        self
    }

    /// Append each value as a positional field.
    pub fn fields<T: Display>(self, values: impl IntoIterator<Item = T>) -> Self {
        values.into_iter().fold(self, Self::field)
    }

    /// Append `1` or `0`.
    pub fn flag(self, value: bool) -> Self {
        self.field(u8::from(value))
    }

    /// Append `key=1` or `key=0`.
    pub fn kv_flag(self, key: &str, value: bool) -> Self {
        self.kv(key, u8::from(value))
    }

    pub fn finish(self) -> String {
        self.0
    }
}

/// Remembers the last value sent so a message goes out only when it changes.
///
/// ```ignore
/// self.status.send_if_changed(status, |s| spring::send_lua_ui_msg(&s.encode(), ""));
/// ```
pub struct SendOnChange<T> {
    last: RefCell<Option<T>>,
}

impl<T> Default for SendOnChange<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SendOnChange<T> {
    pub const fn new() -> Self {
        Self {
            last: RefCell::new(None),
        }
    }

    /// Forget the last value, so the next call sends again (after a restart
    /// or when the receiver was reloaded).
    pub fn reset(&self) {
        self.last.replace(None);
    }
}

impl<T: PartialEq + Clone> SendOnChange<T> {
    /// Call `send` with `value` unless it equals the last value sent
    /// successfully. `send` reports success; a failed send is retried on the
    /// next call. Returns whether a message went out.
    pub fn send_if_changed(&self, value: T, send: impl FnOnce(&T) -> bool) -> bool {
        if self.last.borrow().as_ref() == Some(&value) {
            return false;
        }
        // No borrow is held while `send` calls into the engine.
        if !send(&value) {
            return false;
        }
        self.last.replace(Some(value));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positional_fields() {
        let mut f = fields("MOVE|12.5|-3|1|x", "MOVE").unwrap();
        assert_eq!(f.xz(), Some((12.5, -3.0)));
        assert_eq!(f.flag(), Some(true));
        assert_eq!(f.i32(), None);
        assert!(f.is_done());
    }

    #[test]
    fn prefix_must_match_whole_field() {
        assert!(fields("MOVEX|1", "MOVE").is_none());
        assert!(fields("MOVE", "MOVE").unwrap().is_done());
        assert!(fields("OTHER|1", "MOVE").is_none());
    }

    #[test]
    fn rejects_non_finite() {
        let mut f = fields("P|inf|NaN|2", "P").unwrap();
        assert_eq!(f.f32(), None);
        assert_eq!(f.f32(), None);
        assert_eq!(f.f32(), Some(2.0));
    }

    #[test]
    fn key_values_skip_bare_fields() {
        let pairs: alloc::vec::Vec<_> = key_values("S|a=1|junk|b=x=y", "S").unwrap().collect();
        assert_eq!(pairs, [("a", "1"), ("b", "x=y")]);
    }

    #[test]
    fn rest_keeps_separators() {
        let mut f = fields("S|1|free|text", "S").unwrap();
        assert_eq!(f.i32(), Some(1));
        assert_eq!(f.rest(), Some("free|text"));
        assert!(f.is_done());
        assert_eq!(Encoder::new("L").fields([1, 2]).finish(), "L|1|2");
    }

    #[test]
    fn encoder_round_trip() {
        let msg = Encoder::new("S")
            .kv("hp", 40)
            .flag(true)
            .field(1.5)
            .finish();
        assert_eq!(msg, "S|hp=40|1|1.5");
    }

    #[test]
    fn send_on_change_retries_failures() {
        let latest = SendOnChange::new();
        assert!(!latest.send_if_changed(1, |_| false));
        assert!(latest.send_if_changed(1, |_| true));
        assert!(!latest.send_if_changed(1, |_| true));
        assert!(latest.send_if_changed(2, |_| true));
        latest.reset();
        assert!(latest.send_if_changed(2, |_| true));
    }
}
