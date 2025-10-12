import fs from "node:fs/promises";

// mémoire partagée importée par le module
const memory = new WebAssembly.Memory({ initial: 1 });

// --- petit allocateur bump ---
const PAGE = 65536;
let heap = 4096; // base du tas (évite d’écraser tes segments de données statiques)

function align(n, a = 16) { return (n + (a - 1)) & ~(a - 1); }
function ensure(end) {
  const needPages = Math.ceil(end / PAGE);
  const curPages = memory.buffer.byteLength / PAGE;
  if (needPages > curPages) memory.grow(needPages - curPages);
}
function alloc(len) {
  heap = align(heap);
  const ptr = heap >>> 0;
  heap += len >>> 0;
  ensure(heap);
  return ptr;
}

// impl de env.log lisant dans la mémoire importée
const td = new TextDecoder("utf-8");
const te = new TextEncoder();

const env = {
  memory,
  log(ptr, len) {
    const bytes = new Uint8Array(memory.buffer, ptr >>> 0, len >>> 0);
    const s = td.decode(bytes);
    console.log(s);
  },
};

// espace 'str' importé par le wasm
const str = {
  // convert integer to string (i32) -> (i32, i32) : retourne [ptr, len]
  to_str(n) {
    n = n | 0;
    const bytes = te.encode(String(n));
    const ptr = alloc(bytes.length);
    new Uint8Array(memory.buffer, ptr, bytes.length).set(bytes);
    return [ptr, bytes.length | 0];
  },

  // concat two strings (l_ptr,i32, l_len,i32, r_ptr,i32, r_len,i32) -> (ptr,len)
  concat(lPtr, lLen, rPtr, rLen) {
    lPtr = lPtr >>> 0; lLen = lLen >>> 0;
    rPtr = rPtr >>> 0; rLen = rLen >>> 0;

    const total = (lLen + rLen) >>> 0;
    const outPtr = alloc(total);
    const dst = new Uint8Array(memory.buffer, outPtr, total);

    const lhs = new Uint8Array(memory.buffer, lPtr, lLen);
    const rhs = new Uint8Array(memory.buffer, rPtr, rLen);
    dst.set(lhs, 0);
    dst.set(rhs, lLen);
    return [outPtr, total];
  },
};

// charge le wasm généré par Rust
const bytes = await fs.readFile("./js/app.wasm");

// instancie avec { env, str } (multi-retours requis côté runtime)
const { instance } = await WebAssembly.instantiate(bytes, { env, str });

// appelle la seule exportée
instance.exports.main();
