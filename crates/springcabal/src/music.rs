//! Background music that loops, as every game's `snd_music` widget does.
//!
//! The engine's music channel plays a stream once. The widget watches the
//! stream's play position (`spring::sound_stream_time`, Lua's
//! `Spring.GetSoundStreamTime`) and starts the track again just before its
//! end, or as soon as the engine reports that it has stopped, the way the
//! Lua games did.

use core::cell::Cell;

#[cfg(target_arch = "wasm32")]
use spring::ResultExt;
#[cfg(target_arch = "wasm32")]
use spring_addons::{AddonContext, Widget};

/// Restart this long before the end, so the loop has no audible gap.
const LOOP_LEAD: f32 = 0.015;

/// A looping music track; register it as a widget.
///
/// ```ignore
/// handler.add(MusicLoop::new("sounds/theme.ogg", 0.4));
/// ```
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub struct MusicLoop {
    track: Cell<&'static str>,
    volume: Cell<f32>,
    state: Cell<State>,
    /// A failure to start the current track has been logged.
    warned: Cell<bool>,
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    /// Start (or restart) on the next update.
    Pending,
    /// `heard`: the engine has reported the stream's length since it
    /// started. After that, a length of zero means the engine dropped the
    /// stream at its end.
    Playing {
        heard: bool,
    },
    Stopped,
}

impl MusicLoop {
    pub const fn new(track: &'static str, volume: f32) -> Self {
        Self {
            track: Cell::new(track),
            volume: Cell::new(volume),
            state: Cell::new(State::Pending),
            warned: Cell::new(false),
        }
    }

    /// Start the current track again from the beginning (e.g. on restart).
    pub fn restart(&self) {
        self.state.set(State::Pending);
    }

    /// Switch to another track; it starts on the next update.
    pub fn switch(&self, track: &'static str, volume: f32) {
        if track != self.track.get() {
            self.warned.set(false);
        }
        self.track.set(track);
        self.volume.set(volume);
        self.restart();
    }
}

#[cfg(target_arch = "wasm32")]
impl MusicLoop {
    /// Stop the music until [`restart`](Self::restart) or
    /// [`switch`](Self::switch).
    pub fn stop(&self) {
        self.state.set(State::Stopped);
        spring::sound::stop_sound_stream(0).log_err("music: stop");
    }

    fn start(&self) {
        match spring::sound::play_sound_stream(self.track.get(), self.volume.get(), false) {
            Ok(true) => {
                self.state.set(State::Playing { heard: false });
                self.warned.set(false);
            }
            // Retried every update; logged once per track until it plays.
            failed => {
                if !self.warned.replace(true) {
                    spring::log_warning!(
                        "springcabal",
                        "music: could not play {}: {failed:?}",
                        self.track.get()
                    );
                }
            }
        }
    }

    fn tick(&self) {
        // Without sound (or before the stream reports a length) the track
        // plays once.
        let time = || {
            spring::sound_stream_time()
                .ok()
                .map(|time| (time.played, time.length))
        };
        match next(self.state.get(), time) {
            Step::Wait => {}
            Step::Heard => self.state.set(State::Playing { heard: true }),
            Step::Start => self.start(),
        }
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, PartialEq)]
enum Step {
    Wait,
    /// The stream has reported its length.
    Heard,
    /// Start (or restart) the track.
    Start,
}

/// What to do this update, given the stream's `(played, length)` in seconds
/// (asked only while playing).
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn next(state: State, time: impl FnOnce() -> Option<(f32, f32)>) -> Step {
    let heard = match state {
        State::Stopped => return Step::Wait,
        State::Pending => return Step::Start,
        State::Playing { heard } => heard,
    };
    let Some((played, length)) = time() else {
        return Step::Wait;
    };
    if length > LOOP_LEAD {
        if played >= length - LOOP_LEAD {
            Step::Start
        } else if heard {
            Step::Wait
        } else {
            Step::Heard
        }
    } else if heard {
        Step::Start
    } else {
        Step::Wait
    }
}

#[cfg(target_arch = "wasm32")]
impl<G> Widget<G> for MusicLoop {
    fn name(&self) -> &'static str {
        "SndMusic"
    }

    fn update(&self, _ctx: &AddonContext<'_, G>, _dt: f32) {
        self.tick();
    }

    fn shutdown(&self, _ctx: &AddonContext<'_, G>) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYING: State = State::Playing { heard: false };
    const HEARD: State = State::Playing { heard: true };

    #[test]
    fn starts_and_stops() {
        assert_eq!(next(State::Pending, || None), Step::Start);
        assert_eq!(next(State::Stopped, || Some((0.0, 10.0))), Step::Wait);
    }

    #[test]
    fn loops_just_before_the_end() {
        assert_eq!(next(PLAYING, || Some((1.0, 10.0))), Step::Heard);
        assert_eq!(next(HEARD, || Some((9.0, 10.0))), Step::Wait);
        assert_eq!(next(HEARD, || Some((9.99, 10.0))), Step::Start);
    }

    #[test]
    fn restarts_a_dropped_stream_once_heard() {
        // Before the length is known, a zero length means "not yet".
        assert_eq!(next(PLAYING, || Some((0.0, 0.0))), Step::Wait);
        // After it, the engine dropped the stream at its end.
        assert_eq!(next(HEARD, || Some((0.0, 0.0))), Step::Start);
        // No sound: play once.
        assert_eq!(next(HEARD, || None), Step::Wait);
    }
}
