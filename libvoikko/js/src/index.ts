import type {
  Token,
  Sentence,
  GrammarError,
  Analysis,
  SuggestionStrategy,
  TokenType,
  SentenceStartType,
  VoikkoInitOptions,
} from './types.js';
import { loadWasm, loadDict } from './wasm-loader.js';
import type { WasmVoikko } from '../wasm/voikko_wasm.js';

export type {
  Token,
  Sentence,
  GrammarError,
  Analysis,
  TokenType,
  SentenceStartType,
  SuggestionStrategy,
  VoikkoInitOptions,
} from './types.js';

// ── Type mappings ────────────────────────────────────────────────

const TOKEN_TYPE_MAP: Record<string, TokenType> = {
  Word: 'WORD',
  Punctuation: 'PUNCTUATION',
  Whitespace: 'WHITESPACE',
  Unknown: 'UNKNOWN',
  None: 'NONE',
};

const SENTENCE_TYPE_MAP: Record<string, SentenceStartType> = {
  Probable: 'PROBABLE',
  Possible: 'POSSIBLE',
  None: 'NONE',
  NoStart: 'NO_START',
};

/** Grammar error short descriptions (Finnish, matching C++ voikko_error_message_cstr) */
const GRAMMAR_ERROR_DESCRIPTIONS: Record<number, string> = {
  1: 'Virheellinen kirjoitusasu',
  2: 'Poista ylimääräinen väli.',
  3: 'Ylimääräinen väli välimerkin edessä',
  4: 'Poista ylimääräinen pilkku.',
  5: 'Virheellinen virkkeen aloittava merkki',
  6: 'Harkitse sanan kirjoittamista pienellä alkukirjaimella.',
  7: 'Sana on kirjoitettava isolla alkukirjaimella.',
  8: 'Sana on kirjoitettu kahteen kertaan.',
  9: 'Välimerkki puuttuu virkkeen lopusta.',
  10: 'Virheelliset välimerkit lainauksen lopussa',
  11: 'Suomenkieliseen tekstiin sopimaton lainausmerkki',
  12: 'Väärin sijoitettu sulkumerkki',
  13: 'Kieltoverbi ja pääverbi eivät sovi yhteen.',
  14: 'Jälkimmäisen verbin tulisi olla a/ä-päätteisessä infinitiivissä.',
  15: 'Jälkimmäisen verbin tulisi olla maan/mään-päätteisessä infinitiivissä.',
  16: 'Sidesana (ja, tai, mutta, ...) ei voi olla virkkeen viimeinen sana.',
  17: 'Tarkista, puuttuuko virkkeestä pääverbi tai -verbejä.',
  18: 'Virkkeestä saattaa puuttua pilkku, tai siinä voi olla ylimääräinen verbi.',
};

/** Enumerated attribute values (static, matching C++ voikkoGetAttributeValues) */
const ENUMERATED_ATTRIBUTES: Record<string, string[]> = {
  CLASS: [
    'nimisana', 'laatusana', 'nimisana_laatusana', 'teonsana', 'seikkasana',
    'asemosana', 'suhdesana', 'huudahdussana', 'sidesana', 'etuliite',
    'lukusana', 'lyhenne', 'kieltosana', 'etunimi', 'sukunimi', 'paikannimi', 'nimi',
  ],
  NUMBER: ['singular', 'plural'],
  PERSON: ['1', '2', '3', '4'],
  MOOD: [
    'indicative', 'conditional', 'potential', 'imperative',
    'A-infinitive', 'E-infinitive', 'MA-infinitive', 'MINEN-infinitive', 'MAINEN-infinitive',
  ],
  TENSE: ['present_simple', 'past_imperfective'],
  COMPARISON: ['positive', 'comparative', 'superlative'],
  NEGATIVE: ['false', 'true', 'both'],
  PARTICIPLE: [
    'present_active', 'present_passive', 'past_active', 'past_passive', 'agent', 'negation',
  ],
  POSSESSIVE: ['1s', '2s', '1p', '2p', '3'],
  SIJAMUOTO: [
    'nimento', 'omanto', 'osanto', 'olento', 'tulento', 'kohdanto',
    'sisaolento', 'sisaeronto', 'sisatulento', 'ulkoolento', 'ulkoeronto',
    'ulkotulento', 'vajanto', 'seuranto', 'keinonto', 'kerrontosti',
  ],
  FOCUS: ['läs', 'kAAn', 'kin', 'hAn', 'pA', 's'],
  KYSYMYSLIITE: ['true'],
};

