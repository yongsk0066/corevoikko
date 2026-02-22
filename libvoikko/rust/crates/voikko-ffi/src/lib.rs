// FFI functions are inherently unsafe — callers must ensure pointer validity.
// Safety contracts are documented per-function in the public API comments.
#![allow(clippy::missing_safety_doc)]

// voikko-ffi: C-compatible FFI layer for VoikkoHandle.
//
// This crate exposes a stable C ABI that can be consumed by any language
// with C FFI support (Python/ctypes, C#/P-Invoke, Common Lisp/CFFI, etc.).
//
// Memory management rules:
// - Opaque `VoikkoHandle` pointer: created by `voikko_new`, freed by `voikko_free`.
// - Returned strings: caller must free with `voikko_free_str`.
// - Returned string arrays: caller must free with `voikko_free_str_array`.
// - Returned analysis/grammar/token/sentence arrays: caller frees with dedicated functions.
// - All input strings are UTF-8 encoded, null-terminated C strings.

use std::ffi::{CStr, CString, c_char, c_int};
use std::ptr;
use std::slice;

use voikko_core::grammar_error;
use voikko_fi::handle::VoikkoHandle;

// ── Handle lifecycle ─────────────────────────────────────────────

/// Create a new Voikko handle from raw dictionary data.
///
/// - `mor_data` + `mor_len`: contents of `mor.vfst` (required)
/// - `autocorr_data` + `autocorr_len`: contents of `autocorr.vfst` (optional, NULL to skip)
///
/// Returns an opaque pointer on success, NULL on failure.
/// On failure, if `error_out` is non-NULL, it receives a heap-allocated error string
/// that the caller must free with `voikko_free_str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_new(
    mor_data: *const u8,
    mor_len: usize,
    autocorr_data: *const u8,
    autocorr_len: usize,
    error_out: *mut *mut c_char,
) -> *mut VoikkoHandle {
    if mor_data.is_null() || mor_len == 0 {
        set_error(error_out, "mor_data is null or empty");
        return ptr::null_mut();
    }

    let mor = unsafe { slice::from_raw_parts(mor_data, mor_len) };
    let autocorr = if autocorr_data.is_null() || autocorr_len == 0 {
        None
    } else {
        Some(unsafe { slice::from_raw_parts(autocorr_data, autocorr_len) })
    };

    match VoikkoHandle::from_bytes(mor, autocorr, "fi") {
        Ok(handle) => Box::into_raw(Box::new(handle)),
        Err(e) => {
            set_error(error_out, &e.to_string());
            ptr::null_mut()
        }
    }
}

/// Free a VoikkoHandle created by `voikko_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free(handle: *mut VoikkoHandle) {
    if !handle.is_null() {
        drop(unsafe { Box::from_raw(handle) });
    }
}

// ── Spell checking ──────────────────────────────────────────────

/// Check whether a word is correctly spelled.
/// Returns 1 for correct, 0 for incorrect, -1 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_spell(handle: *const VoikkoHandle, word: *const c_char) -> c_int {
    let Some(handle) = (unsafe { handle.as_ref() }) else {
        return -1;
    };
    let Some(word) = cstr_to_str(word) else {
        return -1;
    };
    if handle.spell(word) { 1 } else { 0 }
}

/// Generate spelling suggestions.
///
/// Returns a NULL-terminated array of C strings. Caller must free with
/// `voikko_free_str_array`. Returns NULL on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_suggest(
    handle: *const VoikkoHandle,
    word: *const c_char,
) -> *mut *mut c_char {
    let Some(handle) = (unsafe { handle.as_ref() }) else {
        return ptr::null_mut();
    };
    let Some(word) = cstr_to_str(word) else {
        return ptr::null_mut();
    };
    let suggestions = handle.suggest(word);
    strings_to_c_array(&suggestions)
}

// ── Morphological analysis ──────────────────────────────────────

