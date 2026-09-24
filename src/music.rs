//! Background music that loops, as every game's `snd_music` widget does.
//!
//! The engine's music channel plays a stream once, and `get_sound_stream_time`
//! reports the length of the current stream but not how far it has played. So
//! the widget counts elapsed time itself and restarts the stream just before
//! it ends. The Lua games did the same.

use core::cell::Cell;

use spring_addons::{AddonContext, Widget};

/// Restart this long before the end, so the loop has no audible gap.
const LOOP_LEAD: f32 = 0.015;

/// A looping music track; register it as a widget.
///
/// ```ignore
/// handler.add(MusicLoop::new("sounds/theme.ogg", 0.4).length_hint(91.0));
/// ```
pub struct MusicLoop {
    track: Cell<&'static str>,
    volume: Cell<f32>,
    length_hint: f32,
    state: Cell<State>,
    elapsed: Cell<f32>,
    length: Cell<f32>,
    failure_logged: Cell<bool>,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    /// Start (or restart) on the next update.
    Pending,
    Playing,
    Stopped,
}

impl MusicLoop {
    pub const fn new(track: &'static str, volume: f32) -> Self {
        Self {
            track: Cell::new(track),
            volume: Cell::new(volume),
            length_hint: 0.0,
            state: Cell::new(State::Pending),
            elapsed: Cell::new(0.0),
            length: Cell::new(0.0),
            failure_logged: Cell::new(false),
        }
    }

    /// Track length in seconds, used when the engine can't report it.
    pub const fn length_hint(mut self, seconds: f32) -> Self {
        self.length_hint = seconds;
        self
    }

    /// Start the current track again from the beginning (e.g. on restart).
    pub fn restart(&self) {
        self.state.set(State::Pending);
    }

    /// Switch to another track; it starts on the next update.
    pub fn switch(&self, track: &'static str, volume: f32) {
        self.track.set(track);
        self.volume.set(volume);
        self.restart();
    }

    /// Stop the music until [`restart`](Self::restart) or
    /// [`switch`](Self::switch).
    pub fn stop(&self) {
        self.state.set(State::Stopped);
        let _ = spring::sound::stop_sound_stream(0);
    }

    fn start(&self) {
        match spring::sound::play_sound_stream(self.track.get(), self.volume.get(), false) {
            Ok(true) => {
                let length = spring::sound::get_sound_stream_time(0)
                    .ok()
                    .filter(|length| *length > LOOP_LEAD)
                    .unwrap_or(self.length_hint);
                self.length.set(length);
                self.elapsed.set(0.0);
                self.state.set(State::Playing);
            }
            Ok(false) | Err(_) => {
                // Retried every update; logged once.
                if !self.failure_logged.replace(true) {
                    spring::log_warning!(
                        "springcabal",
                        "music: could not play {}",
                        self.track.get()
                    );
                }
            }
        }
    }

    fn tick(&self, dt: f32) {
        match self.state.get() {
            State::Stopped => {}
            State::Pending => self.start(),
            State::Playing => {
                let elapsed = self.elapsed.get() + dt.max(0.0);
                self.elapsed.set(elapsed);
                let length = self.length.get();
                // Without a known length the stream plays once.
                if length > LOOP_LEAD && elapsed >= length - LOOP_LEAD {
                    self.start();
                }
            }
        }
    }
}

impl<G> Widget<G> for MusicLoop {
    fn name(&self) -> &'static str {
        "snd_music"
    }

    fn update(&self, _ctx: &AddonContext<'_, G>, dt: f32) {
        self.tick(dt);
    }

    fn shutdown(&self, _ctx: &AddonContext<'_, G>) {
        self.stop();
    }
}
