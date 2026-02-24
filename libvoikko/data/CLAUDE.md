# data

Grammar checker help content.

## Files

- `gchelp.xml` -- Finnish-language help text for each grammar error code (1-18). Each `<error>` element contains a `<description>` (short label) and `<help>` (detailed explanation in CDATA with HTML). Used by `tools/bin/voikko-gchelp-webpages` to generate help pages and by the grammar checker UI to display user-facing error descriptions.
