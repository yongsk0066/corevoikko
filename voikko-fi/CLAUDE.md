# voikko-fi

Finnish morphological dictionary source data for Voikko. This is linguistic data (foma scripts, lexicon files, Python generators), not Rust code.

The build pipeline converts XML vocabulary and handwritten lexicon rules into VFST binary transducers that the Rust runtime (`voikko-fi` crate) reads at load time.

## Build

Requires `foma`, `python3`, and GNU `make`.

```bash
make vvfst                              # build dictionary
make vvfst-install DESTDIR=~/.voikko    # install for current user
make clean                              # remove generated files
make update-vocabulary                  # fetch latest XML from joukahainen.puimula.org
```

Build variants can be tuned with `GENLEX_OPTS`, `VOIKKO_VARIANT`, `VVFST_BASEFORMS`, and `VANHAT_MUODOT`. See README.md for full details.

## Key Directories

- `common/` -- Shared Python modules (`hfconv.py` for morphology conversion, `voikkoutils.py`, `generate_lex_common.py`)
- `vocabulary/` -- Source vocabulary data. `joukahainen.xml` (8.7MB) is the master word database. `flags.txt` documents all vocabulary flags. `autocorrect/` holds autocorrection rules.
- `vvfst/` -- Main build directory for standard Finnish. Contains foma scripts (`main.foma.in`), Python generators (`generate_lex.py`, `generate_taivutuskaavat.py`), handwritten lexicon files (`*.lexc`), and build outputs.
- `vvfst-medicine/` -- Variant build with medical/scientific vocabulary included. Same structure as `vvfst/` but with expanded word list.
- `devenv/` -- Docker development environment for dictionary building.

## Build Outputs (in `vvfst/`)

- `mor.vfst` (3.8MB) -- Morphological analysis transducer. This is what the Rust runtime loads.
- `autocorr.vfst` (11KB) -- Autocorrection transducer.
- `index.txt` -- Dictionary metadata (format version, language, license, description).

These three files are also bundled into the npm package at `libvoikko/js/dict/`.

## Build Pipeline

```
joukahainen.xml ──> generate_lex.py ──> joukahainen-*.lexc ──┐
                                                              ├──> all.lexc ──> foma ──> all.att ──> mor.vfst
taivutuskaavat.lexc.in ──> generate_taivutuskaavat.py ──> taivutuskaavat.lexc ──┘
                    handwritten *.lexc (root, poikkeavat, lukusanat, ...) ──────┘
```
