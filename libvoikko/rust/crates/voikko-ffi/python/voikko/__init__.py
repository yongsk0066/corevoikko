"""Voikko Finnish NLP — Python bindings via Rust FFI (ctypes).

Usage:
    from voikko import Voikko

    v = Voikko("/path/to/dict")
    v.spell("koira")      # True
    v.suggest("koirra")   # ["koira", ...]
    v.terminate()
"""

from voikko._binding import Voikko

__all__ = ["Voikko"]
