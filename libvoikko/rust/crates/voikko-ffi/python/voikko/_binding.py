"""ctypes binding for libvoikko_ffi (Rust cdylib)."""

from __future__ import annotations

import ctypes
import os
import platform
import sys
from ctypes import (
    POINTER,
    Structure,
    c_char_p,
    c_int,
    c_size_t,
    c_uint8,
    c_void_p,
)
from pathlib import Path
from typing import Optional


# ── Library loading ──────────────────────────────────────────────

def _find_library() -> str:
    """Find libvoikko_ffi shared library."""
    system = platform.system()
    if system == "Darwin":
        name = "libvoikko_ffi.dylib"
    elif system == "Windows":
        name = "voikko_ffi.dll"
    else:
        name = "libvoikko_ffi.so"

    # Check env var first
    env_path = os.environ.get("VOIKKO_FFI_LIB")
    if env_path and os.path.isfile(env_path):
        return env_path

    # Check relative to this file (development layout)
    here = Path(__file__).resolve().parent
    candidates = [
        here / name,
        here.parent / name,
        # Rust build output
        here.parents[3] / "target" / "release" / name,
        here.parents[3] / "target" / "debug" / name,
    ]
    for p in candidates:
        if p.is_file():
            return str(p)

    # Fallback: let ctypes search system paths
    return name


_lib = ctypes.CDLL(_find_library())


# ── C struct definitions ────────────────────────────────────────

class _VoikkoAnalysis(Structure):
    _fields_ = [
        ("keys", POINTER(c_char_p)),
        ("values", POINTER(c_char_p)),
    ]


class _VoikkoAnalysisArray(Structure):
    _fields_ = [
        ("analyses", POINTER(_VoikkoAnalysis)),
        ("count", c_size_t),
    ]


class _VoikkoGrammarError(Structure):
    _fields_ = [
        ("error_code", c_int),
        ("start_pos", c_size_t),
        ("error_len", c_size_t),
        ("short_description", c_char_p),
        ("suggestions", POINTER(c_char_p)),
    ]


class _VoikkoGrammarErrorArray(Structure):
    _fields_ = [
        ("errors", POINTER(_VoikkoGrammarError)),
        ("count", c_size_t),
    ]


class _VoikkoToken(Structure):
    _fields_ = [
        ("token_type", c_int),
        ("text", c_char_p),
        ("position", c_size_t),
    ]


class _VoikkoTokenArray(Structure):
    _fields_ = [
        ("tokens", POINTER(_VoikkoToken)),
        ("count", c_size_t),
    ]


class _VoikkoSentence(Structure):
    _fields_ = [
        ("sentence_type", c_int),
        ("sentence_len", c_size_t),
    ]


class _VoikkoSentenceArray(Structure):
    _fields_ = [
        ("sentences", POINTER(_VoikkoSentence)),
        ("count", c_size_t),
    ]


# ── Function signatures ─────────────────────────────────────────

_lib.voikko_new.argtypes = [
    POINTER(c_uint8), c_size_t,
    POINTER(c_uint8), c_size_t,
    POINTER(c_char_p),
]
_lib.voikko_new.restype = c_void_p

_lib.voikko_free.argtypes = [c_void_p]
_lib.voikko_free.restype = None

_lib.voikko_spell.argtypes = [c_void_p, c_char_p]
_lib.voikko_spell.restype = c_int

_lib.voikko_suggest.argtypes = [c_void_p, c_char_p]
_lib.voikko_suggest.restype = POINTER(c_char_p)

_lib.voikko_analyze.argtypes = [c_void_p, c_char_p]
_lib.voikko_analyze.restype = _VoikkoAnalysisArray

_lib.voikko_hyphenate.argtypes = [c_void_p, c_char_p]
_lib.voikko_hyphenate.restype = c_void_p  # raw pointer, must free

_lib.voikko_insert_hyphens.argtypes = [c_void_p, c_char_p, c_char_p, c_int]
_lib.voikko_insert_hyphens.restype = c_void_p  # raw pointer, must free

_lib.voikko_grammar_errors.argtypes = [c_void_p, c_char_p, c_char_p]
_lib.voikko_grammar_errors.restype = _VoikkoGrammarErrorArray

