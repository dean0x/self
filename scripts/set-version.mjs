#!/usr/bin/env node
// set-version.mjs — single source of truth for version bumps.
// Usage: node scripts/set-version.mjs <x.y.z>
// Rewrites: Cargo.toml (package.version) + all six npm package.json files
//           (version field + optionalDependencies pins in the main package).
// Zero external dependencies.

import { readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');

function die(msg) {
  process.stderr.write('error: ' + msg + '\n');
  process.exit(1);
}

// ---------- argument validation ----------

const version = process.argv[2];
if (!version) die('usage: set-version.mjs <x.y.z>');
if (!/^\d+\.\d+\.\d+$/.test(version)) die('invalid semver: ' + version + ' (expected x.y.z)');

// ---------- Cargo.toml ----------

const cargoPath = resolve(ROOT, 'Cargo.toml');
const cargoSrc = readFileSync(cargoPath, 'utf8');

// Match the `version = "..."` line inside the [package] section only.
// \nversion anchors to line-start, ruling out keys like rust-version.
// [^\[]* stops at the next TOML section header — safe even if the file grows.
const cargoPkgRe = /(\[package\][^\[]*\nversion\s*=\s*)"[^"]*"/;
if (!cargoPkgRe.test(cargoSrc)) die('could not locate `version = "..."` line in Cargo.toml [package] section');
const newCargo = cargoSrc.replace(cargoPkgRe, '$1"' + version + '"');
writeFileSync(cargoPath, newCargo, 'utf8');
console.log('Cargo.toml              → ' + version);

// ---------- platform package.json files ----------

const PLATFORMS = [
  'linux-x64',
  'linux-arm64',
  'darwin-x64',
  'darwin-arm64',
  'windows-x64',
];

for (const plat of PLATFORMS) {
  const pkgPath = resolve(ROOT, 'npm', 'platforms', plat, 'package.json');
  const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
  pkg.version = version;
  writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n', 'utf8');
  console.log('npm/platforms/' + plat + '/package.json → ' + version);
}

// ---------- main package.json ----------

const mainPath = resolve(ROOT, 'npm', 'self', 'package.json');
const main = JSON.parse(readFileSync(mainPath, 'utf8'));
main.version = version;

if (main.optionalDependencies) {
  for (const dep of Object.keys(main.optionalDependencies)) {
    main.optionalDependencies[dep] = version;
  }
}

writeFileSync(mainPath, JSON.stringify(main, null, 2) + '\n', 'utf8');
console.log('npm/self/package.json   → ' + version);

console.log('\nAll 7 version sites set to ' + version);
