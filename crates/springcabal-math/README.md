# Engine-compatible float math

`powf`, `sinf` and `cosf` use the same float implementation as the pinned
Recoil 2026.10.25 Lua runtime. One-bit differences in other math libraries can
change seeded spawning, steering and later work. The functions run inside the
Wasm module and require no engine imports.

The numeric source comes from RecoilEngine/streflop commit
`570f86fb616a20179914aa7fc9645e683a3f5b4d`, `libm/flt-32/`.
The original Sun permission notices remain in each source file. Adaptations
are a standalone float/type/bit adapter, removal of unused weak aliases and
C names on the three entry points. Numeric formulas and constants are retained.

Build with Cargo as usual. Clang and its LLVM archiver must be installed;
`CXX` and `AR` can name their executable paths. The build uses strict float
arithmetic, with contraction disabled, and no C++ standard library.

`cargo test -p springcabal-math --target x86_64-unknown-linux-gnu` checks
recorded values from the actual engine, including cases where Rust libm differs.
