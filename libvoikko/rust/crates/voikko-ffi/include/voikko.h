/* voikko.h — C FFI for Rust Voikko Finnish NLP library
 *
 * Memory rules:
 * - voikko_new() returns an opaque handle; free with voikko_free().
 * - Functions returning *char: free with voikko_free_str().
 * - Functions returning **char (NULL-terminated): free with voikko_free_str_array().
 * - Struct arrays: free with their dedicated voikko_free_* function.
 * - voikko_version() returns a static pointer — do NOT free.
 * - voikko_attribute_values() returns static pointers — do NOT free.
 * - All input strings must be valid UTF-8, null-terminated.
 */

#ifndef VOIKKO_H
#define VOIKKO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Opaque handle ─────────────────────────────────────────────── */

typedef struct VoikkoHandle VoikkoHandle;

VoikkoHandle *voikko_new(const uint8_t *mor_data, size_t mor_len,
                         const uint8_t *autocorr_data, size_t autocorr_len,
                         char **error_out);
void voikko_free(VoikkoHandle *handle);

/* ── Spell checking ──────────────────────────────────────────── */

int voikko_spell(const VoikkoHandle *handle, const char *word);
char **voikko_suggest(const VoikkoHandle *handle, const char *word);

/* ── Morphological analysis ──────────────────────────────────── */

typedef struct {
    char **keys;    /* NULL-terminated */
    char **values;  /* NULL-terminated, parallel to keys */
} VoikkoAnalysis;

typedef struct {
    VoikkoAnalysis *analyses;
    size_t count;
} VoikkoAnalysisArray;

VoikkoAnalysisArray voikko_analyze(const VoikkoHandle *handle, const char *word);
void voikko_free_analyses(VoikkoAnalysisArray arr);

/* ── Hyphenation ─────────────────────────────────────────────── */

char *voikko_hyphenate(const VoikkoHandle *handle, const char *word);
char *voikko_insert_hyphens(const VoikkoHandle *handle, const char *word,
                            const char *separator, int allow_context_changes);

/* ── Grammar checking ────────────────────────────────────────── */

typedef struct {
    int error_code;
    size_t start_pos;
    size_t error_len;
    char *short_description;
    char **suggestions;  /* NULL-terminated */
} VoikkoGrammarError;

typedef struct {
    VoikkoGrammarError *errors;
    size_t count;
} VoikkoGrammarErrorArray;

VoikkoGrammarErrorArray voikko_grammar_errors(const VoikkoHandle *handle,
                                               const char *text,
                                               const char *language);
void voikko_free_grammar_errors(VoikkoGrammarErrorArray arr);

/* ── Tokenization ────────────────────────────────────────────── */

/* Token types: 0=None, 1=Word, 2=Punctuation, 3=Whitespace, 4=Unknown */
typedef struct {
    int token_type;
    char *text;
    size_t position;
} VoikkoToken;

typedef struct {
    VoikkoToken *tokens;
    size_t count;
} VoikkoTokenArray;

VoikkoTokenArray voikko_tokens(const VoikkoHandle *handle, const char *text);
void voikko_free_tokens(VoikkoTokenArray arr);

/* ── Sentence detection ──────────────────────────────────────── */

/* Sentence types: 0=None, 1=NoStart, 2=Probable, 3=Possible */
typedef struct {
    int sentence_type;
    size_t sentence_len;
} VoikkoSentence;

typedef struct {
    VoikkoSentence *sentences;
    size_t count;
} VoikkoSentenceArray;

VoikkoSentenceArray voikko_sentences(const VoikkoHandle *handle, const char *text);
void voikko_free_sentences(VoikkoSentenceArray arr);

/* ── Option setters ──────────────────────────────────────────── */

void voikko_set_ignore_dot(VoikkoHandle *handle, int value);
void voikko_set_ignore_numbers(VoikkoHandle *handle, int value);
void voikko_set_ignore_uppercase(VoikkoHandle *handle, int value);
void voikko_set_no_ugly_hyphenation(VoikkoHandle *handle, int value);
void voikko_set_accept_first_uppercase(VoikkoHandle *handle, int value);
void voikko_set_accept_all_uppercase(VoikkoHandle *handle, int value);
void voikko_set_ocr_suggestions(VoikkoHandle *handle, int value);
void voikko_set_ignore_nonwords(VoikkoHandle *handle, int value);
void voikko_set_accept_extra_hyphens(VoikkoHandle *handle, int value);
void voikko_set_accept_missing_hyphens(VoikkoHandle *handle, int value);
void voikko_set_accept_titles_in_gc(VoikkoHandle *handle, int value);
void voikko_set_accept_unfinished_paragraphs_in_gc(VoikkoHandle *handle, int value);
void voikko_set_hyphenate_unknown_words(VoikkoHandle *handle, int value);
void voikko_set_accept_bulleted_lists_in_gc(VoikkoHandle *handle, int value);
void voikko_set_min_hyphenated_word_length(VoikkoHandle *handle, int value);
void voikko_set_max_suggestions(VoikkoHandle *handle, int value);
void voikko_set_speller_cache_size(VoikkoHandle *handle, int value);

/* ── Utility ─────────────────────────────────────────────────── */

const char *voikko_version(void);
const char *const *voikko_attribute_values(const char *name);

/* ── Memory management ───────────────────────────────────────── */

void voikko_free_str(char *s);
void voikko_free_str_array(char **arr);

#ifdef __cplusplus
}
#endif

#endif /* VOIKKO_H */