// ── Voikko class ─────────────────────────────────────────────────

/**
 * Finnish language NLP toolkit powered by Rust WASM.
 *
 * Provides spell checking, suggestions, hyphenation, grammar checking,
 * morphological analysis, tokenization, and sentence detection.
 *
 * @example
 * ```typescript
 * const voikko = await Voikko.init('fi', { dictionaryPath: './dict' });
 * voikko.spell('koira');    // true
 * voikko.suggest('koirra'); // ['koira', ...]
 * voikko.terminate();
 * ```
 */
export class Voikko {
  #handle: WasmVoikko;

  private constructor(handle: WasmVoikko) {
    this.#handle = handle;
  }

  /**
   * Initialize a Voikko instance for the given language.
   *
   * Pipeline: loadWasm ∥ loadDict → new WasmVoikko(morData, autocorrData)
   * WASM module is cached after first call.
   *
   * @param language - BCP 47 language tag (default: 'fi')
   * @param options - Dictionary loading options
   */
  static async init(
    language: string = 'fi',
    options: VoikkoInitOptions = {},
  ): Promise<Voikko> {
    const [{ WasmVoikko }, dict] = await Promise.all([
      loadWasm(),
      loadDict(options),
    ]);

    const morData = dict.get('mor.vfst');
    if (!morData) {
      throw new Error('Voikko: mor.vfst not found in dictionary files');
    }
    const autocorrData = dict.get('autocorr.vfst') ?? null;

    const handle = new WasmVoikko(morData, autocorrData);
    return new Voikko(handle);
  }

  /** Release all resources. The instance must not be used after this call. */
  terminate(): void {
    this.#handle.terminate();
  }

  /** Check spelling. Returns true if the word is correct. */
  spell(word: string): boolean {
    return this.#handle.spell(word);
  }

  /** Get spelling suggestions for a (misspelled) word. */
  suggest(word: string): string[] {
    return this.#handle.suggest(word);
  }

  /**
   * Check text for grammar errors.
   * Accepts multiple paragraphs separated by newline characters.
   *
   * Paragraphs are delimited by `\n` (or `\r\n`). Each paragraph is checked
   * independently and error positions are adjusted to the original text offsets.
   */
  grammarErrors(text: string, _language: string = 'fi'): GrammarError[] {
    const result: GrammarError[] = [];
    let pos = 0;

    while (pos <= text.length) {
      // Find the next \n boundary in the original text
      let nlPos = text.indexOf('\n', pos);
      if (nlPos === -1) nlPos = text.length;

      // Strip trailing \r for \r\n line endings
      let paraEnd = nlPos;
      if (paraEnd > pos && text[paraEnd - 1] === '\r') paraEnd--;

      const para = text.substring(pos, paraEnd);
      if (para.length > 0) {
        const raw: { errorCode: number; startPos: number; errorLen: number; suggestions: string[]; shortDescription?: string }[] =
          this.#handle.grammarErrors(para);
        for (const e of raw) {
          result.push({
            errorCode: e.errorCode,
            startPos: e.startPos + pos,
            errorLen: e.errorLen,
            suggestions: e.suggestions,
            shortDescription: e.shortDescription || GRAMMAR_ERROR_DESCRIPTIONS[e.errorCode] || '',
          });
        }
      }

      // Advance past the \n (or stop if we reached end of text)
      if (nlPos >= text.length) break;
      pos = nlPos + 1;
    }

    return result;
  }

  /** Morphological analysis of a word. */
  analyze(word: string): Analysis[] {
    return this.#handle.analyze(word) as Analysis[];
  }

  /** Split text into tokens. */
  tokens(text: string): Token[] {
    const raw: { tokenType: string; text: string }[] = this.#handle.tokens(text);
    return raw.map((t) => ({
      type: TOKEN_TYPE_MAP[t.tokenType] ?? 'UNKNOWN',
      text: t.text,
    }));
  }

  /** Split text into sentences. */
  sentences(text: string): Sentence[] {
    const raw: { sentenceType: string; sentenceLen: number }[] = this.#handle.sentences(text);
    const result: Sentence[] = [];
    let pos = 0;
    for (const s of raw) {
      const sentenceText = text.substring(pos, pos + s.sentenceLen);
      result.push({
        text: sentenceText,
        nextStartType: SENTENCE_TYPE_MAP[s.sentenceType] ?? 'NONE',
      });
      pos += s.sentenceLen;
    }
    return result;
  }

