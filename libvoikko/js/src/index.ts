import type {
  Token,
  Sentence,
  GrammarError,
  Analysis,
  SuggestionStrategy,
  VoikkoInitOptions,
  RawVoikkoInstance,
} from './types.js';
import { loadWasm, loadDict, mountDict } from './wasm-loader.js';

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

/**
 * Finnish language NLP toolkit powered by libvoikko WASM.
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
  private raw: RawVoikkoInstance;

  private constructor(raw: RawVoikkoInstance) {
    this.raw = raw;
  }

  /**
   * Initialize a Voikko instance for the given language.
   *
   * Pipeline: loadWasm ∥ loadDict → mountDict → module.init
   * WASM module is cached after first call.
   *
   * @param language - BCP 47 language tag (default: 'fi')
   * @param options - Dictionary loading options
   */
  static async init(
    language: string = 'fi',
    options: VoikkoInitOptions = {},
  ): Promise<Voikko> {
    const [module, dict] = await Promise.all([
      loadWasm(options.locateFile),
      loadDict(options),
    ]);
    mountDict(module, dict);
    const raw = module.init(language);
    return new Voikko(raw);
  }

  /** Release all resources. The instance must not be used after this call. */
  terminate(): void {
    this.raw.terminate();
  }

  /** Check spelling. Returns true if the word is correct. */
  spell(word: string): boolean {
    return this.raw.spell(word);
  }

  /** Get spelling suggestions for a (misspelled) word. */
  suggest(word: string): string[] {
    return this.raw.suggest(word);
  }

  /**
   * Check text for grammar errors.
   * Accepts multiple paragraphs separated by newline characters.
   */
  grammarErrors(text: string, language: string = 'fi'): GrammarError[] {
    return this.raw.grammarErrors(text, language);
  }

  /** Morphological analysis of a word. */
  analyze(word: string): Analysis[] {
    return this.raw.analyze(word);
  }

  /** Split text into tokens. */
  tokens(text: string): Token[] {
    return this.raw.tokens(text);
  }

  /** Split text into sentences. */
  sentences(text: string): Sentence[] {
    return this.raw.sentences(text);
  }

  /**
   * Get the hyphenation pattern for a word.
   * ' ' = no hyphenation, '-' = hyphenation point (preserved),
   * '=' = hyphenation point (replaced by hyphen).
   */
  getHyphenationPattern(word: string): string {
    return this.raw.getHyphenationPattern(word);
  }

  /** Hyphenate a word with the given separator. */
  hyphenate(word: string, separator?: string, allowContextChanges?: boolean): string {
    return this.raw.hyphenate(word, separator, allowContextChanges);
  }

  /** Get possible values for an enumerated morphological attribute. */
  attributeValues(attributeName: string): string[] | null {
    return this.raw.attributeValues(attributeName);
  }

  // -- Option setters --

  /** Ignore dot at the end of a word. Default: false */
  setIgnoreDot(value: boolean): void { this.raw.setIgnoreDot(value); }

  /** Ignore words containing numbers. Default: false */
  setIgnoreNumbers(value: boolean): void { this.raw.setIgnoreNumbers(value); }

  /** Accept words written entirely in uppercase without checking. Default: false */
  setIgnoreUppercase(value: boolean): void { this.raw.setIgnoreUppercase(value); }

  /** Accept words when the first letter is uppercase. Default: true */
  setAcceptFirstUppercase(value: boolean): void { this.raw.setAcceptFirstUppercase(value); }

  /** Accept words when all letters are uppercase (still checked). Default: true */
  setAcceptAllUppercase(value: boolean): void { this.raw.setAcceptAllUppercase(value); }

  /** Ignore non-words such as URLs and email addresses. Default: true */
  setIgnoreNonwords(value: boolean): void { this.raw.setIgnoreNonwords(value); }

  /** Allow some extra hyphens in words. Default: false */
  setAcceptExtraHyphens(value: boolean): void { this.raw.setAcceptExtraHyphens(value); }

  /** Accept missing hyphens at word boundaries. Default: false */
  setAcceptMissingHyphens(value: boolean): void { this.raw.setAcceptMissingHyphens(value); }

  /** Accept incomplete sentences in titles/headings. Default: false */
  setAcceptTitlesInGc(value: boolean): void { this.raw.setAcceptTitlesInGc(value); }

  /** Accept incomplete sentences at paragraph end. Default: false */
  setAcceptUnfinishedParagraphsInGc(value: boolean): void { this.raw.setAcceptUnfinishedParagraphsInGc(value); }

  /** Accept paragraphs valid as bulleted list items. Default: false */
  setAcceptBulletedListsInGc(value: boolean): void { this.raw.setAcceptBulletedListsInGc(value); }

  /** Skip ugly but correct hyphenation positions. Default: false */
  setNoUglyHyphenation(value: boolean): void { this.raw.setNoUglyHyphenation(value); }

  /** Hyphenate unknown words. Default: true */
  setHyphenateUnknownWords(value: boolean): void { this.raw.setHyphenateUnknownWords(value); }

  /** Minimum length for words that may be hyphenated. Default: 2 */
  setMinHyphenatedWordLength(value: number): void { this.raw.setMinHyphenatedWordLength(value); }

  /** Set the suggestion strategy. Default: TYPO */
  setSuggestionStrategy(strategy: SuggestionStrategy): void { this.raw.setSuggestionStrategy(strategy); }
}