/// Opaque analysis result.
#[repr(C)]
pub struct VoikkoAnalysis {
    /// NULL-terminated array of attribute keys (C strings).
    pub keys: *mut *mut c_char,
    /// NULL-terminated array of attribute values (C strings), parallel to keys.
    pub values: *mut *mut c_char,
}

/// Opaque analysis array result.
#[repr(C)]
pub struct VoikkoAnalysisArray {
    pub analyses: *mut VoikkoAnalysis,
    pub count: usize,
}

/// Perform morphological analysis.
///
/// Returns a heap-allocated `VoikkoAnalysisArray`. Caller must free with
/// `voikko_free_analyses`. Returns a struct with count=0 on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_analyze(
    handle: *const VoikkoHandle,
    word: *const c_char,
) -> VoikkoAnalysisArray {
    let empty = VoikkoAnalysisArray { analyses: ptr::null_mut(), count: 0 };

    let Some(handle) = (unsafe { handle.as_ref() }) else { return empty; };
    let Some(word) = cstr_to_str(word) else { return empty; };

    let analyses = handle.analyze(word);
    let count = analyses.len();
    if count == 0 {
        return empty;
    }

    let mut c_analyses: Vec<VoikkoAnalysis> = Vec::with_capacity(count);
    for a in &analyses {
        let attrs: Vec<(&str, &str)> = a.attributes().iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        let mut keys: Vec<*mut c_char> = Vec::with_capacity(attrs.len() + 1);
        let mut values: Vec<*mut c_char> = Vec::with_capacity(attrs.len() + 1);
        for (k, v) in &attrs {
            keys.push(str_to_c(k));
            values.push(str_to_c(v));
        }
        keys.push(ptr::null_mut()); // NULL terminator
        values.push(ptr::null_mut());

        let keys_ptr = keys.as_mut_ptr();
        let values_ptr = values.as_mut_ptr();
        std::mem::forget(keys);
        std::mem::forget(values);

        c_analyses.push(VoikkoAnalysis { keys: keys_ptr, values: values_ptr });
    }

    let analyses_ptr = c_analyses.as_mut_ptr();
    std::mem::forget(c_analyses);

    VoikkoAnalysisArray { analyses: analyses_ptr, count }
}

/// Free an analysis array returned by `voikko_analyze`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free_analyses(arr: VoikkoAnalysisArray) {
    if arr.analyses.is_null() || arr.count == 0 {
        return;
    }
    let analyses = unsafe { Vec::from_raw_parts(arr.analyses, arr.count, arr.count) };
    for a in analyses {
        free_null_terminated_array(a.keys);
        free_null_terminated_array(a.values);
    }
}

// ── Hyphenation ─────────────────────────────────────────────────

/// Get the hyphenation pattern for a word.
///
/// Returns a heap-allocated C string. Caller must free with `voikko_free_str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_hyphenate(
    handle: *const VoikkoHandle,
    word: *const c_char,
) -> *mut c_char {
    let Some(handle) = (unsafe { handle.as_ref() }) else { return ptr::null_mut(); };
    let Some(word) = cstr_to_str(word) else { return ptr::null_mut(); };
    str_to_c(&handle.hyphenate(word))
}

/// Insert hyphens with a custom separator.
///
/// Returns a heap-allocated C string. Caller must free with `voikko_free_str`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_insert_hyphens(
    handle: *const VoikkoHandle,
    word: *const c_char,
    separator: *const c_char,
    allow_context_changes: c_int,
) -> *mut c_char {
    let Some(handle) = (unsafe { handle.as_ref() }) else { return ptr::null_mut(); };
    let Some(word) = cstr_to_str(word) else { return ptr::null_mut(); };
    let Some(sep) = cstr_to_str(separator) else { return ptr::null_mut(); };
    str_to_c(&handle.insert_hyphens(word, sep, allow_context_changes != 0))
}

// ── Grammar checking ────────────────────────────────────────────

/// Grammar error returned by FFI.
#[repr(C)]
pub struct VoikkoGrammarError {
    pub error_code: c_int,
    pub start_pos: usize,
    pub error_len: usize,
    pub short_description: *mut c_char,
    pub suggestions: *mut *mut c_char,
}

