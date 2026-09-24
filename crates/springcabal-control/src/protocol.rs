//! The movement message from the UI to the rules.
//!
//! `movement|x|z`: the held direction, each axis -1, 0 or 1 (x to the right,
//! z towards the camera). `movement|0|0` means the keys were released. The UI
//! sends it only when the direction changes, so the rules keep applying the
//! last one they got.

use alloc::string::String;

use springcabal::message::{Encoder, fields};

/// The message prefix.
pub const DIRECTION: &str = "movement";

/// `movement|x|z`.
pub fn encode_direction((x, z): (i32, i32)) -> String {
    Encoder::new(DIRECTION).field(x).field(z).finish()
}

/// The direction in a `movement|x|z` message, each axis clamped to -1..=1.
/// Accepts fractional values (an analog stick, a test), rejects non-finite
/// ones.
pub fn decode_direction(message: &str) -> Option<(f32, f32)> {
    let (x, z) = fields(message, DIRECTION)?.xz()?;
    Some((x.clamp(-1.0, 1.0), z.clamp(-1.0, 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_clamp() {
        assert_eq!(encode_direction((1, -1)), "movement|1|-1");
        assert_eq!(decode_direction("movement|1|-1"), Some((1.0, -1.0)));
        assert_eq!(decode_direction("movement|5|-9"), Some((1.0, -1.0)));
        assert_eq!(decode_direction("movement|0.5|0"), Some((0.5, 0.0)));
        assert_eq!(decode_direction("movement|inf|0"), None);
        assert_eq!(decode_direction("movement|1"), None);
        assert_eq!(decode_direction("movementx|1|0"), None);
    }
}
