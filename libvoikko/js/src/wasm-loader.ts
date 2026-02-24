import type { VoikkoInitOptions } from './types.js';

// ── Error classes ────────────────────────────────────────────────

/** Base error class for all Voikko errors. */
export class VoikkoError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'VoikkoError';
  }
}

/** Thrown when WASM module loading or initialization fails. */
export class WasmLoadError extends VoikkoError {
  constructor(message: string, options?: { cause?: unknown }) {
    super(message);
    this.name = 'WasmLoadError';
    if (options?.cause) this.cause = options.cause;
  }
}

/** Thrown when dictionary files cannot be loaded. */
export class DictionaryLoadError extends VoikkoError {
  readonly fileName: string;
  constructor(fileName: string, message: string, options?: { cause?: unknown }) {
    super(message);
    this.name = 'DictionaryLoadError';
    this.fileName = fileName;
    if (options?.cause) this.cause = options.cause;
  }
}

// ── Constants ────────────────────────────────────────────────────

/** Dictionary files required for initialization (missing = fatal error) */
const REQUIRED_DICT_FILES = ['index.txt', 'mor.vfst'] as const;

/** Dictionary files that are optional (missing = silently skipped) */
const OPTIONAL_DICT_FILES = ['autocorr.vfst'] as const;

declare const __PKG_VERSION__: string;
const CDN_BASE = `https://unpkg.com/@yongsk0066/voikko@${__PKG_VERSION__}`;

function isNode(): boolean {
  return typeof globalThis.window === 'undefined' && typeof globalThis.document === 'undefined';
}

// ── WASM initialization (cached, with error invalidation) ────────

type WasmModule = typeof import('../wasm/voikko_wasm.js');

let cachedInit: Promise<WasmModule> | null = null;

/**
 * Load and initialize the Rust WASM module. Cached after first call.
 * If initialization fails, the cache is cleared so the next call retries.
 *
 * - Node.js: reads WASM binary from disk (bundled in package)
 * - Browser: fetches from wasmUrl option, or unpkg CDN by default
 */
export function loadWasm(options: VoikkoInitOptions = {}): Promise<WasmModule> {
  if (!cachedInit) {
    cachedInit = (async () => {
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
    })().catch((e) => {
      cachedInit = null;
      throw new WasmLoadError(
        `Failed to load WASM module: ${e instanceof Error ? e.message : String(e)}`,
        { cause: e },
      );
    });
  }
  return cachedInit;
}

// ── Dictionary loading (cached) ──────────────────────────────────

const cachedDicts = new Map<string, Promise<Map<string, Uint8Array>>>();

/**
 * Load dictionary files from the appropriate source.
 * Results are cached by source identifier.
 *
 * Resolution order:
 *   1. options.dictionaryUrl / options.dictionaryPath (explicit)
 *   2. Node.js → bundled dictionary shipped with the npm package
 *   3. Browser → unpkg CDN
 */
export async function loadDict(
  options: VoikkoInitOptions,
): Promise<Map<string, Uint8Array>> {
  let cacheKey: string;

  if (options.dictionaryUrl) {
    cacheKey = options.dictionaryUrl;
  } else if (options.dictionaryPath) {
    cacheKey = options.dictionaryPath;
  } else if (isNode()) {
    cacheKey = '__bundled__';
  } else {
    cacheKey = '__cdn__';
  }

  const cached = cachedDicts.get(cacheKey);
  if (cached) return cached;

  const promise = loadDictImpl(options);
  cachedDicts.set(cacheKey, promise);
  return promise;
}

async function loadDictImpl(
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
      const url = urlFor(name);
      try {
        const response = await fetch(url);
        if (!response.ok) {
          throw new DictionaryLoadError(
            name,
            `Failed to fetch required dictionary file "${name}": ${response.status} ${response.statusText}. Check dictionaryUrl or ensure CDN is accessible`,
            { cause: new Error(`HTTP ${response.status} for ${url}`) },
          );
        }
        map.set(name, new Uint8Array(await response.arrayBuffer()));
      } catch (e) {
        if (e instanceof DictionaryLoadError) throw e;
        throw new DictionaryLoadError(
          name,
          `Failed to fetch required dictionary file "${name}". Check dictionaryUrl or ensure CDN is accessible`,
          { cause: e },
        );
      }
    }),
  );

  const warnings: string[] = [];

  await Promise.all(
    OPTIONAL_DICT_FILES.map(async (name) => {
      try {
        const response = await fetch(urlFor(name));
        if (response.ok) {
          map.set(name, new Uint8Array(await response.arrayBuffer()));
        } else if (response.status !== 404) {
          warnings.push(`Optional dictionary file "${name}": ${response.status} ${response.statusText}`);
        }
      } catch (e) {
        // Network error for optional file — collect warning
        warnings.push(
          `Optional dictionary file "${name}": ${e instanceof Error ? e.message : String(e)}`,
        );
      }
    }),
  );

  if (warnings.length > 0) {
    console.warn(`[voikko] ${warnings.length} optional file(s) failed:\n  ${warnings.join('\n  ')}`);
  }

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
      const filePath = join(dictDir, name);
      try {
        const data = await readFile(filePath);
        map.set(name, new Uint8Array(data));
      } catch (e) {
        throw new DictionaryLoadError(
          name,
          `Failed to read required dictionary file "${name}" from ${dictDir}. Check dictionaryPath or ensure bundled dictionary exists`,
          { cause: e },
        );
      }
    }),
  );

  const warnings: string[] = [];

  await Promise.all(
    OPTIONAL_DICT_FILES.map(async (name) => {
      try {
        const data = await readFile(join(dictDir, name));
        map.set(name, new Uint8Array(data));
      } catch (e: any) {
        // ENOENT (file not found) is expected for optional files — skip silently
        if (e?.code !== 'ENOENT') {
          warnings.push(
            `Optional dictionary file "${name}": ${e instanceof Error ? e.message : String(e)}`,
          );
        }
      }
    }),
  );

  if (warnings.length > 0) {
    console.warn(`[voikko] ${warnings.length} optional file(s) failed:\n  ${warnings.join('\n  ')}`);
  }

  return map;
}