/// Grammar error array.
#[repr(C)]
pub struct VoikkoGrammarErrorArray {
    pub errors: *mut VoikkoGrammarError,
    pub count: usize,
}

/// Check text for grammar errors (multi-paragraph, splits at newlines).
///
/// Returns a `VoikkoGrammarErrorArray`. Caller must free with `voikko_free_grammar_errors`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_grammar_errors(
    handle: *const VoikkoHandle,
    text: *const c_char,
    language: *const c_char,
) -> VoikkoGrammarErrorArray {
    let empty = VoikkoGrammarErrorArray { errors: ptr::null_mut(), count: 0 };

    let Some(handle) = (unsafe { handle.as_ref() }) else { return empty; };
    let Some(text) = cstr_to_str(text) else { return empty; };
    let lang = cstr_to_str(language).unwrap_or("fi");

    let errors = handle.grammar_errors_from_text(text);
    let count = errors.len();
    if count == 0 {
        return empty;
    }

    let mut c_errors: Vec<VoikkoGrammarError> = Vec::with_capacity(count);
    for e in &errors {
        let desc = grammar_error::error_code_description_lang(e.error_code, lang);
        c_errors.push(VoikkoGrammarError {
            error_code: e.error_code,
            start_pos: e.start_pos,
            error_len: e.error_len,
            short_description: str_to_c(desc),
            suggestions: strings_to_c_array(&e.suggestions),
        });
    }

    let ptr = c_errors.as_mut_ptr();
    std::mem::forget(c_errors);

    VoikkoGrammarErrorArray { errors: ptr, count }
}

/// Free a grammar error array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free_grammar_errors(arr: VoikkoGrammarErrorArray) {
    if arr.errors.is_null() || arr.count == 0 {
        return;
    }
    let errors = unsafe { Vec::from_raw_parts(arr.errors, arr.count, arr.count) };
    for e in errors {
        free_c_str(e.short_description);
        free_null_terminated_array(e.suggestions);
    }
}

// ── Tokenization ────────────────────────────────────────────────

/// Token returned by FFI.
#[repr(C)]
pub struct VoikkoToken {
    /// Token type: 1=Word, 2=Punctuation, 3=Whitespace, 4=Unknown, 0=None
    pub token_type: c_int,
    pub text: *mut c_char,
    pub position: usize,
}

/// Token array.
#[repr(C)]
pub struct VoikkoTokenArray {
    pub tokens: *mut VoikkoToken,
    pub count: usize,
}

/// Tokenize text.
///
/// Returns a `VoikkoTokenArray`. Caller must free with `voikko_free_tokens`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_tokens(
    handle: *const VoikkoHandle,
    text: *const c_char,
) -> VoikkoTokenArray {
    let empty = VoikkoTokenArray { tokens: ptr::null_mut(), count: 0 };

    let Some(handle) = (unsafe { handle.as_ref() }) else { return empty; };
    let Some(text) = cstr_to_str(text) else { return empty; };

    let tokens = handle.tokens(text);
    let count = tokens.len();
    if count == 0 {
        return empty;
    }

    let mut c_tokens: Vec<VoikkoToken> = Vec::with_capacity(count);
    for t in &tokens {
        c_tokens.push(VoikkoToken {
            token_type: token_type_to_int(t.token_type),
            text: str_to_c(&t.text),
            position: t.pos,
        });
    }

    let ptr = c_tokens.as_mut_ptr();
    std::mem::forget(c_tokens);

    VoikkoTokenArray { tokens: ptr, count }
}

/// Free a token array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free_tokens(arr: VoikkoTokenArray) {
    if arr.tokens.is_null() || arr.count == 0 {
        return;
    }
    let tokens = unsafe { Vec::from_raw_parts(arr.tokens, arr.count, arr.count) };
    for t in tokens {
        free_c_str(t.text);
    }
}

// ── Sentence detection ──────────────────────────────────────────

