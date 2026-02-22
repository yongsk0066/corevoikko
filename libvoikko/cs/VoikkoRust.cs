// VoikkoRust.cs — C# P/Invoke binding for the Rust voikko-ffi cdylib.
//
// License: MPL 1.1 / GPL 2+ / LGPL 2.1+ (tri-license, same as libvoikko)
//
// Target: .NET 6+ / C# 10+
//
// Usage:
//   using var voikko = new Voikko("/path/to/vvfst");   // dir with mor.vfst
//   bool ok = voikko.Spell("koira");
//   List<string> sugg = voikko.Suggest("koirra");
//
// The native library is loaded as "voikko_ffi":
//   macOS:   libvoikko_ffi.dylib
//   Linux:   libvoikko_ffi.so
//   Windows: voikko_ffi.dll
//
// Build the Rust cdylib first:
//   cargo build --release -p voikko-ffi

#nullable enable

using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;

namespace VoikkoRust;

// ── Enums ────────────────────────────────────────────────────────

/// <summary>Token types returned by the tokenizer.</summary>
public enum TokenType
{
    None = 0,
    Word = 1,
    Punctuation = 2,
    Whitespace = 3,
    Unknown = 4,
}

/// <summary>Sentence boundary types.</summary>
public enum SentenceType
{
    None = 0,
    NoStart = 1,
    Probable = 2,
    Possible = 3,
}

// ── Result records ───────────────────────────────────────────────

/// <summary>A single morphological analysis (key-value pairs).</summary>
public sealed class Analysis : Dictionary<string, string> { }

/// <summary>A grammar error with position, description, and suggestions.</summary>
public sealed record GrammarError(
    int ErrorCode,
    int StartPos,
    int ErrorLen,
    string ShortDescription,
    IReadOnlyList<string> Suggestions);

/// <summary>A text token.</summary>
public sealed record Token(TokenType Type, string Text, int Position);

/// <summary>A detected sentence boundary.</summary>
public sealed record Sentence(SentenceType Type, int Length);

// ── Native struct mirrors (blittable, for marshalling) ───────────

