#!/bin/sh
set -e

# ── Usage ───────────────────────────────────────────────────────────

if [ $# -eq 0 ]; then
  echo "Usage: $0 <mode>"
  echo ""
  echo "Modes:"
  echo "  plain    - WASM only (dictionary loaded at runtime via fetch/fs)"
  echo "  embed    - Dictionary embedded in WASM (larger binary, offline use)"
  echo "  preload  - Dictionary as .data file (preloaded at startup)"
  echo ""
  echo "Prerequisites:"
  echo "  source /path/to/emsdk/emsdk_env.sh"
  echo "  cd libvoikko && ./autogen.sh && js/configure.sh"
  exit 0
fi

# ── Check prerequisites ────────────────────────────────────────────

if ! command -v emcc >/dev/null 2>&1; then
  echo "Error: emcc not found. Run: source /path/to/emsdk/emsdk_env.sh"
  exit 1
fi

if [ ! -f Makefile ]; then
  echo "Error: Makefile not found. Run: ./autogen.sh && js/configure.sh"
  exit 1
fi

# ── Clean stale native objects if needed ───────────────────────────

# If .o files exist but aren't WASM, a previous native build is present.
# Mixing native and WASM objects causes linker errors.
SAMPLE_OBJ=$(find src -name '*.o' -print -quit 2>/dev/null)
if [ -n "$SAMPLE_OBJ" ] && ! file "$SAMPLE_OBJ" | grep -q "WebAssembly"; then
  echo "Detected native build artifacts, running make clean..."
  emmake make clean
fi

# ── Build options ──────────────────────────────────────────────────

OPTS="-O3 --closure 1"

case $1 in
  embed)
    # Closure compiler may run out of memory when large dictionary files are embedded.
    OPTS="-O1 --closure 0 --embed-file 5"
    ;;
  preload)
    OPTS="-O3 --closure 1 --preload-file 5"
    ;;
  plain)
    ;;
  *)
    echo "Error: unknown mode '$1' (use: plain, embed, preload)"
    exit 1
    ;;
esac

# ── Compile C++ → .a ──────────────────────────────────────────────

emmake make

LIBFILE=$(find . -name libvoikko.a)
if [ -z "$LIBFILE" ]; then
  echo "Error: libvoikko.a not found after make"
  exit 1
fi

# ── Link → WASM + JS glue ─────────────────────────────────────────

mkdir -p js/wasm

EXPORTED_FUNCTIONS="[
  '_voikkoInit',
  '_voikkoTerminate',
  '_voikkoSetBooleanOption',
  '_voikkoSetIntegerOption',
  '_voikkoSpellCstr',
  '_voikkoSuggestCstr',
  '_voikkoGetAttributeValues',
  '_voikkoHyphenateCstr',
  '_voikkoInsertHyphensCstr',
  '_voikkoFreeCstrArray',
  '_voikkoFreeCstr',
  '_voikkoNextTokenCstr',
  '_voikkoNextSentenceStartCstr',
  '_voikkoNextGrammarErrorCstr',
  '_voikkoGetGrammarErrorCode',
  '_voikkoGetGrammarErrorStartPos',
  '_voikkoGetGrammarErrorLength',
  '_voikkoGetGrammarErrorSuggestions',
  '_voikkoFreeGrammarError',
  '_voikkoGetGrammarErrorShortDescription',
  '_voikkoFreeErrorMessageCstr',
  '_voikko_list_dicts',
  '_voikko_free_dicts',
  '_voikko_dict_language',
  '_voikko_dict_script',
  '_voikko_dict_variant',
  '_voikko_dict_description',
  '_voikkoGetVersion',
  '_voikkoAnalyzeWordCstr',
  '_voikko_free_mor_analysis',
  '_voikko_mor_analysis_keys',
  '_voikko_mor_analysis_value_cstr',
  '_voikko_free_mor_analysis_value_cstr',
  '_emscripten_builtin_memalign',
  '_free',
  '_malloc'
]"

# Remove whitespace/newlines from the JSON array
EXPORTED_FUNCTIONS=$(echo "$EXPORTED_FUNCTIONS" | tr -d '[:space:]')

emcc -g0 "$LIBFILE" $OPTS \
  -o js/wasm/libvoikko.mjs \
  --post-js js/legacy/libvoikko_api.js \
  -s MODULARIZE=1 \
  -s EXPORT_ES6=1 \
  -s EXPORT_NAME="'Libvoikko'" \
  -s NO_EXIT_RUNTIME=1 \
  -s EXPORTED_FUNCTIONS="$EXPORTED_FUNCTIONS" \
  -s EXPORTED_RUNTIME_METHODS="['cwrap','FS']" \
  -s ALLOW_MEMORY_GROWTH=1

echo ""
echo "Build complete:"
ls -lh js/wasm/libvoikko.mjs js/wasm/libvoikko.wasm
