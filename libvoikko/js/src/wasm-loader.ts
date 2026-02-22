import type { VoikkoInitOptions } from './types.js';

/** Dictionary files required for Finnish V5 format */
const DICT_FILES = ['index.txt', 'mor.vfst', 'autocorr.vfst'] as const;

// ── WASM initialization (cached) ─────────────────────────────────

type WasmModule = typeof import('../wasm/voikko_wasm.js');

let cachedInit: Promise<WasmModule> | null = null;

/**
 * Load and initialize the Rust WASM module. Cached after first call.
 * Returns the module with WasmVoikko class ready to use.
 *
 * In Node.js, the WASM binary is read from disk and passed directly
 * to the init function (fetch() cannot resolve local file paths).
 * In browsers, the init function fetches the WASM file automatically.
 */
export function loadWasm(): Promise<WasmModule> {
  return (cachedInit ??= (async () => {
    const wasm = await import('../wasm/voikko_wasm.js');

    if (typeof globalThis.window === 'undefined' && typeof globalThis.document === 'undefined') {
      // Node.js: read WASM binary from disk
      const { readFile } = await import('node:fs/promises');
      const { fileURLToPath } = await import('node:url');
      const { dirname, join } = await import('node:path');
      const thisDir = dirname(fileURLToPath(import.meta.url));
      const wasmPath = join(thisDir, '..', 'wasm', 'voikko_wasm_bg.wasm');
      const wasmBytes = await readFile(wasmPath);
      await wasm.default(wasmBytes);
    } else {
      // Browser: init() will auto-fetch the .wasm file
      await wasm.default();
    }

    return wasm;
  })());
}

// ── Dictionary loading ───────────────────────────────────────────

/**
 * Load dictionary files from the appropriate source.
 * Returns filename → bytes entries.
 */
export async function loadDict(
  options: VoikkoInitOptions,
): Promise<Map<string, Uint8Array>> {
  if (options.dictionaryUrl) return fetchDict(options.dictionaryUrl);
  if (options.dictionaryPath) return readDict(options.dictionaryPath);
  throw new Error(
    'Voikko: either dictionaryUrl (browser) or dictionaryPath (Node.js) must be provided',
  );
}

/** Fetch dictionary files from a URL (browser). */
async function fetchDict(baseUrl: string): Promise<Map<string, Uint8Array>> {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';

  const entries = await Promise.all(
    DICT_FILES.map(async (name) => {
      const response = await fetch(`${base}5/mor-standard/${name}`);
      if (!response.ok) {
        throw new Error(
          `Failed to fetch dictionary file ${name}: ${response.status} ${response.statusText}`,
        );
      }
      return [name, new Uint8Array(await response.arrayBuffer())] as const;
    }),
  );

  return new Map(entries);
}

/**
 * Read dictionary files from local filesystem (Node.js).
 * Supports both flat layout ({path}/index.txt) and V5 structure ({path}/5/mor-standard/).
 */
async function readDict(dirPath: string): Promise<Map<string, Uint8Array>> {
  const { readFile, access } = await import('node:fs/promises');
  const { join } = await import('node:path');

  const flatFile = join(dirPath, DICT_FILES[0]);
  let dictDir: string;
  try {
    await access(flatFile);
    dictDir = dirPath;
  } catch {
    dictDir = join(dirPath, '5', 'mor-standard');
  }

  const entries = await Promise.all(
    DICT_FILES.map(async (name) => {
      const data = await readFile(join(dictDir, name));
      return [name, new Uint8Array(data)] as const;
    }),
  );

  return new Map(entries);
}
