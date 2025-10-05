const fs = require("fs");
const path = require("path");
const { TextDecoder } = require("util");

// charge le binaire
const wasmPath = path.join(__dirname, "app.wasm");
const bytes = fs.readFileSync(wasmPath);

// prépare imports
const memory = new WebAssembly.Memory({ initial: 1 }); // = 64KiB
const decoder = new TextDecoder("utf-8");

const imports = {
  env: {
    memory,
    log(ptr, len) {
      const view = new Uint8Array(memory.buffer, ptr, len);
      console.log(decoder.decode(view));
    },
  },
};

(async () => {
  const { instance } = await WebAssembly.instantiate(bytes, imports);
  // appelle la seule fonction exportée
  instance.exports.main();
})();