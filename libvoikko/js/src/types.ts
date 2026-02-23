/** Token types returned by the tokenizer */
export type TokenType = 'NONE' | 'WORD' | 'PUNCTUATION' | 'WHITESPACE' | 'UNKNOWN';

/** Sentence boundary types */
export type SentenceStartType = 'NONE' | 'NO_START' | 'PROBABLE' | 'POSSIBLE';

/** Spelling suggestion strategies */
export type SuggestionStrategy = 'TYPO' | 'OCR';

export interface Token {
  type: TokenType;
  text: string;
}

export interface Sentence {
  text: string;
  nextStartType: SentenceStartType;
}

export interface GrammarError {
  /** Error code describing the type of error. */
  errorCode: number;
  /** Start of the error segment within the paragraph */
  startPos: number;
  /** Length of the error segment */
  errorLen: number;
  /** List of suggested replacements for the marked error */
  suggestions: string[];
  /** Human readable short description for the error. */
  shortDescription: string;
}

export interface Analysis {
  /**
   * Base form of the given word.
   * Example: kissalla -> kissa
   */
  BASEFORM?: string;
  /**
   * Word class (Finnish: sanaluokka).
   * Values: nimisana, laatusana, teonsana, seikkasana, asemosana,
   * suhdesana, huudahdussana, sidesana, etunimi, sukunimi, paikannimi,
   * nimi, kieltosana, lyhenne, lukusana, etuliite
   */
  CLASS?: string;
  /**
   * Comparison form of an adjective.
   * Values: positive, comparative, superlative
   */
  COMPARISON?: string;
  /** Raw FST transducer output */
  FSTOUTPUT?: string;
  /** Focus particle (-kin or -kAAn) */
  FOCUS?: string;
  /** Question clitic (-ko or -kö). Value: "true" if present */
  KYSYMYSLIITE?: string;
  /**
   * Mood of a verb.
   * Values: indicative, conditional, imperative, potential
   */
  MOOD?: string;
  /**
   * Whether the verb is in connegative form.
   * Values: false, true, both
   */
  NEGATIVE?: string;
  /**
   * Grammatical number.
   * Values: singular, dual, trial, plural
   */
  NUMBER?: string;
  /**
   * Participle type.
   * Values: present_active, present_passive, past_active,
   * past_passive, agent, negation
   */
  PARTICIPLE?: string;
  /**
   * Person of a verb.
   * Values: 1, 2, 3, 4 (4 = passive)
   */
  PERSON?: string;
  /**
   * Possessive suffix.
   * Values: 1s, 2s, 1p, 2p, 3
   */
  POSSESSIVE?: string;
  /**
   * Noun case (Finnish: sijamuoto).
   * Values: nimento, omanto, osanto, olento, tulento, kohdanto,
   * sisaolento, sisaeronto, sisatulento, ulkoolento, ulkoeronto,
   * ulkotulento, vajanto, seuranto, keinonto, kerrontosti
   */
  SIJAMUOTO?: string;
  /**
   * Morpheme boundaries and character case pattern.
   * Characters: = (morpheme start), - (hyphen), p/q (lowercase),
   * i/j (uppercase). q/j forbid hyphenation before them.
   * Example: autokauppa -> =pppp=pppppp
   */
  STRUCTURE?: string;
  /**
   * Tense of a verb.
   * Values: present_simple, past_imperfective
   */
  TENSE?: string;
  /** Word bases with morpheme boundaries */
  WORDBASES?: string;
  /** Word IDs referencing Joukahainen dictionary entries */
  WORDIDS?: string;
  /** Additional attributes not explicitly typed */
  [key: string]: string | undefined;
}

/** Options for initializing a Voikko instance */
export interface VoikkoInitOptions {
  /**
   * URL base path for fetching dictionary files (browser).
   * The path should contain the V5 dictionary structure:
   * `{url}/5/mor-standard/index.txt`, `mor.vfst`, `autocorr.vfst`
   *
   * If omitted in browser, defaults to the unpkg CDN.
   */
  dictionaryUrl?: string;
  /**
   * Local filesystem path for dictionary files (Node.js).
   * Accepts both flat layout (files directly in path) and
   * V5 structure ({path}/5/mor-standard/).
   *
   * If omitted in Node.js, uses the bundled dictionary.
   */
  dictionaryPath?: string;
  /**
   * URL to the WASM binary (browser only).
   * If omitted, defaults to the unpkg CDN.
   */
  wasmUrl?: string;
}
