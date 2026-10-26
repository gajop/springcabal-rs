//! Float math matching the engine's Lua runtime.
//!
//! The same pinned fdlibm/streflop implementation is compiled into Wasm, so
//! tiny libm differences cannot change seeded game decisions and trajectories.
unsafe extern "C" {
    fn cabal_math_powf(x: f32, y: f32) -> f32;
    fn cabal_math_sinf(x: f32) -> f32;
    fn cabal_math_cosf(x: f32) -> f32;
    fn cabal_math_acosf(x: f32) -> f32;
}

/// Raise `x` to `y` using the engine's 32-bit math.
#[inline]
pub fn powf(x: f32, y: f32) -> f32 {
    // The C function takes and returns only scalar floats and retains no state.
    unsafe { cabal_math_powf(x, y) }
}
/// Sine using the engine's 32-bit argument reduction and polynomial.
#[inline]
pub fn sinf(x: f32) -> f32 {
    unsafe { cabal_math_sinf(x) }
}
/// Cosine using the engine's 32-bit argument reduction and polynomial.
#[inline]
pub fn cosf(x: f32) -> f32 {
    unsafe { cabal_math_cosf(x) }
}

/// Arc cosine using the engine's 32-bit argument reduction and polynomial.
#[inline]
pub fn acosf(x: f32) -> f32 {
    unsafe { cabal_math_acosf(x) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_recorded_engine_spawn_vectors() {
        // Actual Lua results from the Hunted synced engine probe. Current
        // Rust libm differs by one bit in at least one result per vector.
        for (angle, sine, cosine) in [
            (0x40838660, 0xbf52f654, 0xbf110540),
            (0x401da54e, 0x3f20a5e2, 0xbf4751c4),
            (0x400df88f, 0x3f4c2f28, 0xbf1a6aea),
        ] {
            assert_eq!(sinf(f32::from_bits(angle)).to_bits(), sine);
            assert_eq!(cosf(f32::from_bits(angle)).to_bits(), cosine);
        }
    }

    #[test]
    fn matches_the_recorded_engine_fractional_power() {
        // Rabbit 3403 at frame 20: a one-bit result difference changes speed.
        let fear = f32::from_bits(0x424a2e3b);
        assert_eq!(powf(fear / 45.0, 0.8).to_bits(), 0x3f8c7850);
    }

    #[test]
    fn arc_cosine_preserves_endpoints_and_domain() {
        assert_eq!(acosf(1.0), 0.0);
        assert_eq!(acosf(0.0), core::f32::consts::FRAC_PI_2);
        assert_eq!(acosf(-1.0), core::f32::consts::PI);
        assert!(acosf(1.01).is_nan());
        assert!(acosf(f32::NAN).is_nan());
    }

    #[test]
    fn scalar_entry_points_preserve_basic_and_signed_zero_cases() {
        assert_eq!(powf(2.0, 3.0), 8.0);
        assert_eq!(powf(0.0, 0.0), 1.0);
        assert_eq!(cosf(0.0), 1.0);
        assert_eq!(sinf(-0.0).to_bits(), (-0.0_f32).to_bits());
        assert!(sinf(f32::INFINITY).is_nan());
        assert!(cosf(f32::INFINITY).is_nan());
    }
}
