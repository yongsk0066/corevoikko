# @yongsk0066/voikko

Finnish NLP library for JavaScript — spell checking, morphological analysis, hyphenation, grammar checking, and tokenization. Powered by Rust compiled to WebAssembly.

## Install

```bash
npm install @yongsk0066/voikko
```

## Usage

### Node.js (zero-config)

Finnish dictionary is bundled in the package. No setup required.

```typescript
import { Voikko } from '@yongsk0066/voikko';

const voikko = await Voikko.init();

voikko.spell('koira');       // true
voikko.spell('koirra');      // false
voikko.suggest('koirra');    // ['koira', 'koiraa', ...]
voikko.analyze('koirien');   // [{ BASEFORM: 'koira', CLASS: 'nimisana', ... }]
voikko.hyphenate('kissa');   // 'kis-sa'

voikko.grammarErrors('Minä on hyvä.');
// [{ errorCode: 1, startPos: 0, errorLen: 6, suggestions: ['Minä olen'], ... }]

voikko.tokens('Hei maailma!');
// [{ type: 'WORD', text: 'Hei' }, { type: 'WHITESPACE', text: ' ' }, ...]

voikko.sentences('Hei. Miten menee?');
// [{ text: 'Hei. ', nextStartType: 'PROBABLE' }, { text: 'Miten menee?', ... }]

voikko.terminate(); // release resources
```

### Browser (zero-config)

WASM and dictionary files are automatically fetched from CDN. No setup needed.

```typescript
import { Voikko } from '@yongsk0066/voikko';

const voikko = await Voikko.init();
voikko.spell('koira'); // true
```

### Browser (self-hosted)

For production, serve files yourself to avoid CDN dependency:

```typescript
const voikko = await Voikko.init('fi', {
  dictionaryUrl: '/dict/',   // expects /dict/5/mor-standard/{files}
  wasmUrl: '/voikko.wasm',
});
```

Copy dictionary files from `node_modules/@yongsk0066/voikko/dict/` and the WASM binary from `node_modules/@yongsk0066/voikko/wasm/voikko_wasm_bg.wasm` to your public directory.

## API

### `Voikko.init(language?, options?)`

Creates a new Voikko instance. Returns `Promise<Voikko>`.

- `language` — BCP 47 tag (default: `'fi'`)
- `options.dictionaryUrl` — URL base for dictionary files (browser)
- `options.dictionaryPath` — Local filesystem path (Node.js)
- `options.wasmUrl` — URL to WASM binary (browser)

All options are optional. Defaults to bundled dictionary (Node.js) or CDN (browser).

### Spell checking

- `voikko.spell(word)` — `boolean`
- `voikko.suggest(word)` — `string[]`

### Morphological analysis

- `voikko.analyze(word)` — `Analysis[]`

Returns analysis objects with fields: `BASEFORM`, `CLASS`, `STRUCTURE`, `SIJAMUOTO`, `NUMBER`, `PERSON`, `MOOD`, `TENSE`, `PARTICIPLE`, `POSSESSIVE`, `COMPARISON`, `NEGATIVE`, `FOCUS`, `WORDBASES`, `WORDIDS`, `FSTOUTPUT`.

### Hyphenation

- `voikko.hyphenate(word, separator?, allowContextChanges?)` — `string`
- `voikko.getHyphenationPattern(word)` — `string`

### Grammar checking

- `voikko.grammarErrors(text)` — `GrammarError[]`

### Tokenization

- `voikko.tokens(text)` — `Token[]` with types: `WORD`, `PUNCTUATION`, `WHITESPACE`, `UNKNOWN`
- `voikko.sentences(text)` — `Sentence[]`

### Options

- `voikko.setIgnoreDot(boolean)`
- `voikko.setIgnoreNumbers(boolean)`
- `voikko.setIgnoreUppercase(boolean)`
- `voikko.setAcceptFirstUppercase(boolean)`
- `voikko.setAcceptAllUppercase(boolean)`
- `voikko.setIgnoreNonwords(boolean)`
- `voikko.setAcceptExtraHyphens(boolean)`
- `voikko.setAcceptMissingHyphens(boolean)`
- `voikko.setAcceptTitlesInGc(boolean)`
- `voikko.setAcceptUnfinishedParagraphsInGc(boolean)`
- `voikko.setAcceptBulletedListsInGc(boolean)`
- `voikko.setNoUglyHyphenation(boolean)`
- `voikko.setHyphenateUnknownWords(boolean)`
- `voikko.setMinHyphenatedWordLength(number)`
- `voikko.setSuggestionStrategy('TYPO' | 'OCR')`

### Cleanup

- `voikko.terminate()` — Release all resources. Instance must not be used after this call.

## Error Handling

The library provides typed errors for programmatic handling:

```typescript
import { Voikko, WasmLoadError, DictionaryLoadError } from '@yongsk0066/voikko';

try {
  const voikko = await Voikko.init();
} catch (e) {
  if (e instanceof WasmLoadError) {
    console.error('WASM failed to load:', e.message);
  } else if (e instanceof DictionaryLoadError) {
    console.error('Dictionary file missing:', e.fileName);
  }
}
```

Calling methods on a terminated instance throws an error:

```typescript
voikko.terminate();
voikko.spell('koira'); // Error: Cannot use Voikko instance after terminate()
```

## Bundle Size

| Component | Size | Notes |
|-----------|------|-------|
| JS wrapper | 14 KB | `dist/index.mjs` |
| WASM binary | 189 KB | Rust compiled, wasm-opt applied |
| Dictionary | 3.8 MB | Finnish morphology (`mor.vfst`) |

Node.js: All files are bundled in the package.
Browser: WASM and dictionary are fetched from CDN by default (~4 MB total on first load, cached by browser).

## Next.js / SSR

Voikko uses WebAssembly and must be initialized on the client side:

```typescript
'use client';
import { useState, useEffect } from 'react';
import type { Voikko as VoikkoType } from '@yongsk0066/voikko';

export function useVoikko() {
  const [voikko, setVoikko] = useState<VoikkoType | null>(null);

  useEffect(() => {
    let instance: VoikkoType | null = null;
    import('@yongsk0066/voikko').then(({ Voikko }) =>
      Voikko.init().then((v) => { instance = v; setVoikko(v); })
    );
    return () => { instance?.terminate(); };
  }, []);

  return voikko;
}
```

## Concurrency

A single Voikko instance is safe to use across multiple async operations. One instance per process is recommended for Node.js servers:

```typescript
const voikko = await Voikko.init();

app.get('/spell/:word', (req, res) => {
  res.json({ correct: voikko.spell(req.params.word) });
});

// Call terminate() only on shutdown
process.on('SIGTERM', () => voikko.terminate());
```

## License

[MPL 1.1 / GPL 2+ / LGPL 2.1+](https://github.com/yongsk0066/corevoikko/blob/master/LICENSE)
