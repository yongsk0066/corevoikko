import { describe, it, expect, beforeAll, beforeEach, afterAll, afterEach } from 'vitest';
import type { Voikko as VoikkoType } from '../src/index.js';

// -- Tier 1: Type/Structure tests (always runnable) --

describe('Voikko module exports', () => {
  it('should export Voikko class with init method', async () => {
    const { Voikko } = await import('../src/index.js');
    expect(Voikko).toBeDefined();
    expect(typeof Voikko.init).toBe('function');
  });

  it('should export type interfaces', async () => {
    const mod = await import('../src/index.js');
    expect(mod.Voikko).toBeDefined();
  });
});

// -- Tier 2: Integration tests (require WASM + dictionary) --

const DICT_PATH = process.env['VOIKKO_DICT_PATH'];
const HAS_DICTIONARY = !!DICT_PATH;

describe.skipIf(!HAS_DICTIONARY)('Voikko integration', () => {
  let voikko: VoikkoType;

  beforeAll(async () => {
    const { Voikko } = await import('../src/index.js');
    voikko = await Voikko.init('fi', { dictionaryPath: DICT_PATH! });
  });

  afterAll(() => {
    voikko.terminate();
  });

  // Spell checking

  it('spell', () => {
    expect(voikko.spell('määrä')).toBe(true);
    expect(voikko.spell('määä')).toBe(false);
  });

  it('spellAfterTerminateThrowsException', async () => {
    const { Voikko } = await import('../src/index.js');
    const tmp = await Voikko.init('fi', { dictionaryPath: DICT_PATH! });
    tmp.terminate();
    expect(() => tmp.spell('kissa')).toThrow();
  });

  // Suggestions

  it('suggest', () => {
    expect(voikko.suggest('koirra')).toContain('koira');
    expect(voikko.suggest('määärä')).toContain('määrä');
    expect(voikko.suggest('lasjkblvankirknaslvethikertvhgn')).toHaveLength(0);
  });

  it('suggestReturnsArgumentIfWordIsCorrect', () => {
    expect(voikko.suggest('koira')).toEqual(['koira']);
  });

  // Grammar

  it('grammarErrorsAndExplanation', () => {
    expect(voikko.grammarErrors('Minä olen joten kuten kaunis.', 'fi')).toEqual([{
      startPos: 10,
      errorLen: 11,
      suggestions: ['jotenkuten'],
      errorCode: 1,
      shortDescription: 'Virheellinen kirjoitusasu',
    }]);
  });

  it('noGrammarErrorsInEmptyParagraph', () => {
    expect(voikko.grammarErrors('Olen täi.\n\nOlen täi.', 'fi')).toEqual([]);
  });

  it('grammarErrorOffsetsInMultipleParagraphs', () => {
    const errors = voikko.grammarErrors('Olen täi.\n\nOlen joten kuten.', 'fi');
    expect(errors).toHaveLength(1);
    expect(errors[0].startPos).toBe(16);
    expect(errors[0].errorLen).toBe(11);
  });

  it('grammarErrorWithWindowsParagraphSeparator', () => {
    const errors = voikko.grammarErrors('Olen täi.\r\nOlen joten kuten.', 'fi');
    expect(errors).toHaveLength(1);
    expect(errors[0].startPos).toBe(16);
    expect(errors[0].errorLen).toBe(11);
  });

  it('grammarErrorWithMultipleWindowsParagraphSeparator', () => {
    const errors = voikko.grammarErrors('Olen täi.\r\nOlen täi.\r\nOlen joten kuten.', 'fi');
    expect(errors).toHaveLength(1);
    expect(errors[0].startPos).toBe(27);
    expect(errors[0].errorLen).toBe(11);
  });

  // Morphological analysis

  it('analyze', () => {
    const analysisList = voikko.analyze('kansaneläkehakemus');
    expect(analysisList).toHaveLength(1);
    expect(analysisList[0].STRUCTURE).toBe('=pppppp=ppppp=ppppppp');
  });

  // Tokenization

  it('tokens', () => {
    expect(voikko.tokens('kissa ja koira sekä härkä')).toEqual([
      { type: 'WORD', text: 'kissa' },
      { type: 'WHITESPACE', text: ' ' },
      { type: 'WORD', text: 'ja' },
      { type: 'WHITESPACE', text: ' ' },
      { type: 'WORD', text: 'koira' },
      { type: 'WHITESPACE', text: ' ' },
      { type: 'WORD', text: 'sekä' },
      { type: 'WHITESPACE', text: ' ' },
      { type: 'WORD', text: 'härkä' },
    ]);
  });

  it('nullCharIsUnknownToken', () => {
    expect(voikko.tokens('kissa\0koira')).toEqual([
      { type: 'WORD', text: 'kissa' },
      { type: 'UNKNOWN', text: '\0' },
      { type: 'WORD', text: 'koira' },
    ]);
    expect(voikko.tokens('kissa\0\0koira')).toEqual([
      { type: 'WORD', text: 'kissa' },
      { type: 'UNKNOWN', text: '\0' },
      { type: 'UNKNOWN', text: '\0' },
      { type: 'WORD', text: 'koira' },
    ]);
    expect(voikko.tokens('kissa\0')).toEqual([
      { type: 'WORD', text: 'kissa' },
      { type: 'UNKNOWN', text: '\0' },
    ]);
    expect(voikko.tokens('\0kissa')).toEqual([
      { type: 'UNKNOWN', text: '\0' },
      { type: 'WORD', text: 'kissa' },
    ]);
    expect(voikko.tokens('\0')).toEqual([
      { type: 'UNKNOWN', text: '\0' },
    ]);
    expect(voikko.tokens('')).toHaveLength(0);
  });

  // Sentences

  it('sentences', () => {
    expect(voikko.sentences('Kissa ei ole koira. Koira ei ole kissa.')).toEqual([
      { nextStartType: 'PROBABLE', text: 'Kissa ei ole koira. ' },
      { nextStartType: 'NONE', text: 'Koira ei ole kissa.' },
    ]);
  });

  // Hyphenation

  it('hyphenationPattern', () => {
    expect(voikko.getHyphenationPattern('kissa')).toBe('   - ');
    expect(voikko.getHyphenationPattern('määrä')).toBe('   - ');
    expect(voikko.getHyphenationPattern('kuorma-auto')).toBe('    - =  - ');
    expect(voikko.getHyphenationPattern("vaa'an")).toBe('   =  ');
  });

  it('hyphenate', () => {
    expect(voikko.hyphenate('kissa')).toBe('kis-sa');
    expect(voikko.hyphenate('määrä')).toBe('mää-rä');
    expect(voikko.hyphenate('kuorma-auto')).toBe('kuor-ma-au-to');
    expect(voikko.hyphenate("vaa'an")).toBe('vaa-an');
  });

  it('hyphenateWithCustomSeparator', () => {
    expect(voikko.hyphenate('kissa', '&shy;', true)).toBe('kis&shy;sa');
    expect(voikko.hyphenate('kuorma-auto', '&shy;', true)).toBe('kuor&shy;ma-au&shy;to');
    expect(voikko.hyphenate("vaa'an", '&shy;', true)).toBe('vaa&shy;an');
    expect(voikko.hyphenate("vaa'an", '&shy;', false)).toBe("vaa'an");
  });

  it('attributeValuesForEnumeratedAttribute', () => {
    const values = voikko.attributeValues('NUMBER');
    expect(values).toHaveLength(2);
    expect(values).toContain('singular');
    expect(values).toContain('plural');
  });

  it('attributeValuesForNonEnumeratedAttribute', () => {
    expect(voikko.attributeValues('BASEFORM')).toBeNull();
  });

  it('attributeValuesForUnknownAttribute', () => {
    expect(voikko.attributeValues('XYZ')).toBeNull();
  });
});

