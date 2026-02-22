/* The contents of this file are subject to the Mozilla Public License Version
 * 1.1 (the "License"); you may not use this file except in compliance with
 * the License. You may obtain a copy of the License at
 * http://www.mozilla.org/MPL/
 *
 * Software distributed under the License is distributed on an "AS IS" basis,
 * WITHOUT WARRANTY OF ANY KIND, either express or implied. See the License
 * for the specific language governing rights and limitations under the
 * License.
 *
 * The Original Code is Libvoikko: Library of natural language processing tools.
 * The Initial Developer of the Original Code is Harri Pitkanen <hatapitk@iki.fi>.
 * Portions created by the Initial Developer are Copyright (C) 2010 - 2012
 * the Initial Developer. All Rights Reserved.
 *
 * Alternatively, the contents of this file may be used under the terms of
 * either the GNU General Public License Version 2 or later (the "GPL"), or
 * the GNU Lesser General Public License Version 2.1 or later (the "LGPL"),
 * in which case the provisions of the GPL or the LGPL are applicable instead
 * of those above. If you wish to allow use of your version of this file only
 * under the terms of either the GPL or the LGPL, and not to allow others to
 * use your version of this file under the terms of the MPL, indicate your
 * decision by deleting the provisions above and replace them with the notice
 * and other provisions required by the GPL or the LGPL. If you do not delete
 * the provisions above, a recipient may use your version of this file under
 * the terms of any one of the MPL, the GPL or the LGPL.
 *********************************************************************************/

package org.puimula.libvoikko;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;

import com.sun.jna.Library;
import com.sun.jna.Memory;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.Structure;
import com.sun.jna.ptr.PointerByReference;

/**
 * Java wrapper for the Rust voikko-ffi cdylib (libvoikko_ffi).
 *
 * <p>This class uses JNA to call the Rust FFI functions directly, replacing the
 * old C++ libvoikko JNA binding. The Rust library takes raw dictionary bytes
 * via {@code voikko_new} rather than a filesystem path, so the constructor
 * reads {@code mor.vfst} (and optionally {@code autocorr.vfst}) from the
 * supplied dictionary directory.
 *
 * <p>Methods are synchronized so that instances are safe for use from multiple
 * threads. For best performance in heavily multithreaded applications, create
 * separate instances per thread.
 *
 * <p>Implements {@link AutoCloseable} so it can be used in try-with-resources.
 */
public class VoikkoRust implements AutoCloseable {