_lib.voikko_tokens.argtypes = [c_void_p, c_char_p]
_lib.voikko_tokens.restype = _VoikkoTokenArray

_lib.voikko_sentences.argtypes = [c_void_p, c_char_p]
_lib.voikko_sentences.restype = _VoikkoSentenceArray

_lib.voikko_version.argtypes = []
_lib.voikko_version.restype = c_char_p

_lib.voikko_attribute_values.argtypes = [c_char_p]
_lib.voikko_attribute_values.restype = POINTER(c_char_p)

_lib.voikko_free_str.argtypes = [c_char_p]
_lib.voikko_free_str.restype = None

_lib.voikko_free_str_array.argtypes = [POINTER(c_char_p)]
_lib.voikko_free_str_array.restype = None

_lib.voikko_free_analyses.argtypes = [_VoikkoAnalysisArray]
_lib.voikko_free_analyses.restype = None

_lib.voikko_free_grammar_errors.argtypes = [_VoikkoGrammarErrorArray]
_lib.voikko_free_grammar_errors.restype = None

_lib.voikko_free_tokens.argtypes = [_VoikkoTokenArray]
_lib.voikko_free_tokens.restype = None

_lib.voikko_free_sentences.argtypes = [_VoikkoSentenceArray]
_lib.voikko_free_sentences.restype = None

# Option setters
for _name in [
    "voikko_set_ignore_dot", "voikko_set_ignore_numbers",
    "voikko_set_ignore_uppercase", "voikko_set_no_ugly_hyphenation",
    "voikko_set_accept_first_uppercase", "voikko_set_accept_all_uppercase",
    "voikko_set_ocr_suggestions", "voikko_set_ignore_nonwords",
    "voikko_set_accept_extra_hyphens", "voikko_set_accept_missing_hyphens",
    "voikko_set_accept_titles_in_gc",
    "voikko_set_accept_unfinished_paragraphs_in_gc",
    "voikko_set_hyphenate_unknown_words",
    "voikko_set_accept_bulleted_lists_in_gc",
    "voikko_set_min_hyphenated_word_length",
    "voikko_set_max_suggestions", "voikko_set_speller_cache_size",
]:
    fn = getattr(_lib, _name)
    fn.argtypes = [c_void_p, c_int]
    fn.restype = None


# ── Helper functions ─────────────────────────────────────────────

_TOKEN_TYPES = {0: "NONE", 1: "WORD", 2: "PUNCTUATION", 3: "WHITESPACE", 4: "UNKNOWN"}
_SENTENCE_TYPES = {0: "NONE", 1: "NO_START", 2: "PROBABLE", 3: "POSSIBLE"}


def _read_null_terminated(ptr: POINTER(c_char_p)) -> list[str]:
    """Read a NULL-terminated array of C strings into a Python list."""
    if not ptr:
        return []
    result = []
    i = 0
    while ptr[i]:
        result.append(ptr[i].decode("utf-8"))
        i += 1
    return result


def _enc(s: str) -> bytes:
    return s.encode("utf-8")


# ── Public API ───────────────────────────────────────────────────

class GrammarError:
    """Grammar error detected by Voikko."""

    __slots__ = ("error_code", "start_pos", "error_len", "short_description", "suggestions")

    def __init__(self, code: int, start: int, length: int, desc: str, sugg: list[str]):
        self.error_code = code
        self.start_pos = start
        self.error_len = length
        self.short_description = desc
        self.suggestions = sugg

    def __repr__(self) -> str:
        return (
            f"GrammarError(code={self.error_code}, pos={self.start_pos}, "
            f"len={self.error_len}, desc={self.short_description!r})"
        )


class Token:
    """Text token."""

    __slots__ = ("type", "text", "position")

    def __init__(self, token_type: str, text: str, position: int):
        self.type = token_type
        self.text = text
        self.position = position

    def __repr__(self) -> str:
        return f"Token({self.type}, {self.text!r})"


class Sentence:
    """Detected sentence boundary."""

    __slots__ = ("type", "length")

    def __init__(self, sentence_type: str, length: int):
        self.type = sentence_type
        self.length = length

    def __repr__(self) -> str:
        return f"Sentence({self.type}, len={self.length})"


