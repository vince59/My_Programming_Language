import fs from "node:fs/promises";

// mémoire partagée importée par le module
const memory = new WebAssembly.Memory({ initial: 1 });

// impl de env.log lisant dans la mémoire importée
const td = new TextDecoder("utf-8");
const env = {
  memory,
  log(ptr, len) {
    const bytes = new Uint8Array(memory.buffer, ptr, len);
    const s = td.decode(bytes);
    console.log(s);
  },
};

// charge le wasm généré par Rust
const bytes = await fs.readFile("./js/app.wasm");

// instancie avec { env }
const { instance } = await WebAssembly.instantiate(bytes, { env });

// appelle la seule exportée
instance.exports.main();