/**
 * Ferrum JavaScript Core Library
 *
 * This file provides the core JavaScript APIs that are available
 * in all Ferrum runtimes.
 */

// Global console object
globalThis.console = {
  log: function(...args) {
    // This will be bound to the native implementation
    nativeConsole.log(...args);
  },
  error: function(...args) {
    nativeConsole.error(...args);
  },
  warn: function(...args) {
    nativeConsole.warn(...args);
  },
  info: function(...args) {
    nativeConsole.info(...args);
  },
  debug: function(...args) {
    nativeConsole.debug(...args);
  },
  trace: function(...args) {
    nativeConsole.trace(...args);
  },
  assert: function(condition, ...args) {
    if (!condition) {
      nativeConsole.assert(...args);
    }
  }
};

// setTimeout and setInterval
globalThis.setTimeout = function(callback, delay, ...args) {
  return nativeSetTimeout(callback, delay, args);
};

globalThis.setInterval = function(callback, delay, ...args) {
  return nativeSetInterval(callback, delay, args);
};

globalThis.clearTimeout = function(id) {
  nativeClearTimeout(id);
};

globalThis.clearInterval = function(id) {
  nativeClearInterval(id);
};

// Fetch API
globalThis.fetch = function(url, options) {
  return nativeFetch(url, options);
};

// Text encoding/decoding
globalThis.TextEncoder = TextEncoder;
globalThis.TextDecoder = TextDecoder;

// URL APIs
globalThis.URL = URL;
globalThis.URLSearchParams = URLSearchParams;

// EventTarget base class
globalThis.EventTarget = EventTarget;

// Performance API
globalThis.performance = {
  now: function() {
    return nativePerformanceNow();
  }
};

// Crypto API (minimal)
globalThis.crypto = {
  randomUUID: function() {
    return nativeRandomUUID();
  },
  getRandomValues: function(array) {
    return nativeGetRandomValues(array);
  }
};

// Process information
globalThis.Deno = {
  build: {
    target: "unknown",
    arch: "unknown",
    os: "unknown",
    vendor: "unknown",
    env: "unknown"
  },
  version: {
    deno: "0.0.0",
    v8: "unknown",
    typescript: "unknown"
  },
  args: [], // Will be populated from Rust side
  pid: nativeGetPid(),
  cwd: function() {
    return nativeCwd();
  },
  env: {
    get: function(key) {
      return nativeEnvGet(key);
    },
    set: function(key, value) {
      nativeEnvSet(key, value);
    },
    delete: function(key) {
      nativeEnvDelete(key);
    },
    toJSON: function() {
      return nativeEnvToJSON();
    }
  },
  stdout: {
    write: function(data) {
      return nativeStdoutWrite(data);
    }
  },
  stderr: {
    write: function(data) {
      return nativeStderrWrite(data);
    }
  },
  stdin: {
    read: function() {
      return nativeStdinRead();
    }
  },
  exit: function(code = 0) {
    nativeExit(code);
  }
};

// File system APIs
globalThis.Deno.readTextFile = function(path) {
  return nativeReadTextFile(path);
};

globalThis.Deno.readFileSync = function(path) {
  return nativeReadTextFileSync(path);
};

globalThis.Deno.writeTextFile = function(path, data) {
  return nativeWriteTextFile(path, data);
};

globalThis.Deno.writeTextFileSync = function(path, data) {
  return nativeWriteTextFileSync(path, data);
};

globalThis.Deno.readFile = function(path) {
  return nativeReadFile(path);
};

globalThis.Deno.readFileSync = function(path) {
  return nativeReadFileSync(path);
};

globalThis.Deno.writeFile = function(path, data) {
  return nativeWriteFile(path, data);
};

globalThis.Deno.writeFileSync = function(path, data) {
  return nativeWriteFileSync(path, data);
};

globalThis.Deno.mkdir = function(path, options) {
  return nativeMkdir(path, options);
};

globalThis.Deno.mkdirSync = function(path, options) {
  return nativeMkdirSync(path, options);
};

globalThis.Deno.remove = function(path, options) {
  return nativeRemove(path, options);
};

globalThis.Deno.removeSync = function(path, options) {
  return nativeRemoveSync(path, options);
};

globalThis.Deno.stat = function(path) {
  return nativeStat(path);
};

globalThis.Deno.statSync = function(path) {
  return nativeStatSync(path);
};

globalThis.Deno.readDir = function(path) {
  return nativeReadDir(path);
};

globalThis.Deno.readDirSync = function(path) {
  return nativeReadDirSync(path);
};

// Export for module use
export default {};