// -- Tier 2: Option setter tests (need fresh instance per test) --

describe.skipIf(!HAS_DICTIONARY)('Voikko option setters', () => {
  let voikko: VoikkoType;

  beforeEach(async () => {
    const { Voikko } = await import('../src/index.js');
    voikko = await Voikko.init('fi', { dictionaryPath: DICT_PATH! });
  });

  afterEach(() => {
    voikko.terminate();
  });

  it('setIgnoreDot', () => {
    voikko.setIgnoreDot(false);
    expect(voikko.spell('kissa.')).toBe(false);
    voikko.setIgnoreDot(true);
    expect(voikko.spell('kissa.')).toBe(true);
  });

  it('setIgnoreNumbers', () => {
    voikko.setIgnoreNumbers(false);
    expect(voikko.spell('kissa2')).toBe(false);
    voikko.setIgnoreNumbers(true);
    expect(voikko.spell('kissa2')).toBe(true);
  });

  it('setIgnoreUppercase', () => {
    voikko.setIgnoreUppercase(false);
    expect(voikko.spell('KAAAA')).toBe(false);
    voikko.setIgnoreUppercase(true);
    expect(voikko.spell('KAAAA')).toBe(true);
  });

  it('setAcceptFirstUppercase', () => {
    voikko.setAcceptFirstUppercase(false);
    expect(voikko.spell('Kissa')).toBe(false);
    voikko.setAcceptFirstUppercase(true);
    expect(voikko.spell('Kissa')).toBe(true);
  });

  it('upperCaseScandinavianLetters', () => {
    expect(voikko.spell('Äiti')).toBe(true);
    expect(voikko.spell('Ääiti')).toBe(false);
    expect(voikko.spell('š')).toBe(true);
    expect(voikko.spell('Š')).toBe(true);
  });

  it('acceptAllUppercase', () => {
    voikko.setIgnoreUppercase(false);
    voikko.setAcceptAllUppercase(false);
    expect(voikko.spell('KISSA')).toBe(false);
    voikko.setAcceptAllUppercase(true);
    expect(voikko.spell('KISSA')).toBe(true);
    expect(voikko.spell('KAAAA')).toBe(false);
  });

  it('ignoreNonwords', () => {
    voikko.setIgnoreNonwords(false);
    expect(voikko.spell('hatapitk@iki.fi')).toBe(false);
    voikko.setIgnoreNonwords(true);
    expect(voikko.spell('hatapitk@iki.fi')).toBe(true);
    expect(voikko.spell('ashdaksd')).toBe(false);
  });

  it('acceptExtraHyphens', () => {
    voikko.setAcceptExtraHyphens(false);
    expect(voikko.spell('kerros-talo')).toBe(false);
    voikko.setAcceptExtraHyphens(true);
    expect(voikko.spell('kerros-talo')).toBe(true);
  });

  it('acceptMissingHyphens', () => {
    voikko.setAcceptMissingHyphens(false);
    expect(voikko.spell('sosiaali')).toBe(false);
    voikko.setAcceptMissingHyphens(true);
    expect(voikko.spell('sosiaali')).toBe(true);
  });

  it('setAcceptTitlesInGc', () => {
    voikko.setAcceptTitlesInGc(false);
    expect(voikko.grammarErrors('Kissa on eläin', 'fi')).toHaveLength(1);
    voikko.setAcceptTitlesInGc(true);
    expect(voikko.grammarErrors('Kissa on eläin', 'fi')).toHaveLength(0);
  });

  it('setAcceptUnfinishedParagraphsInGc', () => {
    voikko.setAcceptUnfinishedParagraphsInGc(false);
    expect(voikko.grammarErrors('Kissa on ', 'fi')).toHaveLength(1);
    voikko.setAcceptUnfinishedParagraphsInGc(true);
    expect(voikko.grammarErrors('Kissa on ', 'fi')).toHaveLength(0);
  });

  it('setAcceptBulletedListsInGc', () => {
    voikko.setAcceptBulletedListsInGc(false);
    expect(voikko.grammarErrors('kissa', 'fi').length).toBeGreaterThan(0);
    voikko.setAcceptBulletedListsInGc(true);
    expect(voikko.grammarErrors('kissa', 'fi')).toHaveLength(0);
  });

  it('setNoUglyHyphenation', () => {
    voikko.setNoUglyHyphenation(false);
    expect(voikko.hyphenate('iva')).toBe('i-va');
    voikko.setNoUglyHyphenation(true);
    expect(voikko.hyphenate('iva')).toBe('iva');
  });

  it('setHyphenateUnknownWordsWorks', () => {
    voikko.setHyphenateUnknownWords(false);
    expect(voikko.hyphenate('kirjutepo')).toBe('kirjutepo');
    voikko.setHyphenateUnknownWords(true);
    expect(voikko.hyphenate('kirjutepo')).toBe('kir-ju-te-po');
  });

  it('setMinHyphenatedWordLength', () => {
    voikko.setMinHyphenatedWordLength(6);
    expect(voikko.hyphenate('koira')).toBe('koira');
    voikko.setMinHyphenatedWordLength(2);
    expect(voikko.hyphenate('koira')).toBe('koi-ra');
  });

  it('setSuggestionStrategy', () => {
    voikko.setSuggestionStrategy('OCR');
    expect(voikko.suggest('koari')).not.toContain('koira');
    expect(voikko.suggest('koir_')).toContain('koira');
    voikko.setSuggestionStrategy('TYPO');
    expect(voikko.suggest('koari')).toContain('koira');
  });
});