class Voikko:
    """Finnish language NLP toolkit powered by Rust.

    Args:
        dict_path: Path to directory containing mor.vfst (and optionally autocorr.vfst).
                   Supports both flat layout and V5 structure ({path}/5/mor-standard/).
    """

    def __init__(self, dict_path: str):
        path = Path(dict_path)

        # Auto-detect V5 structure
        mor_path = path / "mor.vfst"
        if not mor_path.is_file():
            v5 = path / "5" / "mor-standard" / "mor.vfst"
            if v5.is_file():
                mor_path = v5
                path = v5.parent
            else:
                raise FileNotFoundError(f"mor.vfst not found in {dict_path}")

        mor_data = mor_path.read_bytes()
        autocorr_path = path / "autocorr.vfst"
        autocorr_data = autocorr_path.read_bytes() if autocorr_path.is_file() else None

        mor_buf = (c_uint8 * len(mor_data))(*mor_data)
        error_msg = c_char_p()

        if autocorr_data:
            ac_buf = (c_uint8 * len(autocorr_data))(*autocorr_data)
            handle = _lib.voikko_new(mor_buf, len(mor_data), ac_buf, len(autocorr_data), ctypes.byref(error_msg))
        else:
            handle = _lib.voikko_new(mor_buf, len(mor_data), None, 0, ctypes.byref(error_msg))

        if not handle:
            msg = error_msg.value.decode("utf-8") if error_msg.value else "unknown error"
            _lib.voikko_free_str(error_msg)
            raise RuntimeError(f"Failed to initialize Voikko: {msg}")

        self._handle = handle

    def __del__(self):
        self.terminate()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.terminate()

    def terminate(self) -> None:
        """Release resources."""
        if hasattr(self, "_handle") and self._handle:
            _lib.voikko_free(self._handle)
            self._handle = None

    def spell(self, word: str) -> bool:
        """Check spelling."""
        self._check_handle()
        return _lib.voikko_spell(self._handle, _enc(word)) == 1

    def suggest(self, word: str) -> list[str]:
        """Get spelling suggestions."""
        self._check_handle()
        ptr = _lib.voikko_suggest(self._handle, _enc(word))
        if not ptr:
            return []
        result = _read_null_terminated(ptr)
        _lib.voikko_free_str_array(ptr)
        return result

    def analyze(self, word: str) -> list[dict[str, str]]:
        """Morphological analysis."""
        self._check_handle()
        arr = _lib.voikko_analyze(self._handle, _enc(word))
        result = []
        for i in range(arr.count):
            a = arr.analyses[i]
            d = {}
            j = 0
            while a.keys[j]:
                k = a.keys[j].decode("utf-8")
                v = a.values[j].decode("utf-8")
                d[k] = v
                j += 1
            result.append(d)
        _lib.voikko_free_analyses(arr)
        return result

    def hyphenate(self, word: str, separator: str = "-", allow_context_changes: bool = True) -> str:
        """Hyphenate a word with the given separator."""
        self._check_handle()
        ptr = _lib.voikko_insert_hyphens(self._handle, _enc(word), _enc(separator), int(allow_context_changes))
        if not ptr:
            return word
        result = ctypes.cast(ptr, c_char_p).value.decode("utf-8")
        _lib.voikko_free_str(ctypes.cast(ptr, c_char_p))
        return result

    def get_hyphenation_pattern(self, word: str) -> str:
        """Get raw hyphenation pattern."""
        self._check_handle()
        ptr = _lib.voikko_hyphenate(self._handle, _enc(word))
        if not ptr:
            return " " * len(word)
        result = ctypes.cast(ptr, c_char_p).value.decode("utf-8")
        _lib.voikko_free_str(ctypes.cast(ptr, c_char_p))
        return result

    def grammar_errors(self, text: str, language: str = "fi") -> list[GrammarError]:
        """Check text for grammar errors."""
        self._check_handle()
        arr = _lib.voikko_grammar_errors(self._handle, _enc(text), _enc(language))
        result = []
        for i in range(arr.count):
            e = arr.errors[i]
            sugg = _read_null_terminated(e.suggestions) if e.suggestions else []
            desc = e.short_description.decode("utf-8") if e.short_description else ""
            result.append(GrammarError(e.error_code, e.start_pos, e.error_len, desc, sugg))
        _lib.voikko_free_grammar_errors(arr)
        return result

    def tokens(self, text: str) -> list[Token]:
        """Tokenize text."""
        self._check_handle()
        arr = _lib.voikko_tokens(self._handle, _enc(text))
        result = []
        for i in range(arr.count):
            t = arr.tokens[i]
            result.append(Token(
                _TOKEN_TYPES.get(t.token_type, "UNKNOWN"),
                t.text.decode("utf-8") if t.text else "",
                t.position,
            ))
        _lib.voikko_free_tokens(arr)
        return result

    def sentences(self, text: str) -> list[Sentence]:
        """Detect sentence boundaries."""
        self._check_handle()
        arr = _lib.voikko_sentences(self._handle, _enc(text))
        result = []
        for i in range(arr.count):
            s = arr.sentences[i]
            result.append(Sentence(
                _SENTENCE_TYPES.get(s.sentence_type, "NONE"),
                s.sentence_len,
            ))
        _lib.voikko_free_sentences(arr)
        return result

    def attribute_values(self, name: str) -> Optional[list[str]]:
        """Get valid values for a morphological attribute."""
        ptr = _lib.voikko_attribute_values(_enc(name))
        if not ptr:
            return None
        return _read_null_terminated(ptr)

    # -- Option setters --

    def set_ignore_dot(self, v: bool) -> None: _lib.voikko_set_ignore_dot(self._handle, int(v))
    def set_ignore_numbers(self, v: bool) -> None: _lib.voikko_set_ignore_numbers(self._handle, int(v))
    def set_ignore_uppercase(self, v: bool) -> None: _lib.voikko_set_ignore_uppercase(self._handle, int(v))
    def set_no_ugly_hyphenation(self, v: bool) -> None: _lib.voikko_set_no_ugly_hyphenation(self._handle, int(v))
    def set_accept_first_uppercase(self, v: bool) -> None: _lib.voikko_set_accept_first_uppercase(self._handle, int(v))
    def set_accept_all_uppercase(self, v: bool) -> None: _lib.voikko_set_accept_all_uppercase(self._handle, int(v))
    def set_ocr_suggestions(self, v: bool) -> None: _lib.voikko_set_ocr_suggestions(self._handle, int(v))
    def set_ignore_nonwords(self, v: bool) -> None: _lib.voikko_set_ignore_nonwords(self._handle, int(v))
    def set_accept_extra_hyphens(self, v: bool) -> None: _lib.voikko_set_accept_extra_hyphens(self._handle, int(v))
    def set_accept_missing_hyphens(self, v: bool) -> None: _lib.voikko_set_accept_missing_hyphens(self._handle, int(v))
    def set_accept_titles_in_gc(self, v: bool) -> None: _lib.voikko_set_accept_titles_in_gc(self._handle, int(v))
    def set_accept_unfinished_paragraphs_in_gc(self, v: bool) -> None: _lib.voikko_set_accept_unfinished_paragraphs_in_gc(self._handle, int(v))
    def set_hyphenate_unknown_words(self, v: bool) -> None: _lib.voikko_set_hyphenate_unknown_words(self._handle, int(v))
    def set_accept_bulleted_lists_in_gc(self, v: bool) -> None: _lib.voikko_set_accept_bulleted_lists_in_gc(self._handle, int(v))
    def set_min_hyphenated_word_length(self, v: int) -> None: _lib.voikko_set_min_hyphenated_word_length(self._handle, v)
    def set_max_suggestions(self, v: int) -> None: _lib.voikko_set_max_suggestions(self._handle, v)
    def set_speller_cache_size(self, v: int) -> None: _lib.voikko_set_speller_cache_size(self._handle, v)

    @staticmethod
    def version() -> str:
        """Get library version."""
        return _lib.voikko_version().decode("utf-8")

    def _check_handle(self) -> None:
        if not self._handle:
            raise RuntimeError("Voikko instance has been terminated")
