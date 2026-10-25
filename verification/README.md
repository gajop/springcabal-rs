# Music playback regression

From a configured `springcabal-dev` checkout containing this repository, run:

```sh
springcabal-rs/verification/music-loop.sh
```

The check uses the existing development verification helpers and Area 17 as a
consumer of `springcabal::music::MusicLoop`, alongside the original Lua widget.
It needs FFmpeg, clips the real soundtrack to two seconds in private staged game
copies, and checks at least three stream restarts plus continued PCM output.
SDL's disk driver captures only the test engine's audio without using speakers.
Game checkouts and installed WASM modules are left untouched. Logs and captured
audio go to `verification/out/shared-music-{lua,wasm}/` in the development repo.
Set `SPRINGCABAL_DEV_ROOT` if this repository is located elsewhere.
