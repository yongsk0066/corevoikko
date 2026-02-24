# tools

Python CLI utilities for Voikko dictionary development and testing. Most scripts require `libvoikko` Python module and developer preferences configured via `voikko_dev_prefs.py`.

These tools are for dictionary maintainers. They are not part of the Rust build or the npm package.

## bin/ -- CLI Scripts

- `voikkotest` -- Main regression test runner. Compares spell/grammar/hyphenation results against baseline files in `tests/voikkotest/`. Requires developer preferences.
- `voikko-build-dicts` -- Automates building multiple dictionary variants with different vocabulary options. Run from `voikko-fi/` directory.
- `voikko-gc-pretty` -- Pretty-prints grammar checker output from stdin text. Useful for diffable grammar traces.
- `voikko-readability` -- Calculates Flesch Reading Ease and Flesch-Kincaid Grade Level for Finnish text from stdin.
- `voikko-convert-to-baseform` -- Converts running text to base forms and outputs frequency list.
- `voikko-inflect-word` -- Generates all inflected forms of a Finnish word.
- `voikko-conllu` -- Produces and compares CoNLL-U formatted morphological data.
- `voikko-gchelp-webpages` -- Generates grammar checker help web pages from `libvoikko/data/gchelp.xml`.
- `kotus-diff` -- Compares Joukahainen vocabulary against the KOTUS (Research Institute for the Languages of Finland) word list.
- `wp-wordlist` -- Extracts word frequency lists from Wikipedia XML dumps.
- `ooovoikkotest` -- Tests Voikko integration with OpenOffice/LibreOffice via python-uno.
- `voikkodiff` -- Shell script to diff baseline vs. current spell check results.
- `anagrammivoikko` -- Finnish anagram generator.
- `voikko-cppcheck` -- Runs cppcheck on C++ sources (legacy).
- `voikko-valgrind-libvoikko` -- Runs valgrind memory checks on libvoikko binaries (legacy).

## pylib/ -- Shared Python Modules

- `voikkoutils.py` -- Developer preferences loader, XML vocabulary parser, word class constants.
- `voikkoinfl.py` -- Finnish word inflection library (generates affix rules).
- `voikkostatistics.py` -- Text readability statistics (Flesch, syllable counting).

## doc/

- `voikko_dev_prefs.py` -- Template for developer preferences file. Copy to Python path and configure paths for `corevoikko`, `voikkotest_dir`, `libvoikko_bin`, etc.