/// Sentence returned by FFI.
#[repr(C)]
pub struct VoikkoSentence {
    /// Sentence type: 0=None, 1=NoStart, 2=Probable, 3=Possible
    pub sentence_type: c_int,
    pub sentence_len: usize,
}

/// Sentence array.
#[repr(C)]
pub struct VoikkoSentenceArray {
    pub sentences: *mut VoikkoSentence,
    pub count: usize,
}

/// Detect sentence boundaries.
///
/// Returns a `VoikkoSentenceArray`. Caller must free with `voikko_free_sentences`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_sentences(
    handle: *const VoikkoHandle,
    text: *const c_char,
) -> VoikkoSentenceArray {
    let empty = VoikkoSentenceArray { sentences: ptr::null_mut(), count: 0 };

    let Some(handle) = (unsafe { handle.as_ref() }) else { return empty; };
    let Some(text) = cstr_to_str(text) else { return empty; };

    let sentences = handle.sentences(text);
    let count = sentences.len();
    if count == 0 {
        return empty;
    }

    let mut c_sentences: Vec<VoikkoSentence> = Vec::with_capacity(count);
    for s in &sentences {
        c_sentences.push(VoikkoSentence {
            sentence_type: sentence_type_to_int(s.sentence_type),
            sentence_len: s.sentence_len,
        });
    }

    let ptr = c_sentences.as_mut_ptr();
    std::mem::forget(c_sentences);

    VoikkoSentenceArray { sentences: ptr, count }
}

/// Free a sentence array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free_sentences(arr: VoikkoSentenceArray) {
    if arr.sentences.is_null() || arr.count == 0 {
        return;
    }
    drop(unsafe { Vec::from_raw_parts(arr.sentences, arr.count, arr.count) });
}

// ── Option setters ──────────────────────────────────────────────

macro_rules! bool_setter {
    ($name:ident, $method:ident) => {
        #[unsafe(no_mangle)]
        pub unsafe extern "C" fn $name(handle: *mut VoikkoHandle, value: c_int) {
            if let Some(handle) = unsafe { handle.as_mut() } {
                handle.$method(value != 0);
            }
        }
    };
}