  /**
   * Get the hyphenation pattern for a word.
   * ' ' = no hyphenation, '-' = hyphenation point (preserved),
   * '=' = hyphenation point (replaced by hyphen).
   */
  getHyphenationPattern(word: string): string {
    return this.#handle.hyphenate(word);
  }

  /**
   * Hyphenate a word with the given separator.
   * @param separator - Character(s) to insert at hyphenation points (default: '-')
   * @param allowContextChanges - If true, handle context-sensitive hyphens (default: true)
   */
  hyphenate(word: string, separator: string = '-', allowContextChanges: boolean = true): string {
    const pattern = this.#handle.hyphenate(word);
    const chars = [...word];
    const patChars = [...pattern];
    let result = '';

    // Pattern meanings (one character per input character):
    // ' ' (space) = no hyphenation point
    // '-' = soft hyphenation point: insert separator BEFORE this character
    // '=' = compound boundary: REPLACE this character with separator
    //       Exception: if the character is already '-', keep it (no double hyphen)
    //       When allowContextChanges=false, '=' positions are skipped entirely

    for (let i = 0; i < chars.length; i++) {
      if (i < patChars.length) {
        if (patChars[i] === '-') {
          // Soft hyphen: insert separator before this character, then emit the char
          result += separator;
        } else if (patChars[i] === '=' && allowContextChanges) {
          if (chars[i] === '-') {
            // Existing hyphen at compound boundary — preserve it (no double hyphen)
            result += chars[i];
            continue;
          }
          // Replace the character (e.g., apostrophe in "vaa'an") with separator
          result += separator;
          continue;
        }
      }
      result += chars[i];
    }
    return result;
  }

  /** Get possible values for an enumerated morphological attribute. */
  attributeValues(attributeName: string): string[] | null {
    return ENUMERATED_ATTRIBUTES[attributeName] ?? null;
  }

  // -- Option setters --

  /** Ignore dot at the end of a word. Default: false */
  setIgnoreDot(value: boolean): void { this.#handle.setIgnoreDot(value); }

  /** Ignore words containing numbers. Default: false */
  setIgnoreNumbers(value: boolean): void { this.#handle.setIgnoreNumbers(value); }

  /** Accept words written entirely in uppercase without checking. Default: false */
  setIgnoreUppercase(value: boolean): void { this.#handle.setIgnoreUppercase(value); }

  /** Accept words when the first letter is uppercase. Default: true */
  setAcceptFirstUppercase(value: boolean): void { this.#handle.setAcceptFirstUppercase(value); }

  /** Accept words when all letters are uppercase (still checked). Default: true */
  setAcceptAllUppercase(value: boolean): void { this.#handle.setAcceptAllUppercase(value); }

  /** Ignore non-words such as URLs and email addresses. Default: true */
  setIgnoreNonwords(value: boolean): void { this.#handle.setIgnoreNonwords(value); }

  /** Allow some extra hyphens in words. Default: false */
  setAcceptExtraHyphens(value: boolean): void { this.#handle.setAcceptExtraHyphens(value); }

  /** Accept missing hyphens at word boundaries. Default: false */
  setAcceptMissingHyphens(value: boolean): void { this.#handle.setAcceptMissingHyphens(value); }

  /** Accept incomplete sentences in titles/headings. Default: false */
  setAcceptTitlesInGc(value: boolean): void { this.#handle.setAcceptTitlesInGc(value); }

  /** Accept incomplete sentences at paragraph end. Default: false */
  setAcceptUnfinishedParagraphsInGc(value: boolean): void { this.#handle.setAcceptUnfinishedParagraphsInGc(value); }

  /** Accept paragraphs valid as bulleted list items. Default: false */
  setAcceptBulletedListsInGc(value: boolean): void { this.#handle.setAcceptBulletedListsInGc(value); }

  /** Skip ugly but correct hyphenation positions. Default: false */
  setNoUglyHyphenation(value: boolean): void { this.#handle.setNoUglyHyphenation(value); }

  /** Hyphenate unknown words. Default: true */
  setHyphenateUnknownWords(value: boolean): void { this.#handle.setHyphenateUnknownWords(value); }

  /** Minimum length for words that may be hyphenated. Default: 2 */
  setMinHyphenatedWordLength(value: number): void { this.#handle.setMinHyphenatedWordLength(value); }

  /** Set the suggestion strategy. Default: TYPO */
  setSuggestionStrategy(strategy: SuggestionStrategy): void {
    this.#handle.setOcrSuggestions(strategy === 'OCR');
  }
}
