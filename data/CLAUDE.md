# data

Linguistic reference data and affix rules for Finnish. Used by the `kotus-diff` tool and as reference material for dictionary development.

## Key Files

- `subst.aff` (46KB) -- Noun (substantive) affix rules for all Finnish inflection classes (Kotus classes 1-51). Defines declension patterns with consonant gradation, possessive suffixes, and compound word rules.
- `verb.aff` (15KB) -- Verb affix rules for Finnish conjugation classes.
- `pohja.aff` -- Base affix file header (encoding, compound rules, possessive suffix definitions).
- `kotus-diff-ignore.txt` (21KB) -- Words to ignore when comparing Joukahainen vocabulary against the KOTUS (Research Institute for the Languages of Finland) word list. Used by `tools/bin/kotus-diff`.
- `sitaattilainat.txt` -- Foreign loan words (citation loans) that don't follow Finnish noun inflection.

## words/

XML word list format definition and examples:

- `wordlist.dtd` -- DTD schema for Joukahainen word list XML format. Defines the structure: `<word>` with `<forms>`, `<classes>`, `<inflection>`, `<usage>`, `<compounding>`, `<derivation>`, `<style>`, `<frequency>`, `<info>`.
- `fi_FI-example.xml` -- Example word entry showing all possible elements and flags.
- `flags.txt` -- Complete flag reference mapping Joukahainen IDs to flag names and descriptions (inflection, usage, compounding, derivation, style flags).
