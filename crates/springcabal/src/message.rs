//! The `PREFIX|field|field` text messages our games pass between rules and UI.
//!
//! Rules send status to the UI with [`send_to_ui`] (`spring::send_to_unsynced`);
//! the UI sends input to the rules with [`send_to_rules`]
//! (`spring::send_lua_rules_msg`). Fields are either positional
//! (`MOVE|12.5|300`) or `key=value` pairs (`STATUS|health=40|gameEnd=0`).
//!
//! [`Encoder`] escapes `\` and `|` in values (and `=` in keys) with a
//! backslash, so a value can't split into two fields; [`Fields::text`] and
//! [`unescape`] undo it. Parsing is strict about numbers: a non-finite float
//! never reaches synced code, and the encoder reports one it is asked to
//! write, since the receiver would drop that message.

use alloc::borrow::Cow;
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

/// A message whose first field names the command (`hold|zap|12|40`): the
/// command and the remaining fields.
pub fn command(message: &str) -> Option<(&str, Fields<'_>)> {
    let (command, rest) = match split_unescaped(message, b'|') {
        Some((command, rest)) => (command, Some(rest)),
        None => (message, None),
    };
    (!command.is_empty()).then_some((
        command,
        Fields {
            rest: rest.filter(|rest| !rest.is_empty()),
        },
    ))
}

/// The `key=value` fields of `message` after `prefix|`. Fields without `=`
/// are skipped. Keys and values come as sent (escaped); numbers need no
/// unescaping, text does ([`unescape`]).
pub fn key_values<'a>(
    message: &'a str,
    prefix: &str,
) -> Option<impl Iterator<Item = (&'a str, &'a str)>> {
    let fields = fields(message, prefix)?;
    Some(fields.filter_map(|field| split_unescaped(field, b'=')))
}

/// A field as it was before [`Encoder`] escaped it.
pub fn unescape(field: &str) -> Cow<'_, str> {
    if !field.contains('\\') {
        return Cow::Borrowed(field);
    }
    let mut out = String::with_capacity(field.len());
    let mut chars = field.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => out.extend(chars.next()),
            c => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Split at the first `separator` that isn't escaped.
fn split_unescaped(text: &str, separator: u8) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            byte if byte == separator => return Some((&text[..i], &text[i + 1..])),
            _ => i += 1,
        }
    }
    None
}

/// Positional message fields; each accessor consumes one. Clone it to look
/// ahead without consuming.
#[derive(Clone)]
pub struct Fields<'a> {
    rest: Option<&'a str>,
}

impl<'a> Iterator for Fields<'a> {
    /// The field as sent (escaped); see [`Fields::text`].
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest?;
        match split_unescaped(rest, b'|') {
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

    /// `1` or `0` (as [`Encoder::flag`] writes), or `true` / `false`.
    pub fn flag(&mut self) -> Option<bool> {
        match self.next()? {
            "1" | "true" => Some(true),
            "0" | "false" => Some(false),
            _ => None,
        }
    }

    /// A text field, unescaped.
    pub fn text(&mut self) -> Option<Cow<'a, str>> {
        self.next().map(unescape)
    }

    /// Two finite floats, typically a map position `x|z`.
    pub fn xz(&mut self) -> Option<(f32, f32)> {
        Some((self.f32()?, self.f32()?))
    }

    /// Three finite floats.
    pub fn xyz(&mut self) -> Option<(f32, f32, f32)> {
        Some((self.f32()?, self.f32()?, self.f32()?))
    }

    /// Everything not consumed yet, `|` included, as one field: the free
    /// text [`Encoder::rest`] wrote last, such as a story line.
    pub fn rest(&mut self) -> Option<&'a str> {
        self.rest.take()
    }

    /// True when every field has been consumed.
    pub fn is_done(&self) -> bool {
        self.rest.is_none()
    }
}

/// Builds a message: `Encoder::new("STATUS").kv("health", 40).finish()`.
pub struct Encoder {
    message: String,
    /// What is wrong with the message, if anything: the receiver would drop
    /// it.
    problem: Option<Problem>,
    /// [`rest`](Self::rest) was written; nothing may follow it.
    closed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Field `n` (from 1) is `NaN` or infinite, which the parser rejects.
    NonFinite(usize),
    /// A field was added after [`Encoder::rest`], so it became part of the
    /// free text.
    AfterRest,
}

