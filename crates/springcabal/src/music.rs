//! Background music that loops, as every game's `snd_music` widget does.
//!
//! The engine's music channel plays a stream once. The widget watches the
//! stream's play position (`spring::sound_stream_time`, Lua's
//! `Spring.GetSoundStreamTime`) and starts the track again just before its
//! end, or as soon as the engine reports that it has stopped, the way the
//! Lua games did.

use core::cell::Cell;

use spring::ResultExt;
use spring_addons::{AddonContext, Widget};

/// Restart this long before the end, so the loop has no audible gap.
const LOOP_LEAD: f32 = 0.015;

/// A looping music track; register it as a widget.
///
/// ```ignore
/// handler.add(MusicLoop::new("sounds/theme.ogg", 0.4));
/// ```
pub struct MusicLoop {
    track: Cell<&'static str>,
    volume: Cell<f32>,
    state: Cell<State>,
}

#[derive(Clone, Copy, PartialEq)]
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
        }
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
        spring::sound::stop_sound_stream(0).log_err("music: stop");
    }

    fn start(&self) {
        match spring::sound::play_sound_stream(self.track.get(), self.volume.get(), false) {
            Ok(true) => self.state.set(State::Playing { heard: false }),
            // Retried every update; logged once.
            failed => spring::log_once!(
                spring::log_level::WARNING,
                "springcabal",
                "music: could not play {}: {failed:?}",
                self.track.get()
            ),
        }
    }

    fn tick(&self) {
        let heard = match self.state.get() {
            State::Stopped => return,
            State::Pending => return self.start(),
            State::Playing { heard } => heard,
        };
        // Without sound (or before the stream reports a length) the track
        // plays once.
        let Ok(time) = spring::sound_stream_time() else {
            return;
        };
        if time.length > LOOP_LEAD {
            if time.played >= time.length - LOOP_LEAD {
                self.start();
            } else if !heard {
                self.state.set(State::Playing { heard: true });
            }
        } else if heard {
            self.start();
        }
    }
}

impl<G> Widget<G> for MusicLoop {
    fn name(&self) -> &'static str {
        "snd_music"
    }

    fn update(&self, _ctx: &AddonContext<'_, G>, _dt: f32) {
        self.tick();
    }

    fn shutdown(&self, _ctx: &AddonContext<'_, G>) {
        self.stop();
    }
}
