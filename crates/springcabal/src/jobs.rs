//! Optional same-tick pure work. Engine calls, RNG and applying results stay
//! on the original thread in their original order. The default is serial.
use alloc::vec::Vec;
use spring::{
    ResultExt,
    jobs::{BatchJob, Worker},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Off,
    On,
    Check,
}

#[derive(Debug, Default)]
pub struct Options {
    mode: Option<Mode>,
    announced: bool,
}
impl Options {
    pub fn enabled<E: spring::capabilities::Game>(
        &mut self,
        spring: &spring::Spring<'_, E>,
    ) -> bool {
        if self.mode.is_none() {
            let value = spring
                .game()
                .get_mod_option("rust_jobs")
                .log_err_once(spring, "reading rust_jobs");
            self.mode = Some(match value.as_ref().map(|v| v.value.as_str()) {
                Some("1" | "on") => Mode::On,
                Some("check") => Mode::Check,
                _ => Mode::Off,
            });
        }
        self.mode != Some(Mode::Off)
    }

    pub fn checking(&self) -> bool {
        self.mode == Some(Mode::Check)
    }

    pub fn run<E: spring::capabilities::Common>(
        &mut self,
        spring: &spring::Spring<'_, E>,
        worker: Worker,
        inputs: &[Vec<u8>],
        output_limits: &[usize],
        label: &str,
        inline: fn(&[u8]) -> Vec<u8>,
    ) -> Option<Vec<Vec<u8>>> {
        if inputs.is_empty() {
            return Some(Vec::new());
        }
        assert_eq!(inputs.len(), output_limits.len());
        let jobs: Vec<_> = inputs
            .iter()
            .zip(output_limits)
            .map(|(input, &max_output_bytes)| BatchJob {
                input,
                max_output_bytes,
            })
            .collect();
        let results = spring
            .jobs()
            .run_this_tick(worker, &jobs)
            .log_err_once(spring, "running same-tick game jobs")?;
        let mut output = Vec::with_capacity(inputs.len());
        for (result, input) in results.into_iter().zip(inputs) {
            let bytes = match result {
                Ok(bytes) => bytes,
                Err(error) => {
                    spring::log_error!(spring, "GameJobs", "{label} worker failed: {error:?}");
                    return None;
                }
            };
            if self.mode == Some(Mode::Check) {
                assert_eq!(
                    bytes,
                    inline(input),
                    "{label} job differs from inline computation"
                );
            }
            output.push(bytes);
        }
        assert_eq!(output.len(), inputs.len());
        if !self.announced {
            spring::log_notice!(
                spring,
                "GameJobs",
                "active {label} mode={:?} chunks={} input_bytes={}",
                self.mode,
                jobs.len(),
                inputs.iter().map(Vec::len).sum::<usize>()
            );
            self.announced = true;
        }
        Some(output)
    }
}

pub const CHUNKS: usize = 4;
pub fn put_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}
pub fn put_f32(bytes: &mut Vec<u8>, value: f32) {
    put_u32(bytes, value.to_bits());
}

pub struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
    pub fn u32(&mut self) -> u32 {
        let (word, rest) = self
            .0
            .split_at_checked(4)
            .expect("truncated game-job packet");
        self.0 = rest;
        u32::from_le_bytes(word.try_into().expect("four-byte word"))
    }
    pub fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }
    pub fn finish(self) {
        assert!(self.0.is_empty(), "trailing game-job bytes");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_preserves_float_bits_including_signed_zero() {
        let mut bytes = Vec::new();
        for value in [0u32, 0x80000000, 0x3f800000, 0x7fc01234] {
            put_f32(&mut bytes, f32::from_bits(value));
        }
        let mut reader = Reader::new(&bytes);
        for value in [0u32, 0x80000000, 0x3f800000, 0x7fc01234] {
            assert_eq!(reader.f32().to_bits(), value);
        }
        reader.finish();
    }
    #[test]
    #[should_panic(expected = "truncated")]
    fn truncated_packets_fail() {
        Reader::new(&[0; 3]).u32();
    }
    #[test]
    #[should_panic(expected = "trailing")]
    fn unconsumed_packets_fail() {
        Reader::new(&[0; 4]).finish();
    }
}
