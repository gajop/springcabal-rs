#!/usr/bin/env bash
# Exercise the shared MusicLoop through Area 17, alongside its Lua original.
# Stage both versions with the framework's existing helpers and capture only
# this engine's audio through SDL's disk driver; nothing plays on the speakers.
set -euo pipefail
fixtures=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
project_root=${SPRINGCABAL_DEV_ROOT:-$(cd "$fixtures/../.." && pwd)}
cd "$project_root"
source verification/lib/common.sh
command -v ffmpeg >/dev/null || vf_die 'ffmpeg is required to clip the test soundtrack'
exec {lock}>>"${VF_LOCK_FILE:-/tmp/springcabal-verify.lock}"
flock "$lock"
for variant in lua wasm; do
    game="$GAMES_DIR/Area-17.sdd"
    if [[ $variant == wasm ]]; then
        game="$GAMES_DIR/Area17-wasm.sdd"
        vf_build "$game/wasm-src" ''
    fi
    vf_workspace "shared-music-$variant" stable
    staged=$(vf_stage_game "$game")
    if [[ $variant == wasm ]]; then
        vf_install_modules "$staged" area17_rules_wasm:LuaRules/wasm/area17-rules.wasm \
            area17_ui_wasm:LuaUI/wasm/area17-ui.wasm
    fi
    ffmpeg -v error -y -i "$game/sounds/music.ogg" -t 2 -c:a libvorbis \
        "$staged/sounds/music-loop-test.ogg"
    mv "$staged/sounds/music-loop-test.ogg" "$staged/sounds/music.ogg"
    cp "$fixtures/music-loop.lua" "$staged/LuaRules/Gadgets/zz_music_loop_test.lua"
    vf_link_map "$(scripts/area17-map.sh --dir)"
    vf_start_file "$(vf_launch_script area17 "$variant")"
    vf_config
    vf_graphics area17
    vf_config_set Sound 1
    vf_config_set snd_volmaster 100
    vf_config_set snd_volmusic 100
    for channel in general battle ui unitreply; do vf_config_set "snd_vol$channel" 0; done
    export SDL_AUDIODRIVER=disk SDL_DISKAUDIOFILE="$VF_OUT_DIR/music.s16le"
    vf_run_visual 25
    vf_save_logs
    vf_require_clean_exit
    vf_forbid_runtime_failures
    vf_require 'SHARED MUSIC LOOP PASSED'
    # The final second must still contain audible PCM after multiple loops.
    python3 - "$SDL_DISKAUDIOFILE" <<'PY'
from array import array
from pathlib import Path
import sys

samples = array('h')
samples.frombytes(Path(sys.argv[1]).read_bytes())
assert len(samples) > 44100 * 2, 'no audio captured'
assert any(abs(value) > 128 for value in samples[-44100 * 2:]), 'music became silent after looping'
PY
    printf '%s: soundtrack looped at least three times and kept producing audio\n' "$variant"
done