    // ── JNA Structure types matching repr(C) Rust structs ──────────

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     char **keys;    // NULL-terminated
     *     char **values;  // NULL-terminated, parallel to keys
     * } VoikkoAnalysis;
     * </pre>
     */
    @Structure.FieldOrder({"keys", "values"})
    public static class NativeVoikkoAnalysis extends Structure {
        public Pointer keys;
        public Pointer values;

        public NativeVoikkoAnalysis() { super(); }
        public NativeVoikkoAnalysis(Pointer p) { super(p); read(); }
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     VoikkoAnalysis *analyses;
     *     size_t count;
     * } VoikkoAnalysisArray;
     * </pre>
     */
    @Structure.FieldOrder({"analyses", "count"})
    public static class NativeVoikkoAnalysisArray extends Structure implements Structure.ByValue {
        public Pointer analyses;
        public long count;
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     int error_code;
     *     size_t start_pos;
     *     size_t error_len;
     *     char *short_description;
     *     char **suggestions;  // NULL-terminated
     * } VoikkoGrammarError;
     * </pre>
     */
    @Structure.FieldOrder({"error_code", "start_pos", "error_len", "short_description", "suggestions"})
    public static class NativeVoikkoGrammarError extends Structure {
        public int error_code;
        public long start_pos;
        public long error_len;
        public Pointer short_description;
        public Pointer suggestions;

        public NativeVoikkoGrammarError() { super(); }
        public NativeVoikkoGrammarError(Pointer p) { super(p); read(); }
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     VoikkoGrammarError *errors;
     *     size_t count;
     * } VoikkoGrammarErrorArray;
     * </pre>
     */
    @Structure.FieldOrder({"errors", "count"})
    public static class NativeVoikkoGrammarErrorArray extends Structure implements Structure.ByValue {
        public Pointer errors;
        public long count;
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     int token_type;
     *     char *text;
     *     size_t position;
     * } VoikkoToken;
     * </pre>
     */
    @Structure.FieldOrder({"token_type", "text", "position"})
    public static class NativeVoikkoToken extends Structure {
        public int token_type;
        public Pointer text;
        public long position;

        public NativeVoikkoToken() { super(); }
        public NativeVoikkoToken(Pointer p) { super(p); read(); }
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     VoikkoToken *tokens;
     *     size_t count;
     * } VoikkoTokenArray;
     * </pre>
     */
    @Structure.FieldOrder({"tokens", "count"})
    public static class NativeVoikkoTokenArray extends Structure implements Structure.ByValue {
        public Pointer tokens;
        public long count;
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     int sentence_type;
     *     size_t sentence_len;
     * } VoikkoSentence;
     * </pre>
     */
    @Structure.FieldOrder({"sentence_type", "sentence_len"})
    public static class NativeVoikkoSentence extends Structure {
        public int sentence_type;
        public long sentence_len;

        public NativeVoikkoSentence() { super(); }
        public NativeVoikkoSentence(Pointer p) { super(p); read(); }
    }

    /**
     * Mirrors the C struct:
     * <pre>
     * typedef struct {
     *     VoikkoSentence *sentences;
     *     size_t count;
     * } VoikkoSentenceArray;
     * </pre>
     */
    @Structure.FieldOrder({"sentences", "count"})
    public static class NativeVoikkoSentenceArray extends Structure implements Structure.ByValue {
        public Pointer sentences;
        public long count;
    }

    // ── JNA Library interface ──────────────────────────────────────

    /**
     * JNA mapping of the voikko-ffi C API.
     *
     * <p>All functions use the C calling convention. Struct-by-value returns
     * (analysis array, grammar error array, token array, sentence array) are
     * represented by their JNA Structure ByValue counterparts.
     */
    public interface VoikkoFfiLibrary extends Library {

        // Handle lifecycle
        Pointer voikko_new(Pointer mor_data, long mor_len,
                           Pointer autocorr_data, long autocorr_len,
                           PointerByReference error_out);
        void voikko_free(Pointer handle);

        // Spell checking
        int voikko_spell(Pointer handle, String word);
        Pointer voikko_suggest(Pointer handle, String word);

        // Morphological analysis
        NativeVoikkoAnalysisArray voikko_analyze(Pointer handle, String word);
        void voikko_free_analyses(NativeVoikkoAnalysisArray arr);

        // Hyphenation
        Pointer voikko_hyphenate(Pointer handle, String word);
        Pointer voikko_insert_hyphens(Pointer handle, String word,
                                      String separator, int allow_context_changes);

        // Grammar checking
        NativeVoikkoGrammarErrorArray voikko_grammar_errors(Pointer handle,
                                                            String text,
                                                            String language);
        void voikko_free_grammar_errors(NativeVoikkoGrammarErrorArray arr);

        // Tokenization
        NativeVoikkoTokenArray voikko_tokens(Pointer handle, String text);
        void voikko_free_tokens(NativeVoikkoTokenArray arr);

        // Sentence detection
        NativeVoikkoSentenceArray voikko_sentences(Pointer handle, String text);
        void voikko_free_sentences(NativeVoikkoSentenceArray arr);

        // Option setters (boolean)
        void voikko_set_ignore_dot(Pointer handle, int value);
        void voikko_set_ignore_numbers(Pointer handle, int value);
        void voikko_set_ignore_uppercase(Pointer handle, int value);
        void voikko_set_no_ugly_hyphenation(Pointer handle, int value);
        void voikko_set_accept_first_uppercase(Pointer handle, int value);
        void voikko_set_accept_all_uppercase(Pointer handle, int value);
        void voikko_set_ocr_suggestions(Pointer handle, int value);
        void voikko_set_ignore_nonwords(Pointer handle, int value);
        void voikko_set_accept_extra_hyphens(Pointer handle, int value);
        void voikko_set_accept_missing_hyphens(Pointer handle, int value);
        void voikko_set_accept_titles_in_gc(Pointer handle, int value);
        void voikko_set_accept_unfinished_paragraphs_in_gc(Pointer handle, int value);
        void voikko_set_hyphenate_unknown_words(Pointer handle, int value);
        void voikko_set_accept_bulleted_lists_in_gc(Pointer handle, int value);

        // Option setters (integer)
        void voikko_set_min_hyphenated_word_length(Pointer handle, int value);
        void voikko_set_max_suggestions(Pointer handle, int value);
        void voikko_set_speller_cache_size(Pointer handle, int value);

        // Utility
        Pointer voikko_version();
        Pointer voikko_attribute_values(String name);

        // Memory management
        void voikko_free_str(Pointer s);
        void voikko_free_str_array(Pointer arr);
    }

    // ── Library loading ────────────────────────────────────────────

    private static volatile VoikkoFfiLibrary lib;

    private static VoikkoFfiLibrary getLib() {
        if (lib == null) {
            synchronized (VoikkoRust.class) {
                if (lib == null) {
                    lib = Native.load("voikko_ffi", VoikkoFfiLibrary.class);
                }
            }
        }
        return lib;
    }

    /**
     * Explicitly add a search path for the native library before constructing
     * any VoikkoRust instances. Useful when the shared library is not on the
     * default system library path.
     *
     * @param libraryPath directory containing the voikko_ffi shared library
     */
    public static void addLibraryPath(String libraryPath) {
        com.sun.jna.NativeLibrary.addSearchPath("voikko_ffi", libraryPath);
    }

    // ── Instance fields ────────────────────────────────────────────

    private Pointer handle;

    // ── Constructors ───────────────────────────────────────────────

    /**
     * Create a new VoikkoRust instance by loading dictionary files from
     * {@code dictPath}. The directory must contain at least {@code mor.vfst}.
     * If {@code autocorr.vfst} exists it will be loaded too.
     *
     * @param dictPath path to a directory containing {@code mor.vfst}
     *                 (and optionally {@code autocorr.vfst})
     * @throws VoikkoException if the dictionary cannot be loaded
     */
    public VoikkoRust(String dictPath) {
        this(dictPath, true);
    }

    /**
     * Create a new VoikkoRust instance.
     *
     * @param dictPath               path to the dictionary directory
     * @param loadAutocorrIfPresent   whether to load autocorr.vfst if it exists
     * @throws VoikkoException if the dictionary cannot be loaded
     */
    public VoikkoRust(String dictPath, boolean loadAutocorrIfPresent) {
        VoikkoFfiLibrary ffi = getLib();

        // Read mor.vfst (required)
        Path morPath = Paths.get(dictPath, "mor.vfst");
        byte[] morBytes;
        try {
            morBytes = Files.readAllBytes(morPath);
        } catch (IOException e) {
            throw new VoikkoException("Failed to read mor.vfst from " + dictPath + ": " + e.getMessage());
        }

        // Read autocorr.vfst (optional)
        byte[] autocorrBytes = null;
        if (loadAutocorrIfPresent) {
            Path autocorrPath = Paths.get(dictPath, "autocorr.vfst");
            if (Files.exists(autocorrPath)) {
                try {
                    autocorrBytes = Files.readAllBytes(autocorrPath);
                } catch (IOException e) {
                    // Non-fatal: autocorr is optional
                }
            }
        }

        // Copy bytes into native memory
        Memory morMem = new Memory(morBytes.length);
        morMem.write(0, morBytes, 0, morBytes.length);

        Memory autocorrMem = null;
        long autocorrLen = 0;
        if (autocorrBytes != null && autocorrBytes.length > 0) {
            autocorrMem = new Memory(autocorrBytes.length);
            autocorrMem.write(0, autocorrBytes, 0, autocorrBytes.length);
            autocorrLen = autocorrBytes.length;
        }

        PointerByReference errorOut = new PointerByReference();
        handle = ffi.voikko_new(
            morMem, morBytes.length,
            autocorrMem != null ? autocorrMem : Pointer.NULL, autocorrLen,
            errorOut
        );

        if (handle == null || handle == Pointer.NULL) {
            String errMsg = "Unknown error";
            Pointer errPtr = errorOut.getValue();
            if (errPtr != null && errPtr != Pointer.NULL) {
                errMsg = errPtr.getString(0, "UTF-8");
                ffi.voikko_free_str(errPtr);
            }
            throw new VoikkoException("Failed to initialize Voikko: " + errMsg);
        }
    }

    // ── AutoCloseable / lifecycle ──────────────────────────────────

    /**
     * Releases the native resources. After calling this method, all other
     * methods will throw {@link VoikkoException}.
     *
     * <p>Equivalent to the old {@code Voikko.terminate()} method.
     */
    @Override
    public synchronized void close() {
        if (handle != null && handle != Pointer.NULL) {
            getLib().voikko_free(handle);
            handle = null;
        }
    }

    /**
     * Alias for {@link #close()} to match the old Voikko API.
     */
    public void terminate() {
        close();
    }

    @SuppressWarnings("deprecation")
    @Override
    protected void finalize() throws Throwable {
        close();
        super.finalize();
    }

    private void requireValidHandle() {
        if (handle == null || handle == Pointer.NULL) {
            throw new VoikkoException("Attempt to use VoikkoRust instance after close() was called");
        }
    }

    // ── Spell checking ─────────────────────────────────────────────

    /**
     * Check the spelling of a word.
     *
     * @param word the word to check
     * @return {@code true} if the word is correct, {@code false} otherwise
     */
    public synchronized boolean spell(String word) {
        requireValidHandle();
        if (!isValidInput(word)) {
            return false;
        }
        int result = getLib().voikko_spell(handle, word);
        return result == 1;
    }

    /**
     * Generate spelling suggestions for a (misspelled) word.
     *
     * @param word the word to suggest corrections for
     * @return list of suggested spellings; empty list if the word is correct
     *         or an error occurs
     */
    public synchronized List<String> suggest(String word) {
        requireValidHandle();
        if (!isValidInput(word)) {
            return Collections.emptyList();
        }
        VoikkoFfiLibrary ffi = getLib();
        Pointer arrPtr = ffi.voikko_suggest(handle, word);
        if (arrPtr == null || arrPtr == Pointer.NULL) {
            return Collections.emptyList();
        }
        try {
            return readNullTerminatedStringArray(arrPtr);
        } finally {
            ffi.voikko_free_str_array(arrPtr);
        }
    }

    // ── Morphological analysis ─────────────────────────────────────

    /**
     * Analyze the morphology of a word.
     *
     * @param word the word to analyze
     * @return list of analysis results (each result is a key-value map)
     */
    public synchronized List<Analysis> analyze(String word) {
        requireValidHandle();
        if (!isValidInput(word)) {
            return Collections.emptyList();
        }
        VoikkoFfiLibrary ffi = getLib();
        NativeVoikkoAnalysisArray arr = ffi.voikko_analyze(handle, word);
        try {
            int count = (int) arr.count;
            if (count == 0 || arr.analyses == null || arr.analyses == Pointer.NULL) {
                return Collections.emptyList();
            }

            int structSize = new NativeVoikkoAnalysis().size();
            List<Analysis> result = new ArrayList<>(count);
            for (int i = 0; i < count; i++) {
                NativeVoikkoAnalysis na = new NativeVoikkoAnalysis(
                    arr.analyses.share(((long) i) * structSize)
                );
                Analysis analysis = new Analysis();

                if (na.keys != null && na.keys != Pointer.NULL
                    && na.values != null && na.values != Pointer.NULL) {
                    Pointer[] keyPtrs = readNullTerminatedPointerArray(na.keys);
                    Pointer[] valPtrs = readNullTerminatedPointerArray(na.values);
                    int pairCount = Math.min(keyPtrs.length, valPtrs.length);
                    for (int j = 0; j < pairCount; j++) {
                        String key = keyPtrs[j].getString(0, "UTF-8");
                        String value = valPtrs[j].getString(0, "UTF-8");
                        analysis.put(key, value);
                    }
                }
                result.add(analysis);
            }
            return result;
        } finally {
            ffi.voikko_free_analyses(arr);
        }
    }

    // ── Hyphenation ────────────────────────────────────────────────

    /**
     * Return a character pattern that describes the hyphenation of a word.
     *
     * <ul>
     * <li>{@code ' '} = no hyphenation at this character</li>
     * <li>{@code '-'} = hyphenation point (character preserved)</li>
     * <li>{@code '='} = hyphenation point (character replaced by hyphen)</li>
     * </ul>
     *
     * @param word the word to hyphenate
     * @return hyphenation pattern string
     */
    public synchronized String getHyphenationPattern(String word) {
        requireValidHandle();
        if (!isValidInput(word)) {
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < word.length(); i++) {
                sb.append(' ');
            }
            return sb.toString();
        }
        VoikkoFfiLibrary ffi = getLib();
        Pointer ptr = ffi.voikko_hyphenate(handle, word);
        if (ptr == null || ptr == Pointer.NULL) {
            StringBuilder sb = new StringBuilder();
            for (int i = 0; i < word.length(); i++) {
                sb.append(' ');
            }
            return sb.toString();
        }
        try {
            return ptr.getString(0, "UTF-8");
        } finally {
            ffi.voikko_free_str(ptr);
        }
    }

    /**
     * Return the word in fully hyphenated form using the default separator "-".
     *
     * @param word the word to hyphenate
     * @return hyphenated word
     */
    public String hyphenate(String word) {
        return hyphenate(word, "-", true);
    }

    /**
     * Return the word in fully hyphenated form with a custom separator.
     *
     * <p>For example, to insert soft hyphens for automatic HTML hyphenation:
     * {@code hyphenate(word, "&amp;shy;", false)}
     *
     * @param word                  the word to hyphenate
     * @param separator             string to insert at hyphenation points
     * @param allowContextChanges   whether to allow changes beyond simple insertion
     * @return the hyphenated word
     */
    public synchronized String hyphenate(String word, String separator, boolean allowContextChanges) {
        requireValidHandle();
        if (!isValidInput(word) || !isValidInput(separator)) {
            return word;
        }
        VoikkoFfiLibrary ffi = getLib();
        Pointer ptr = ffi.voikko_insert_hyphens(handle, word, separator,
            allowContextChanges ? 1 : 0);
        if (ptr == null || ptr == Pointer.NULL) {
            return word;
        }
        try {
            return ptr.getString(0, "UTF-8");
        } finally {
            ffi.voikko_free_str(ptr);
        }
    }

    // ── Grammar checking ───────────────────────────────────────────

    /**
     * Check the given text for grammar errors.
     *
     * <p>Accepts multiple paragraphs separated by newline characters (the FFI
     * layer handles paragraph splitting internally).
     *
     * @param text     the text to check
     * @param language BCP 47 language tag for error descriptions (e.g. "fi")
     * @return list of grammar errors
     */
    public synchronized List<GrammarError> grammarErrors(String text, String language) {
        requireValidHandle();
        if (!isValidInput(text)) {
            return Collections.emptyList();
        }
        VoikkoFfiLibrary ffi = getLib();
        NativeVoikkoGrammarErrorArray arr = ffi.voikko_grammar_errors(handle, text, language);
        try {
            int count = (int) arr.count;
            if (count == 0 || arr.errors == null || arr.errors == Pointer.NULL) {
                return Collections.emptyList();
            }

            int structSize = new NativeVoikkoGrammarError().size();
            List<GrammarError> result = new ArrayList<>(count);
            for (int i = 0; i < count; i++) {
                NativeVoikkoGrammarError ne = new NativeVoikkoGrammarError(
                    arr.errors.share(((long) i) * structSize)
                );

                // Read suggestions
                List<String> suggestions;
                if (ne.suggestions != null && ne.suggestions != Pointer.NULL) {
                    suggestions = readNullTerminatedStringArray(ne.suggestions);
                } else {
                    suggestions = Collections.emptyList();
                }

                // Read short description
                String shortDescription = "";
                if (ne.short_description != null && ne.short_description != Pointer.NULL) {
                    shortDescription = ne.short_description.getString(0, "UTF-8");
                }

                result.add(new GrammarError(
                    ne.error_code,
                    (int) ne.start_pos,
                    (int) ne.error_len,
                    suggestions,
                    shortDescription
                ));
            }
            return result;
        } finally {
            ffi.voikko_free_grammar_errors(arr);
        }
    }

    /**
     * Check text for grammar errors using Finnish ("fi") as the description
     * language.
     *
     * @param text the text to check
     * @return list of grammar errors
     */
    public List<GrammarError> grammarErrors(String text) {
        return grammarErrors(text, "fi");
    }

    // ── Tokenization ───────────────────────────────────────────────

    private static final TokenType[] TOKEN_TYPE_VALUES = TokenType.values();

    /**
     * Tokenize natural language text.
     *
     * @param text the text to tokenize
     * @return list of tokens
     */
    public synchronized List<Token> tokens(String text) {
        requireValidHandle();
        if (!isValidInput(text)) {
            return Collections.emptyList();
        }
        VoikkoFfiLibrary ffi = getLib();
        NativeVoikkoTokenArray arr = ffi.voikko_tokens(handle, text);
        try {
            int count = (int) arr.count;
            if (count == 0 || arr.tokens == null || arr.tokens == Pointer.NULL) {
                return Collections.emptyList();
            }

            int structSize = new NativeVoikkoToken().size();
            List<Token> result = new ArrayList<>(count);
            for (int i = 0; i < count; i++) {
                NativeVoikkoToken nt = new NativeVoikkoToken(
                    arr.tokens.share(((long) i) * structSize)
                );

                TokenType tokenType;
                if (nt.token_type >= 0 && nt.token_type < TOKEN_TYPE_VALUES.length) {
                    tokenType = TOKEN_TYPE_VALUES[nt.token_type];
                } else {
                    tokenType = TokenType.UNKNOWN;
                }

                String tokenText = "";
                if (nt.text != null && nt.text != Pointer.NULL) {
                    tokenText = nt.text.getString(0, "UTF-8");
                }

                result.add(new Token(tokenType, tokenText, (int) nt.position));
            }
            return result;
        } finally {
            ffi.voikko_free_tokens(arr);
        }
    }

    // ── Sentence detection ─────────────────────────────────────────

    private static final SentenceStartType[] SENTENCE_TYPE_VALUES = SentenceStartType.values();

    /**
     * Split natural language text into sentences.
     *
     * @param text the text to split
     * @return list of sentences
     */
    public synchronized List<Sentence> sentences(String text) {
        requireValidHandle();
        if (!isValidInput(text)) {
            return Collections.singletonList(new Sentence(text, SentenceStartType.NONE));
        }
        VoikkoFfiLibrary ffi = getLib();
        NativeVoikkoSentenceArray arr = ffi.voikko_sentences(handle, text);
        try {
            int count = (int) arr.count;
            if (count == 0 || arr.sentences == null || arr.sentences == Pointer.NULL) {
                return Collections.singletonList(new Sentence(text, SentenceStartType.NONE));
            }

            int structSize = new NativeVoikkoSentence().size();
            List<Sentence> result = new ArrayList<>(count);
            int offset = 0;
            for (int i = 0; i < count; i++) {
                NativeVoikkoSentence ns = new NativeVoikkoSentence(
                    arr.sentences.share(((long) i) * structSize)
                );

                SentenceStartType stype;
                if (ns.sentence_type >= 0 && ns.sentence_type < SENTENCE_TYPE_VALUES.length) {
                    stype = SENTENCE_TYPE_VALUES[ns.sentence_type];
                } else {
                    stype = SentenceStartType.NONE;
                }

                int len = (int) ns.sentence_len;
                String sentenceText;
                if (offset + len <= text.length()) {
                    sentenceText = text.substring(offset, offset + len);
                } else {
                    sentenceText = text.substring(offset);
                }
                result.add(new Sentence(sentenceText, stype));
                offset += len;
            }
            return result;
        } finally {
            ffi.voikko_free_sentences(arr);
        }
    }

    // ── Boolean option setters ─────────────────────────────────────

    /**
     * Ignore dot at the end of the word (needed for some word processors).
     * Default: false
     */
    public synchronized void setIgnoreDot(boolean value) {
        requireValidHandle();
        getLib().voikko_set_ignore_dot(handle, boolToInt(value));
    }

    /**
     * Ignore words containing numbers.
     * Default: false
     */
    public synchronized void setIgnoreNumbers(boolean value) {
        requireValidHandle();
        getLib().voikko_set_ignore_numbers(handle, boolToInt(value));
    }

    /**
     * Accept words written completely in uppercase without checking them.
     * Default: false
     */
    public synchronized void setIgnoreUppercase(boolean value) {
        requireValidHandle();
        getLib().voikko_set_ignore_uppercase(handle, boolToInt(value));
    }

    /**
     * Do not insert ugly but correct hyphenation positions.
     * Default: false
     */
    public synchronized void setNoUglyHyphenation(boolean value) {
        requireValidHandle();
        getLib().voikko_set_no_ugly_hyphenation(handle, boolToInt(value));
    }

    /**
     * Accept words even when the first letter is uppercase.
     * Default: true
     */
    public synchronized void setAcceptFirstUppercase(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_first_uppercase(handle, boolToInt(value));
    }

    /**
     * Accept words when all letters are uppercase (still checks the word).
     * Default: true
     */
    public synchronized void setAcceptAllUppercase(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_all_uppercase(handle, boolToInt(value));
    }

    /**
     * Use OCR-optimized suggestion strategy.
     * Default: false (TYPO strategy)
     */
    public synchronized void setOcrSuggestions(boolean value) {
        requireValidHandle();
        getLib().voikko_set_ocr_suggestions(handle, boolToInt(value));
    }

    /**
     * Set the suggestion strategy.
     *
     * @param strategy the strategy to use
     */
    public synchronized void setSuggestionStrategy(SuggestionStrategy strategy) {
        switch (strategy) {
            case OCR:
                setOcrSuggestions(true);
                break;
            case TYPO:
                setOcrSuggestions(false);
                break;
        }
    }

    /**
     * Ignore non-words such as URLs and email addresses (spell checking only).
     * Default: true
     */
    public synchronized void setIgnoreNonwords(boolean value) {
        requireValidHandle();
        getLib().voikko_set_ignore_nonwords(handle, boolToInt(value));
    }

    /**
     * Allow some extra hyphens in words (spell checking only).
     * Default: false
     */
    public synchronized void setAcceptExtraHyphens(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_extra_hyphens(handle, boolToInt(value));
    }

    /**
     * Accept missing hyphens at word boundaries (spell checking only).
     * Default: false
     */
    public synchronized void setAcceptMissingHyphens(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_missing_hyphens(handle, boolToInt(value));
    }

    /**
     * Accept incomplete sentences in titles/headings (grammar checking only).
     * Default: false
     */
    public synchronized void setAcceptTitlesInGc(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_titles_in_gc(handle, boolToInt(value));
    }

    /**
     * Accept incomplete sentences at end of paragraph (grammar checking only).
     * Default: false
     */
    public synchronized void setAcceptUnfinishedParagraphsInGc(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_unfinished_paragraphs_in_gc(handle, boolToInt(value));
    }

    /**
     * Hyphenate unknown words.
     * Default: true
     */
    public synchronized void setHyphenateUnknownWords(boolean value) {
        requireValidHandle();
        getLib().voikko_set_hyphenate_unknown_words(handle, boolToInt(value));
    }

    /**
     * Accept paragraphs valid within bulleted lists (grammar checking only).
     * Default: false
     */
    public synchronized void setAcceptBulletedListsInGc(boolean value) {
        requireValidHandle();
        getLib().voikko_set_accept_bulleted_lists_in_gc(handle, boolToInt(value));
    }

    // ── Integer option setters ─────────────────────────────────────

    /**
     * Set the minimum length for words that may be hyphenated.
     * Default: 2
     *
     * @param length minimum word length
     */
    public synchronized void setMinHyphenatedWordLength(int length) {
        requireValidHandle();
        getLib().voikko_set_min_hyphenated_word_length(handle, length);
    }

    /**
     * Set the maximum number of suggestions returned by {@link #suggest(String)}.
     *
     * @param count maximum number of suggestions
     */
    public synchronized void setMaxSuggestions(int count) {
        requireValidHandle();
        getLib().voikko_set_max_suggestions(handle, count);
    }

    /**
     * Set the speller cache size.
     * 0 = default, 1 = twice the default, -1 = disabled.
     *
     * @param sizeParam cache size parameter
     */
    public synchronized void setSpellerCacheSize(int sizeParam) {
        requireValidHandle();
        getLib().voikko_set_speller_cache_size(handle, sizeParam);
    }

    // ── Utility ────────────────────────────────────────────────────

    /**
     * Get the Voikko library version string.
     *
     * @return version string (e.g. "0.1.0")
     */
    public static String version() {
        Pointer ptr = getLib().voikko_version();
        if (ptr == null || ptr == Pointer.NULL) {
            return "unknown";
        }
        // voikko_version returns a static pointer -- do NOT free
        return ptr.getString(0, "UTF-8");
    }

    /**
     * Get the list of possible values for a morphological analysis attribute.
     *
     * @param attributeName name of the attribute
     * @return list of possible values, or {@code null} if the attribute does
     *         not exist or has no finite set of values
     */
    public List<String> attributeValues(String attributeName) {
        if (!isValidInput(attributeName)) {
            return null;
        }
        Pointer ptr = getLib().voikko_attribute_values(attributeName);
        if (ptr == null || ptr == Pointer.NULL) {
            return null;
        }
        // voikko_attribute_values returns static pointers -- do NOT free
        return readNullTerminatedStringArray(ptr);
    }

    // ── Internal helpers ───────────────────────────────────────────

    private static boolean isValidInput(String s) {
        return s != null && s.indexOf('\0') == -1;
    }

    private static int boolToInt(boolean value) {
        return value ? 1 : 0;
    }

    /**
     * Read a NULL-terminated array of C string pointers into a Java list.
     */
    private static List<String> readNullTerminatedStringArray(Pointer arrayPtr) {
        List<String> result = new ArrayList<>();
        int i = 0;
        while (true) {
            Pointer strPtr = arrayPtr.getPointer(((long) i) * Native.POINTER_SIZE);
            if (strPtr == null || strPtr == Pointer.NULL) {
                break;
            }
            result.add(strPtr.getString(0, "UTF-8"));
            i++;
        }
        return result;
    }

    /**
     * Read a NULL-terminated array of pointers into a Java array.
     */
    private static Pointer[] readNullTerminatedPointerArray(Pointer arrayPtr) {
        List<Pointer> result = new ArrayList<>();
        int i = 0;
        while (true) {
            Pointer p = arrayPtr.getPointer(((long) i) * Native.POINTER_SIZE);
            if (p == null || p == Pointer.NULL) {
                break;
            }
            result.add(p);
            i++;
        }
        return result.toArray(new Pointer[0]);
    }
}