bool_setter!(voikko_set_ignore_dot, set_ignore_dot);
bool_setter!(voikko_set_ignore_numbers, set_ignore_numbers);
bool_setter!(voikko_set_ignore_uppercase, set_ignore_uppercase);
bool_setter!(voikko_set_no_ugly_hyphenation, set_no_ugly_hyphenation);
bool_setter!(voikko_set_accept_first_uppercase, set_accept_first_uppercase);
bool_setter!(voikko_set_accept_all_uppercase, set_accept_all_uppercase);
bool_setter!(voikko_set_ocr_suggestions, set_ocr_suggestions);
bool_setter!(voikko_set_ignore_nonwords, set_ignore_nonwords);
bool_setter!(voikko_set_accept_extra_hyphens, set_accept_extra_hyphens);
bool_setter!(voikko_set_accept_missing_hyphens, set_accept_missing_hyphens);
bool_setter!(voikko_set_accept_titles_in_gc, set_accept_titles_in_gc);
bool_setter!(voikko_set_accept_unfinished_paragraphs_in_gc, set_accept_unfinished_paragraphs_in_gc);
bool_setter!(voikko_set_hyphenate_unknown_words, set_hyphenate_unknown_words);
bool_setter!(voikko_set_accept_bulleted_lists_in_gc, set_accept_bulleted_lists_in_gc);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_set_min_hyphenated_word_length(
    handle: *mut VoikkoHandle,
    value: c_int,
) {
    if let Some(handle) = unsafe { handle.as_mut() } {
        handle.set_min_hyphenated_word_length(value as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_set_max_suggestions(handle: *mut VoikkoHandle, value: c_int) {
    if let Some(handle) = unsafe { handle.as_mut() } {
        handle.set_max_suggestions(value as usize);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_set_speller_cache_size(handle: *mut VoikkoHandle, value: c_int) {
    if let Some(handle) = unsafe { handle.as_mut() } {
        handle.set_speller_cache_size(value as usize);
    }
}

// ── Utility functions ───────────────────────────────────────────

/// Return the library version string.
///
/// The returned pointer is valid for the lifetime of the library (static).
/// Do NOT free this pointer.
#[unsafe(no_mangle)]
pub extern "C" fn voikko_version() -> *const c_char {
    static VERSION: std::sync::LazyLock<CString> =
        std::sync::LazyLock::new(|| CString::new(VoikkoHandle::get_version()).unwrap());
    VERSION.as_ptr()
}

/// Get valid values for an enumerated attribute.
///
/// Returns a NULL-terminated array. The returned pointer and its contents are
/// static — do NOT free them.
/// Returns NULL if the attribute is not recognized.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_attribute_values(
    name: *const c_char,
) -> *const *const c_char {
    let Some(name) = cstr_to_str(name) else { return ptr::null(); };
    // We use a static cache to avoid repeated allocation.
    // This is safe because attribute_values returns &'static data.
    let Some(values) = VoikkoHandle::attribute_values(name) else {
        return ptr::null();
    };

    // Leak a Vec of CString pointers (static lifetime, never freed).
    // This is intentional — attribute values are requested rarely and
    // the set is fixed (12 attributes × small arrays).
    let mut ptrs: Vec<*const c_char> = values
        .iter()
        .map(|v| {
            let cs = CString::new(*v).unwrap();
            let ptr = cs.as_ptr();
            std::mem::forget(cs);
            ptr
        })
        .collect();
    ptrs.push(ptr::null());
    let ptr = ptrs.as_ptr();
    std::mem::forget(ptrs);
    ptr
}

/// Free a heap-allocated C string returned by voikko functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free_str(s: *mut c_char) {
    free_c_str(s);
}

/// Free a NULL-terminated array of C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn voikko_free_str_array(arr: *mut *mut c_char) {
    free_null_terminated_array(arr);
}

// ── Internal helpers ────────────────────────────────────────────

fn cstr_to_str<'a>(s: *const c_char) -> Option<&'a str> {
    if s.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(s) }.to_str().ok()
}

fn str_to_c(s: &str) -> *mut c_char {
    CString::new(s).unwrap_or_default().into_raw()
}

fn set_error(out: *mut *mut c_char, msg: &str) {
    if !out.is_null() {
        unsafe { *out = str_to_c(msg); }
    }
}

fn free_c_str(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

fn strings_to_c_array(strings: &[String]) -> *mut *mut c_char {
    let mut ptrs: Vec<*mut c_char> = strings.iter().map(|s| str_to_c(s)).collect();
    ptrs.push(ptr::null_mut()); // NULL terminator
    let ptr = ptrs.as_mut_ptr();
    std::mem::forget(ptrs);
    ptr
}

fn free_null_terminated_array(arr: *mut *mut c_char) {
    if arr.is_null() {
        return;
    }
    let mut i = 0;
    loop {
        let p = unsafe { *arr.add(i) };
        if p.is_null() {
            break;
        }
        free_c_str(p);
        i += 1;
    }
    // Free the array itself — we know it was allocated as Vec with capacity i+1
    drop(unsafe { Vec::from_raw_parts(arr, i + 1, i + 1) });
}

fn token_type_to_int(tt: voikko_core::enums::TokenType) -> c_int {
    use voikko_core::enums::TokenType;
    match tt {
        TokenType::None => 0,
        TokenType::Word => 1,
        TokenType::Punctuation => 2,
        TokenType::Whitespace => 3,
        TokenType::Unknown => 4,
    }
}

fn sentence_type_to_int(st: voikko_core::enums::SentenceType) -> c_int {
    use voikko_core::enums::SentenceType;
    match st {
        SentenceType::None => 0,
        SentenceType::NoStart => 1,
        SentenceType::Probable => 2,
        SentenceType::Possible => 3,
    }
}
