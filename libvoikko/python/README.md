# Python Binding for Voikko

Python interface to libvoikko, providing Finnish spell checking, morphological analysis, hyphenation, grammar checking, and tokenization.

## Prerequisites

The native Voikko shared library must be installed and accessible on the system library path:
- macOS: `libvoikko.1.dylib`
- Linux: `libvoikko.so.1`
- Windows: `libvoikko-1.dll`

A Finnish dictionary (VFST format) must also be installed in the standard Voikko search path.

## Usage

```python
import libvoikko

v = libvoikko.Voikko("fi")

v.spell("kissa")       # True
v.suggest("kisssa")     # ['kissa', 'kissaa', ...]
v.analyze("kissa")      # [{'SIJAMUOTO': 'nimento', 'CLASS': 'nimisana', ...}]
v.hyphenate("kissa")    # 'kis-sa'

v.terminate()
```

## Custom Library Path

If the shared library is not on the default search path:

```python
libvoikko.Voikko.setLibrarySearchPath("/path/to/lib")
v = libvoikko.Voikko("fi")
```

## API

See the docstrings in `libvoikko.py` for the full API. Key methods on the `Voikko` class: `spell`, `suggest`, `analyze`, `hyphenate`, `grammarErrors`, `tokens`, `sentences`, `terminate`.

## License

MPL 1.1 / GPL 2+ / LGPL 2.1+ (tri-license)