[StructLayout(LayoutKind.Sequential)]
internal struct NativeAnalysis
{
    public IntPtr Keys;     // char** (NULL-terminated)
    public IntPtr Values;   // char** (NULL-terminated)
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeAnalysisArray
{
    public IntPtr Analyses; // NativeAnalysis*
    public nuint Count;
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeGrammarError
{
    public int ErrorCode;
    public nuint StartPos;
    public nuint ErrorLen;
    public IntPtr ShortDescription; // char*
    public IntPtr Suggestions;      // char** (NULL-terminated)
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeGrammarErrorArray
{
    public IntPtr Errors; // NativeGrammarError*
    public nuint Count;
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeToken
{
    public int TokenType;
    public IntPtr Text;    // char*
    public nuint Position;
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeTokenArray
{
    public IntPtr Tokens; // NativeToken*
    public nuint Count;
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeSentence
{
    public int SentenceType;
    public nuint SentenceLen;
}

[StructLayout(LayoutKind.Sequential)]
internal struct NativeSentenceArray
{
    public IntPtr Sentences; // NativeSentence*
    public nuint Count;
}

// ── P/Invoke declarations ────────────────────────────────────────

internal static partial class Native
{
    private const string Lib = "voikko_ffi";

    // -- Handle lifecycle --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr voikko_new(
        byte[] morData, nuint morLen,
        byte[]? autocorrData, nuint autocorrLen,
        out IntPtr errorOut);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free(IntPtr handle);

    // -- Spell checking --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern int voikko_spell(IntPtr handle, byte[] word);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr voikko_suggest(IntPtr handle, byte[] word);

    // -- Morphological analysis --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern NativeAnalysisArray voikko_analyze(IntPtr handle, byte[] word);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free_analyses(NativeAnalysisArray arr);

    // -- Hyphenation --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr voikko_hyphenate(IntPtr handle, byte[] word);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr voikko_insert_hyphens(
        IntPtr handle, byte[] word, byte[] separator, int allowContextChanges);

    // -- Grammar checking --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern NativeGrammarErrorArray voikko_grammar_errors(
        IntPtr handle, byte[] text, byte[] language);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free_grammar_errors(NativeGrammarErrorArray arr);

    // -- Tokenization --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern NativeTokenArray voikko_tokens(IntPtr handle, byte[] text);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free_tokens(NativeTokenArray arr);

    // -- Sentence detection --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern NativeSentenceArray voikko_sentences(IntPtr handle, byte[] text);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free_sentences(NativeSentenceArray arr);

    // -- Option setters (14 boolean + 3 integer) --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_ignore_dot(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_ignore_numbers(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_ignore_uppercase(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_no_ugly_hyphenation(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_first_uppercase(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_all_uppercase(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_ocr_suggestions(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_ignore_nonwords(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_extra_hyphens(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_missing_hyphens(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_titles_in_gc(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_unfinished_paragraphs_in_gc(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_hyphenate_unknown_words(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_accept_bulleted_lists_in_gc(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_min_hyphenated_word_length(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_max_suggestions(IntPtr handle, int value);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_set_speller_cache_size(IntPtr handle, int value);

    // -- Utility --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr voikko_version();

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern IntPtr voikko_attribute_values(byte[] name);

    // -- Memory management --

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free_str(IntPtr s);

    [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
    public static extern void voikko_free_str_array(IntPtr arr);
}

// ── Public API ───────────────────────────────────────────────────

/// <summary>
/// Finnish NLP toolkit powered by the Rust voikko-ffi library.
///
/// Provides spell checking, suggestions, morphological analysis,
/// hyphenation, grammar checking, tokenization, and sentence detection.
///
/// <example>
/// <code>
/// using var v = new Voikko("/path/to/vvfst");
/// Console.WriteLine(v.Spell("koira"));        // True
/// Console.WriteLine(v.Hyphenate("koirarata")); // koi-ra-ra-ta
/// </code>
/// </example>
/// </summary>
public sealed class Voikko : IDisposable
{
    private IntPtr _handle;
    private readonly object _lock = new();
    private bool _disposed;

    /// <summary>
    /// Initialize Voikko from a dictionary directory.
    /// </summary>
    /// <param name="dictPath">
    /// Path to directory containing mor.vfst (and optionally autocorr.vfst).
    /// Supports both flat layout and V5 structure ({path}/5/mor-standard/).
    /// </param>
    /// <exception cref="FileNotFoundException">mor.vfst not found.</exception>
    /// <exception cref="VoikkoException">Native initialization failed.</exception>
    public Voikko(string dictPath)
    {
        var path = dictPath;

        // Auto-detect V5 structure
        var morPath = Path.Combine(path, "mor.vfst");
        if (!File.Exists(morPath))
        {
            var v5 = Path.Combine(path, "5", "mor-standard", "mor.vfst");
            if (File.Exists(v5))
            {
                morPath = v5;
                path = Path.GetDirectoryName(v5)!;
            }
            else
            {
                throw new FileNotFoundException($"mor.vfst not found in {dictPath}");
            }
        }

        byte[] morData = File.ReadAllBytes(morPath);

        var autocorrPath = Path.Combine(path, "autocorr.vfst");
        byte[]? autocorrData = File.Exists(autocorrPath) ? File.ReadAllBytes(autocorrPath) : null;

        _handle = Native.voikko_new(
            morData, (nuint)morData.Length,
            autocorrData, (nuint)(autocorrData?.Length ?? 0),
            out IntPtr errorPtr);

        if (_handle == IntPtr.Zero)
        {
            string msg = errorPtr != IntPtr.Zero
                ? PtrToStringUtf8(errorPtr)
                : "unknown error";
            if (errorPtr != IntPtr.Zero)
                Native.voikko_free_str(errorPtr);
            throw new VoikkoException($"Failed to initialize Voikko: {msg}");
        }
    }

    /// <summary>Dispose and release native resources.</summary>
    public void Dispose()
    {
        if (_disposed) return;
        _disposed = true;
        if (_handle != IntPtr.Zero)
        {
            Native.voikko_free(_handle);
            _handle = IntPtr.Zero;
        }
    }

    // ── Spell checking ───────────────────────────────────────────

    /// <summary>Check whether a word is correctly spelled.</summary>
    public bool Spell(string word)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            return Native.voikko_spell(_handle, Enc(word)) == 1;
        }
    }

    /// <summary>Get spelling suggestions for a misspelled word.</summary>
    public List<string> Suggest(string word)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            IntPtr arr = Native.voikko_suggest(_handle, Enc(word));
            if (arr == IntPtr.Zero)
                return new List<string>();
            try
            {
                return ReadNullTerminatedStringArray(arr);
            }
            finally
            {
                Native.voikko_free_str_array(arr);
            }
        }
    }

