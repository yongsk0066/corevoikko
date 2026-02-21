import type {
  EmscriptenModule,
  EmscriptenModuleFactory,
  VoikkoInitOptions,
} from './types.js';

/** Dictionary files required for Finnish V5 format */
const DICT_FILES = ['index.txt', 'mor.vfst', 'autocorr.vfst'] as const;

/** VFS path matching V5DictionaryLoader convention: {root}/5/mor-{variant}/ */
const DICT_VFS_BASE = '/5/mor-standard';

// ── WASM loading (cached) ──────────────────────────────────────────

let cachedModule: Promise<EmscriptenModule> | null = null;

/**
 * Load the Emscripten WASM module. Cached after first call —
 * the binary is identical regardless of language or dictionary.
 */
export function loadWasm(
  locateFile?: (file: string, prefix: string) => string,
): Promise<EmscriptenModule> {
  return (cachedModule ??= (async () => {
    const { default: createModule } = (await import(
      '../wasm/libvoikko.mjs'
    )) as { default: EmscriptenModuleFactory };

    return createModule(locateFile ? { locateFile } : {});
  })());
}

// ── Dictionary loading ─────────────────────────────────────────────

/**
 * Load dictionary files from the appropriate source.
 * Returns filename → bytes entries, agnostic of Emscripten.
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

// ── VFS mounting ───────────────────────────────────────────────────

/**
 * Mount dictionary files into the Emscripten virtual filesystem.
 * Safe to call multiple times — overwrites are idempotent.
 */
export function mountDict(
  module: EmscriptenModule,
  files: Map<string, Uint8Array>,
): void {
  try { module.FS.mkdir('/5'); } catch { /* already exists */ }
  try { module.FS.mkdir(DICT_VFS_BASE); } catch { /* already exists */ }

  for (const [name, data] of files) {
    module.FS.writeFile(`${DICT_VFS_BASE}/${name}`, data);
  }
}
