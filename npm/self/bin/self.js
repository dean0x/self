#!/usr/bin/env node
'use strict';

// self — continuous learning layer for Claude Code
// Shim: detects platform/arch, locates the native binary, and forwards all args.
// Zero dependencies. CommonJS.

var spawnSync = require('child_process').spawnSync;

var PLATFORM_PACKAGES = {
  'linux-x64':    '@dean0x/self-linux-x64',
  'linux-arm64':  '@dean0x/self-linux-arm64',
  'darwin-x64':   '@dean0x/self-darwin-x64',
  'darwin-arm64': '@dean0x/self-darwin-arm64',
  'win32-x64':    '@dean0x/self-windows-x64'
};

var platform = process.platform;
var arch = process.arch;
var key = platform + '-' + arch;
var pkgName = PLATFORM_PACKAGES[key];

if (!pkgName) {
  process.stderr.write(
    'error: @dean0x/self does not support ' + platform + '/' + arch + '\n' +
    'Supported platforms: linux/x64, linux/arm64, darwin/x64, darwin/arm64, win32/x64\n' +
    'If your platform should be supported, open an issue at https://github.com/dean0x/self\n'
  );
  process.exit(1);
}

var binName = platform === 'win32' ? 'self.exe' : 'self';
var exe;

try {
  exe = require.resolve(pkgName + '/bin/' + binName);
} catch (_err) {
  process.stderr.write(
    'error: @dean0x/self: missing native binary for ' + platform + '/' + arch + '\n' +
    'Expected package: ' + pkgName + '\n' +
    'Detected platform: ' + platform + ', arch: ' + arch + '\n' +
    'Likely causes:\n' +
    '  1. Your platform (' + platform + '/' + arch + ') is not supported\n' +
    '  2. npm install was run with --omit=optional, skipping platform packages\n' +
    'Fix: run  npm install  without --omit=optional, or install ' + pkgName + ' manually.\n'
  );
  process.exit(1);
}

var result = spawnSync(exe, process.argv.slice(2), { stdio: 'inherit' });
process.exit(result.status !== null ? result.status : 1);
