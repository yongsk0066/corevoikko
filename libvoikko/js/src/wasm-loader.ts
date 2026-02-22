import type { VoikkoInitOptions } from './types.js';

/** Dictionary files required for initialization (missing = fatal error) */
const REQUIRED_DICT_FILES = ['index.txt', 'mor.vfst'] as const;

/** Dictionary files that are optional (missing = silently skipped) */
const OPTIONAL_DICT_FILES = ['autocorr.vfst'] as const;

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
 *
 * Resolution order (Node.js):
 *   1. options.dictionaryPath (explicit)
 *   2. Bundled dictionary shipped with the npm package (auto)
 *
 * Browser: options.dictionaryUrl is required.
 */
export async function loadDict(
  options: VoikkoInitOptions,
): Promise<Map<string, Uint8Array>> {
  if (options.dictionaryUrl) return fetchDict(options.dictionaryUrl);
  if (options.dictionaryPath) return readDict(options.dictionaryPath);

  // Node.js: try bundled dictionary
  if (typeof globalThis.window === 'undefined' && typeof globalThis.document === 'undefined') {
    const { fileURLToPath } = await import('node:url');
    const { dirname, join } = await import('node:path');
    const bundledDict = join(dirname(fileURLToPath(import.meta.url)), '..', 'dict');
    return readDict(bundledDict);
  }

  throw new Error(
    'Voikko: dictionaryUrl is required in browser environments',
  );
}

/** Fetch dictionary files from a URL (browser). */
async function fetchDict(baseUrl: string): Promise<Map<string, Uint8Array>> {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';
  const map = new Map<string, Uint8Array>();

  // Required files — throw on failure
  await Promise.all(
    REQUIRED_DICT_FILES.map(async (name) => {
      const response = await fetch(`${base}5/mor-standard/${name}`);
      if (!response.ok) {
        throw new Error(
          `Failed to fetch dictionary file ${name}: ${response.status} ${response.statusText}`,
        );
      }
      map.set(name, new Uint8Array(await response.arrayBuffer()));
    }),
  );

  // Optional files — skip on failure (e.g. 404)
  await Promise.all(
    OPTIONAL_DICT_FILES.map(async (name) => {
      try {
        const response = await fetch(`${base}5/mor-standard/${name}`);
        if (response.ok) {
          map.set(name, new Uint8Array(await response.arrayBuffer()));
        }
      } catch {
        // Network error — skip optional file silently
      }
    }),
  );

  return map;
}

/**
 * Read dictionary files from local filesystem (Node.js).
 * Supports both flat layout ({path}/index.txt) and V5 structure ({path}/5/mor-standard/).
 */
async function readDict(dirPath: string): Promise<Map<string, Uint8Array>> {
  const { readFile, access } = await import('node:fs/promises');
  const { join } = await import('node:path');

  const flatFile = join(dirPath, REQUIRED_DICT_FILES[0]);
  let dictDir: string;
  try {
    await access(flatFile);
    dictDir = dirPath;
  } catch {
    dictDir = join(dirPath, '5', 'mor-standard');
  }

  const map = new Map<string, Uint8Array>();

  // Required files — throw on failure
  await Promise.all(
    REQUIRED_DICT_FILES.map(async (name) => {
      const data = await readFile(join(dictDir, name));
      map.set(name, new Uint8Array(data));
    }),
  );

  // Optional files — skip if not found on disk
  await Promise.all(
    OPTIONAL_DICT_FILES.map(async (name) => {
      try {
        const data = await readFile(join(dictDir, name));
        map.set(name, new Uint8Array(data));
      } catch {
        // File not found — skip optional file silently
      }
    }),
  );

  return map;
}
