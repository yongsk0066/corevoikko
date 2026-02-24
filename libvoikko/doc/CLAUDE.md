# doc

Technical documentation for libvoikko implementors, written in Finnish and English.

## Files

- `morphological-analysis.txt` -- Specification for morphological analysis attributes returned by the `Analyzer` interface. Documents the `STRUCTURE`, `BASEFORM`, `CLASS`, `SIJAMUOTO`, `NUMBER`, `PERSON`, `MOOD`, `TENSE`, `NEGATIVE`, `PARTICIPLE`, `POSSESSIVE`, `COMPARISON`, `WORDBASES`, `WORDIDS`, `FSTOUTPUT`, `FOCUS`, and `KYSYMYSLIITE` attributes. This is the primary reference for understanding analysis output from both the Rust and legacy C++ implementations.
- `oikoluku-korjausehdotukset.txt` -- Algorithm for ranking spell correction suggestions (in Finnish). Describes the four-phase weighting system: word class/case weight (p1), compound word penalty (p2), capitalization distance (p3), and generation order (p4). Final score = p1 * p2 * p3 * p4.
