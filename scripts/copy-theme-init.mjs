#!/usr/bin/env node
/**
 * Copies `theme-init.js` of @zweiund40gmbh/ui to `public/theme-init.js` (generated, git-ignored). The root layout
 * loads it as a file in <head> to apply the stored colour scheme before the first paint without an inline script
 * (CSP `script-src 'self'`). Chained into `dev` and `build` so it follows package updates.
 */
import { copyFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const source = createRequire(import.meta.url).resolve('@zweiund40gmbh/ui/theme-init.js');
const target = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'theme-init.js');
await mkdir(dirname(target), { recursive: true });
await copyFile(source, target);