impl Encoder {
    pub fn new(prefix: &str) -> Self {
        Self {
            message: String::from(prefix),
            problem: None,
            closed: false,
        }
    }

    /// Append a positional field.
    pub fn field(mut self, value: impl Display) -> Self {
        self.separator();
        self.value(value, false);
        self
    }

    /// Append a `key=value` field.
    pub fn kv(mut self, key: &str, value: impl Display) -> Self {
        self.separator();
        self.value(key, true);
        self.message.push('=');
        self.value(value, false);
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

    /// Append free text as it is, `|` and all, as the last field. The
    /// receiver reads it with [`Fields::rest`].
    pub fn rest(mut self, text: &str) -> Self {
        self.separator();
        self.message.push_str(text);
        self.closed = true;
        self
    }

    /// What would make the receiver drop this message, if anything.
    pub fn problem(&self) -> Option<Problem> {
        self.problem
    }

    /// The message. A [`problem`](Self::problem) is logged (once per
    /// message prefix), since the receiver will drop the message.
    pub fn finish(self) -> String {
        #[cfg(target_arch = "wasm32")]
        if let Some(problem) = self.problem {
            report(&self.message, problem);
        }
        self.message
    }

    fn separator(&mut self) {
        if self.closed && self.problem.is_none() {
            self.problem = Some(Problem::AfterRest);
        }
        self.message.push('|');
    }

    fn value(&mut self, value: impl Display, key: bool) {
        let start = self.message.len();
        let _ = write!(self.message, "{value}");
        let written = &self.message[start..];
        if matches!(written, "NaN" | "inf" | "-inf") && self.problem.is_none() {
            let index = self.message[..start].matches('|').count();
            self.problem = Some(Problem::NonFinite(index));
        }
        let special = |c: char| c == '\\' || c == '|' || (key && c == '=');
        if written.contains(special) {
            let escaped: String = written
                .chars()
                .flat_map(|c| {
                    let escape = special(c).then_some('\\');
                    escape.into_iter().chain(core::iter::once(c))
                })
                .collect();
            self.message.truncate(start);
            self.message.push_str(&escaped);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn report(message: &str, problem: Problem) {
    static REPORTED: crate::local::Local<crate::local::Seen> =
        crate::local::Local::new(crate::local::Seen::new());
    let prefix = message.split('|').next().unwrap_or(message);
    if REPORTED.first(prefix) {
        spring::log_warning!(
            "springcabal",
            "message `{message}` will be dropped by its receiver: {problem:?}"
        );
    }
}

/// Send `message` from the UI to the rules (`SendLuaRulesMsg`). Returns
/// whether the engine took it; a failure is logged once per message prefix.
#[cfg(target_arch = "wasm32")]
pub fn send_to_rules(message: &str) -> bool {
    sent(message, "rules", spring::send_lua_rules_msg(message))
}

/// Send `message` from the synced rules to their unsynced side and the UI
/// (`SendToUnsynced`). Returns whether the engine took it; a failure is
/// logged once per message prefix.
#[cfg(target_arch = "wasm32")]
pub fn send_to_ui(message: &str) -> bool {
    sent(message, "UI", spring::send_to_unsynced(message))
}

#[cfg(target_arch = "wasm32")]
fn sent(message: &str, to: &str, result: spring::Result<bool>) -> bool {
    static FAILED: crate::local::Local<crate::local::Seen> =
        crate::local::Local::new(crate::local::Seen::new());
    let prefix = message.split('|').next().unwrap_or(message);
    if matches!(result, Ok(true)) {
        // Log the next failure of this message again.
        FAILED.forget(prefix);
        return true;
    }
    if FAILED.first(prefix) {
        spring::log_warning!(
            "springcabal",
            "`{message}` not sent to the {to}: {result:?}"
        );
    }
    false
}

/// Remembers the last value sent so a message goes out only when it changes.
///
/// ```ignore
/// self.status.send_if_changed(status, |s| send_to_ui(&s.encode()));
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

    /// Start as if `value` had been sent: the receiver's initial state, which
    /// needs no message.
    pub const fn assuming_sent(value: T) -> Self {
        Self {
            last: RefCell::new(Some(value)),
        }
    }

    /// Forget the last value, so the next call sends again (after a restart
    /// or when the receiver was reloaded).
    pub fn reset(&self) {
        self.last.replace(None);
    }
}

impl<T: Clone> SendOnChange<T> {
    /// Like [`send_if_changed`](Self::send_if_changed), with a custom test:
    /// `changed(last, new)` decides whether `value` is worth sending, e.g.
    /// only when a target moved more than a few elmos.
    pub fn send_if(
        &self,
        value: T,
        changed: impl FnOnce(&T, &T) -> bool,
        send: impl FnOnce(&T) -> bool,
    ) -> bool {
        if self
            .last
            .borrow()
            .as_ref()
            .is_some_and(|last| !changed(last, &value))
        {
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

impl<T: PartialEq + Clone> SendOnChange<T> {
    /// Call `send` with `value` unless it equals the last value sent
    /// successfully. `send` reports success; a failed send is retried on the
    /// next call. Returns whether a message went out.
    pub fn send_if_changed(&self, value: T, send: impl FnOnce(&T) -> bool) -> bool {
        self.send_if(value, |last, new| last != new, send)
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
    fn command_splits_first_field() {
        let (name, mut rest) = command("hold|zap|1.5").unwrap();
        assert_eq!(name, "hold");
        let mut ahead = rest.clone();
        assert_eq!(ahead.next(), Some("zap"));
        assert_eq!(rest.next(), Some("zap"));
        assert_eq!(rest.f32(), Some(1.5));
        let (name, rest) = command("release").unwrap();
        assert_eq!(name, "release");
        assert!(rest.is_done());
        assert!(command("").is_none());
    }

    #[test]
    fn send_if_uses_custom_test() {
        let aim = SendOnChange::new();
        let moved = |a: &f32, b: &f32| (a - b).abs() > 8.0;
        assert!(aim.send_if(0.0, moved, |_| true));
        assert!(!aim.send_if(5.0, moved, |_| true));
        assert!(aim.send_if(9.0, moved, |_| true));
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
    fn flags_are_strict() {
        let mut f = fields("F|1|0|true|false|2|", "F").unwrap();
        let flags: alloc::vec::Vec<_> = core::iter::from_fn(|| Some(f.flag())).take(6).collect();
        assert_eq!(
            flags,
            [Some(true), Some(false), Some(true), Some(false), None, None]
        );
    }

    #[test]
    fn separators_in_values_round_trip() {
        let encoder = Encoder::new("S")
            .field("a|b")
            .field(r"back\slash")
            .kv("k=ey", "v=a|l")
            .field(2);
        assert_eq!(encoder.problem(), None);
        let msg = encoder.finish();
        assert_eq!(msg, r"S|a\|b|back\\slash|k\=ey=v=a\|l|2");
        let mut f = fields(&msg, "S").unwrap();
        assert_eq!(f.text().as_deref(), Some("a|b"));
        assert_eq!(f.text().as_deref(), Some(r"back\slash"));
        let (key, value) = split_unescaped(f.next().unwrap(), b'=').unwrap();
        assert_eq!((&*unescape(key), &*unescape(value)), ("k=ey", "v=a|l"));
        assert_eq!(f.i32(), Some(2));
        assert!(f.is_done());
        let pairs: alloc::vec::Vec<_> = key_values(r"S|a\=b=c", "S").unwrap().collect();
        assert_eq!(pairs, [(r"a\=b", "c")]);
    }

    #[test]
    fn non_finite_is_reported() {
        let ok = Encoder::new("P")
            .field(1.5)
            .field(format_args!("{:.1}", 2.0));
        assert_eq!(ok.problem(), None);
        let nan = Encoder::new("P")
            .field(1.0)
            .field(f32::NAN)
            .field(f32::INFINITY);
        assert_eq!(nan.problem(), Some(Problem::NonFinite(2)));
        let msg = Encoder::new("P").kv("x", -f32::INFINITY).finish();
        assert!(fields(&msg, "P").unwrap().f32().is_none());
    }

    #[test]
    fn rest_is_verbatim_and_last() {
        let encoder = Encoder::new("S").field(1).rest("free|text");
        assert_eq!(encoder.problem(), None);
        let msg = encoder.finish();
        let mut f = fields(&msg, "S").unwrap();
        assert_eq!(f.i32(), Some(1));
        assert_eq!(f.rest(), Some("free|text"));
        let late = Encoder::new("S").rest("text").field(1);
        assert_eq!(late.problem(), Some(Problem::AfterRest));
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
        let idle = SendOnChange::assuming_sent(0);
        assert!(!idle.send_if_changed(0, |_| true));
        assert!(idle.send_if_changed(1, |_| true));
    }
}
