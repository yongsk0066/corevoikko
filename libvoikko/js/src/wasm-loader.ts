import type {
  EmscriptenModule,
  EmscriptenModuleFactory,
  VoikkoInitOptions,
} from './types.js';

/** Dictionary files required for Finnish V5 format */
const DICT_FILES = ['index.txt', 'mor.vfst', 'autocorr.vfst'] as const;

/** VFS path matching V5DictionaryLoader convention: {root}/5/mor-{variant}/ */
const DICT_VFS_BASE = '/5/mor-standard';

/**
 * Fetch dictionary files from a URL (browser environment).
 */
async function fetchDictionary(baseUrl: string): Promise<Map<string, Uint8Array>> {
  const base = baseUrl.endsWith('/') ? baseUrl : baseUrl + '/';
  const files = new Map<string, Uint8Array>();

  const fetches = DICT_FILES.map(async (name) => {
    const url = `${base}5/mor-standard/${name}`;
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch dictionary file ${name}: ${response.status} ${response.statusText}`);
    }
    files.set(name, new Uint8Array(await response.arrayBuffer()));
  });

  await Promise.all(fetches);
  return files;
}

/**
 * Read dictionary files from local filesystem (Node.js environment).
 */
async function readDictionaryFromFs(dirPath: string): Promise<Map<string, Uint8Array>> {
  const { readFile } = await import('node:fs/promises');
  const { join } = await import('node:path');

  const files = new Map<string, Uint8Array>();
  const dictDir = join(dirPath, '5', 'mor-standard');

  for (const name of DICT_FILES) {
    const data = await readFile(join(dictDir, name));
    files.set(name, new Uint8Array(data));
  }

  return files;
}

/**
 * Mount dictionary files into the Emscripten virtual filesystem.
 */
function mountDictToVfs(module: EmscriptenModule, files: Map<string, Uint8Array>): void {
  try { module.FS.mkdir('/5'); } catch { /* already exists */ }
  try { module.FS.mkdir(DICT_VFS_BASE); } catch { /* already exists */ }

  for (const [name, data] of files) {
    module.FS.writeFile(`${DICT_VFS_BASE}/${name}`, data);
  }
}

/**
 * Load and initialize the Emscripten WASM module with dictionary data.
 */
export async function loadModule(options: VoikkoInitOptions): Promise<EmscriptenModule> {
  // Dynamic import of the Emscripten ESM glue (preserved as external by tsdown)
  const { default: createModule } = await import(
    '../wasm/libvoikko.mjs'
  ) as { default: EmscriptenModuleFactory };

  const moduleOverrides: Record<string, unknown> = {};
  if (options.locateFile) {
    moduleOverrides.locateFile = options.locateFile;
  }

  const module = await createModule(moduleOverrides);

  // Load dictionary files from the appropriate source
  let dictFiles: Map<string, Uint8Array>;
  if (options.dictionaryUrl) {
    dictFiles = await fetchDictionary(options.dictionaryUrl);
  } else if (options.dictionaryPath) {
    dictFiles = await readDictionaryFromFs(options.dictionaryPath);
  } else {
    throw new Error(
      'Voikko: either dictionaryUrl (browser) or dictionaryPath (Node.js) must be provided',
    );
  }

  mountDictToVfs(module, dictFiles);
  return module;
}