    // ── Morphological analysis ───────────────────────────────────

    /// <summary>Perform morphological analysis on a word.</summary>
    public List<Analysis> Analyze(string word)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            NativeAnalysisArray arr = Native.voikko_analyze(_handle, Enc(word));
            try
            {
                var result = new List<Analysis>((int)arr.Count);
                int structSize = Marshal.SizeOf<NativeAnalysis>();
                for (int i = 0; i < (int)arr.Count; i++)
                {
                    IntPtr aPtr = arr.Analyses + i * structSize;
                    var native = Marshal.PtrToStructure<NativeAnalysis>(aPtr);
                    var analysis = new Analysis();
                    ReadKeyValuePairs(native.Keys, native.Values, analysis);
                    result.Add(analysis);
                }
                return result;
            }
            finally
            {
                Native.voikko_free_analyses(arr);
            }
        }
    }

    // ── Hyphenation ──────────────────────────────────────────────

    /// <summary>
    /// Get the raw hyphenation pattern for a word.
    /// Characters: ' '=no break, '-'=hyphen before, '='=hyphen replaces char.
    /// </summary>
    public string GetHyphenationPattern(string word)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            IntPtr ptr = Native.voikko_hyphenate(_handle, Enc(word));
            if (ptr == IntPtr.Zero)
                return new string(' ', word.Length);
            try
            {
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                Native.voikko_free_str(ptr);
            }
        }
    }

    /// <summary>Hyphenate a word by inserting hyphens at valid break points.</summary>
    /// <param name="word">Word to hyphenate.</param>
    /// <param name="separator">Separator to insert (default: "-").</param>
    /// <param name="allowContextChanges">Allow letter changes at break points.</param>
    public string Hyphenate(string word, string separator = "-", bool allowContextChanges = true)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            IntPtr ptr = Native.voikko_insert_hyphens(
                _handle, Enc(word), Enc(separator), allowContextChanges ? 1 : 0);
            if (ptr == IntPtr.Zero)
                return word;
            try
            {
                return PtrToStringUtf8(ptr);
            }
            finally
            {
                Native.voikko_free_str(ptr);
            }
        }
    }

    // ── Grammar checking ─────────────────────────────────────────

    /// <summary>Check text for grammar errors.</summary>
    /// <param name="text">Text to check (may contain multiple paragraphs).</param>
    /// <param name="language">Language code for error descriptions (default: "fi").</param>
    public List<GrammarError> GrammarErrors(string text, string language = "fi")
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            NativeGrammarErrorArray arr = Native.voikko_grammar_errors(
                _handle, Enc(text), Enc(language));
            try
            {
                var result = new List<GrammarError>((int)arr.Count);
                int structSize = Marshal.SizeOf<NativeGrammarError>();
                for (int i = 0; i < (int)arr.Count; i++)
                {
                    IntPtr ePtr = arr.Errors + i * structSize;
                    var native = Marshal.PtrToStructure<NativeGrammarError>(ePtr);

                    string desc = native.ShortDescription != IntPtr.Zero
                        ? PtrToStringUtf8(native.ShortDescription)
                        : "";

                    var suggestions = native.Suggestions != IntPtr.Zero
                        ? ReadNullTerminatedStringArray(native.Suggestions)
                        : new List<string>();

                    result.Add(new GrammarError(
                        native.ErrorCode,
                        (int)native.StartPos,
                        (int)native.ErrorLen,
                        desc,
                        suggestions));
                }
                return result;
            }
            finally
            {
                Native.voikko_free_grammar_errors(arr);
            }
        }
    }

    // ── Tokenization ─────────────────────────────────────────────

    /// <summary>Tokenize text into words, punctuation, and whitespace.</summary>
    public List<Token> Tokens(string text)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            NativeTokenArray arr = Native.voikko_tokens(_handle, Enc(text));
            try
            {
                var result = new List<Token>((int)arr.Count);
                int structSize = Marshal.SizeOf<NativeToken>();
                for (int i = 0; i < (int)arr.Count; i++)
                {
                    IntPtr tPtr = arr.Tokens + i * structSize;
                    var native = Marshal.PtrToStructure<NativeToken>(tPtr);

                    string tokenText = native.Text != IntPtr.Zero
                        ? PtrToStringUtf8(native.Text)
                        : "";

                    result.Add(new Token(
                        (TokenType)native.TokenType,
                        tokenText,
                        (int)native.Position));
                }
                return result;
            }
            finally
            {
                Native.voikko_free_tokens(arr);
            }
        }
    }

    // ── Sentence detection ───────────────────────────────────────

    /// <summary>Detect sentence boundaries in text.</summary>
    public List<Sentence> Sentences(string text)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            NativeSentenceArray arr = Native.voikko_sentences(_handle, Enc(text));
            try
            {
                var result = new List<Sentence>((int)arr.Count);
                int structSize = Marshal.SizeOf<NativeSentence>();
                for (int i = 0; i < (int)arr.Count; i++)
                {
                    IntPtr sPtr = arr.Sentences + i * structSize;
                    var native = Marshal.PtrToStructure<NativeSentence>(sPtr);
                    result.Add(new Sentence(
                        (SentenceType)native.SentenceType,
                        (int)native.SentenceLen));
                }
                return result;
            }
            finally
            {
                Native.voikko_free_sentences(arr);
            }
        }
    }

    // ── Option setters (14 boolean) ──────────────────────────────

    /// <summary>Ignore trailing dot when spell-checking (e.g., "koira." treated as "koira").</summary>
    public bool IgnoreDot { set { SetBool(Native.voikko_set_ignore_dot, value); } }

    /// <summary>Ignore words containing numbers.</summary>
    public bool IgnoreNumbers { set { SetBool(Native.voikko_set_ignore_numbers, value); } }

    /// <summary>Ignore all-uppercase words.</summary>
    public bool IgnoreUppercase { set { SetBool(Native.voikko_set_ignore_uppercase, value); } }

    /// <summary>Avoid ugly hyphenation points.</summary>
    public bool NoUglyHyphenation { set { SetBool(Native.voikko_set_no_ugly_hyphenation, value); } }

    /// <summary>Accept words where only the first letter is uppercase.</summary>
    public bool AcceptFirstUppercase { set { SetBool(Native.voikko_set_accept_first_uppercase, value); } }

    /// <summary>Accept words in all uppercase.</summary>
    public bool AcceptAllUppercase { set { SetBool(Native.voikko_set_accept_all_uppercase, value); } }

    /// <summary>Use OCR-optimized suggestion strategy.</summary>
    public bool OcrSuggestions { set { SetBool(Native.voikko_set_ocr_suggestions, value); } }

    /// <summary>Ignore non-word tokens during spell checking.</summary>
    public bool IgnoreNonwords { set { SetBool(Native.voikko_set_ignore_nonwords, value); } }

    /// <summary>Accept extra hyphens in compound words.</summary>
    public bool AcceptExtraHyphens { set { SetBool(Native.voikko_set_accept_extra_hyphens, value); } }

    /// <summary>Accept missing hyphens in compound words.</summary>
    public bool AcceptMissingHyphens { set { SetBool(Native.voikko_set_accept_missing_hyphens, value); } }

    /// <summary>Accept titles in grammar checking.</summary>
    public bool AcceptTitlesInGc { set { SetBool(Native.voikko_set_accept_titles_in_gc, value); } }

    /// <summary>Accept unfinished paragraphs in grammar checking.</summary>
    public bool AcceptUnfinishedParagraphsInGc { set { SetBool(Native.voikko_set_accept_unfinished_paragraphs_in_gc, value); } }

    /// <summary>Attempt to hyphenate unknown words.</summary>
    public bool HyphenateUnknownWords { set { SetBool(Native.voikko_set_hyphenate_unknown_words, value); } }

    /// <summary>Accept bulleted lists in grammar checking.</summary>
    public bool AcceptBulletedListsInGc { set { SetBool(Native.voikko_set_accept_bulleted_lists_in_gc, value); } }

    // ── Option setters (3 integer) ───────────────────────────────

    /// <summary>Minimum word length for hyphenation (default: 2).</summary>
    public int MinHyphenatedWordLength
    {
        set
        {
            lock (_lock) { ThrowIfDisposed(); Native.voikko_set_min_hyphenated_word_length(_handle, value); }
        }
    }

    /// <summary>Maximum number of suggestions to return.</summary>
    public int MaxSuggestions
    {
        set
        {
            lock (_lock) { ThrowIfDisposed(); Native.voikko_set_max_suggestions(_handle, value); }
        }
    }

    /// <summary>Speller cache size (0 = no cache, larger = more memory).</summary>
    public int SpellerCacheSize
    {
        set
        {
            lock (_lock) { ThrowIfDisposed(); Native.voikko_set_speller_cache_size(_handle, value); }
        }
    }

    // ── Static utility ───────────────────────────────────────────

    /// <summary>Get the library version string.</summary>
    public static string Version()
    {
        IntPtr ptr = Native.voikko_version();
        return ptr != IntPtr.Zero ? PtrToStringUtf8(ptr) : "";
        // Do NOT free — voikko_version returns a static pointer.
    }

    /// <summary>Get valid values for a morphological attribute name.</summary>
    /// <returns>List of valid values, or null if the attribute is not recognized.</returns>
    public static List<string>? AttributeValues(string name)
    {
        IntPtr ptr = Native.voikko_attribute_values(Enc(name));
        if (ptr == IntPtr.Zero)
            return null;
        return ReadNullTerminatedStringArray(ptr);
        // Do NOT free — voikko_attribute_values returns static pointers.
    }

    // ── Internal helpers ─────────────────────────────────────────

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    private void SetBool(Action<IntPtr, int> setter, bool value)
    {
        lock (_lock)
        {
            ThrowIfDisposed();
            setter(_handle, value ? 1 : 0);
        }
    }

    /// <summary>Encode a C# string to a null-terminated UTF-8 byte array.</summary>
    private static byte[] Enc(string s)
    {
        int byteCount = Encoding.UTF8.GetByteCount(s);
        byte[] buf = new byte[byteCount + 1]; // +1 for null terminator
        Encoding.UTF8.GetBytes(s, 0, s.Length, buf, 0);
        return buf;
    }

    /// <summary>Read a UTF-8 C string from an IntPtr.</summary>
    private static string PtrToStringUtf8(IntPtr ptr)
    {
        return Marshal.PtrToStringUTF8(ptr) ?? "";
    }

    /// <summary>Read a NULL-terminated char** array into a List of strings.</summary>
    private static List<string> ReadNullTerminatedStringArray(IntPtr arr)
    {
        var result = new List<string>();
        int offset = 0;
        while (true)
        {
            IntPtr strPtr = Marshal.ReadIntPtr(arr, offset);
            if (strPtr == IntPtr.Zero)
                break;
            result.Add(PtrToStringUtf8(strPtr));
            offset += IntPtr.Size;
        }
        return result;
    }

    /// <summary>
    /// Read parallel NULL-terminated key/value arrays into a dictionary.
    /// </summary>
    private static void ReadKeyValuePairs(IntPtr keys, IntPtr values, Dictionary<string, string> dict)
    {
        int offset = 0;
        while (true)
        {
            IntPtr keyPtr = Marshal.ReadIntPtr(keys, offset);
            if (keyPtr == IntPtr.Zero)
                break;
            IntPtr valPtr = Marshal.ReadIntPtr(values, offset);
            string key = PtrToStringUtf8(keyPtr);
            string val = valPtr != IntPtr.Zero ? PtrToStringUtf8(valPtr) : "";
            dict[key] = val;
            offset += IntPtr.Size;
        }
    }
}

/// <summary>Exception thrown when Voikko initialization or operations fail.</summary>
public class VoikkoException : Exception
{
    public VoikkoException(string message) : base(message) { }
}
