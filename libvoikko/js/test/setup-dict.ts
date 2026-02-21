/**
 * Vitest globalSetup: auto-detect voikko-fi dictionary from monorepo.
 *
 * Priority:
 * 1. VOIKKO_DICT_PATH env var (explicit, use as-is)
 * 2. ../../voikko-fi/vvfst/ (monorepo auto-detect)
 */
import { existsSync } from 'node:fs';
import { resolve, join } from 'node:path';

const DICT_FILES = ['index.txt', 'mor.vfst', 'autocorr.vfst'] as const;

export async function setup() {
  if (process.env['VOIKKO_DICT_PATH']) {
    return;
  }

  const vvfstDir = resolve(__dirname, '..', '..', '..', 'voikko-fi', 'vvfst');
  const hasDict = DICT_FILES.every((f) => existsSync(join(vvfstDir, f)));

  if (hasDict) {
    process.env['VOIKKO_DICT_PATH'] = vvfstDir;
  }
}
