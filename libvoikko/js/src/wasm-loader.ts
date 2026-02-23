import type { VoikkoInitOptions } from './types.js';

/** Dictionary files required for initialization (missing = fatal error) */
const REQUIRED_DICT_FILES = ['index.txt', 'mor.vfst'] as const;

/** Dictionary files that are optional (missing = silently skipped) */
const OPTIONAL_DICT_FILES = ['autocorr.vfst'] as const;

declare const __PKG_VERSION__: string;
const CDN_BASE = `https://unpkg.com/@yongsk0066/voikko@${__PKG_VERSION__}`;

function isNode(): boolean {
  return typeof globalThis.window === 'undefined' && typeof globalThis.document === 'undefined';
}

// ── WASM initialization (cached) ─────────────────────────────────

type WasmModule = typeof import('../wasm/voikko_wasm.js');

let cachedInit: Promise<WasmModule> | null = null;

/**
 * Load and initialize the Rust WASM module. Cached after first call.
 *
 * - Node.js: reads WASM binary from disk (bundled in package)
 * - Browser: fetches from wasmUrl option, or unpkg CDN by default
 */
export function loadWasm(options: VoikkoInitOptions = {}): Promise<WasmModule> {
  return (cachedInit ??= (async () => {
    const wasm = await import('../wasm/voikko_wasm.js');

    if (isNode()) {
      const { readFile } = await import('node:fs/promises');
      const { fileURLToPath } = await import('node:url');
      const { dirname, join } = await import('node:path');
      const thisDir = dirname(fileURLToPath(import.meta.url));
      const wasmPath = join(thisDir, '..', 'wasm', 'voikko_wasm_bg.wasm');
      const wasmBytes = await readFile(wasmPath);
      await wasm.default(wasmBytes);
    } else {
      // Browser: use explicit URL or CDN fallback
      const wasmUrl = options.wasmUrl ?? `${CDN_BASE}/wasm/voikko_wasm_bg.wasm`;
      await wasm.default(wasmUrl);
    }

    return wasm;
  })());
}

// ── Dictionary loading ───────────────────────────────────────────

/**
 * Load dictionary files from the appropriate source.
 *
 * Resolution order:
 *   1. options.dictionaryUrl / options.dictionaryPath (explicit)
 *   2. Node.js → bundled dictionary shipped with the npm package
 *   3. Browser → unpkg CDN
 */
export async function loadDict(
  options: VoikkoInitOptions,
): Promise<Map<string, Uint8Array>> {
  if (options.dictionaryUrl) return fetchDict(options.dictionaryUrl);
  if (options.dictionaryPath) return readDict(options.dictionaryPath);

  if (isNode()) {
    // Node.js: bundled dictionary
    const { fileURLToPath } = await import('node:url');
    const { dirname, join } = await import('node:path');
    const bundledDict = join(dirname(fileURLToPath(import.meta.url)), '..', 'dict');
    return readDict(bundledDict);
  }

  // Browser: CDN fallback (flat layout — files directly under /dict/)
  return fetchDictFlat(`${CDN_BASE}/dict`);
}

/**
 * Fetch dictionary files from a V5 structured URL (browser).
 * Expects: {base}/5/mor-standard/{file}
 */
async function fetchDict(baseUrl: string): Promise<Map<string, Uint8Array>> {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';
  return fetchFiles((name) => `${base}5/mor-standard/${name}`);
}

/**
 * Fetch dictionary files from a flat URL (browser, CDN).
 * Expects: {base}/{file}
 */
async function fetchDictFlat(baseUrl: string): Promise<Map<string, Uint8Array>> {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';
  return fetchFiles((name) => `${base}${name}`);
}

/** Shared fetch logic for both V5 and flat URL layouts. */
async function fetchFiles(
  urlFor: (name: string) => string,
): Promise<Map<string, Uint8Array>> {
  const map = new Map<string, Uint8Array>();

  await Promise.all(
    REQUIRED_DICT_FILES.map(async (name) => {
      const response = await fetch(urlFor(name));
      if (!response.ok) {
        throw new Error(
          `Failed to fetch dictionary file ${name}: ${response.status} ${response.statusText}`,
        );
      }
      map.set(name, new Uint8Array(await response.arrayBuffer()));
    }),
  );

  await Promise.all(
    OPTIONAL_DICT_FILES.map(async (name) => {
      try {
        const response = await fetch(urlFor(name));
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

  await Promise.all(
    REQUIRED_DICT_FILES.map(async (name) => {
      const data = await readFile(join(dictDir, name));
      map.set(name, new Uint8Array(data));
    }),
  );

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
