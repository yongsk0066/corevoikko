# legacy

Preserved C++ original implementation of libvoikko. All active development has moved to `libvoikko/rust/`. This code is kept for reference only.

The C++ source was the original implementation before the Rust port (Phases 0-5). It is not built by CI and should not be modified.

## Subdirectories

- `cpp-src/` -- C++ source code. Organized by module: `morphology/`, `spellchecker/`, `grammar/`, `hyphenator/`, `fst/`, `tokenizer/`, `sentence/`, `character/`, `setup/`, `tools/`, `utils/`, `utf8/`, `compatibility/`. Also contains the public C API headers (`voikko.h`, `voikko_defines.h`, `voikko_enums.h`, `voikko_structs.h`).
- `cpp-test/` -- Python test suite for the C++ library (`libvoikkoTest.py`, `DeprecatedApiTest.py`, `Utf8ApiTest.py`, etc.). Tests use `libvoikko` Python ctypes bindings.
- `autotools/` -- Autotools build system (`configure.ac`, `autogen.sh`, `Makefile.am`). Used by the Docker container and original Linux packaging.
- `emscripten/` -- Emscripten WASM build scripts for the C++ version (`build.sh`, `libvoikko_api.js`). Superseded by Rust `voikko-wasm` crate.
- `cs-cpp/` -- C# P/Invoke bindings for the C++ shared library (`Voikko.cs`, etc.). Superseded by Rust `voikko-ffi` crate + `libvoikko/cs/VoikkoRust.cs`.
- `cs-cpp-tests/` -- C# unit tests for the C++ bindings.
