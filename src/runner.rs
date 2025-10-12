// runner.rs (wasmi 0.51.x)
// Provides: env.memory, env.log, str.to_str, str.concat
// Uses exported mutable global 'heap_ptr' as a bump allocator.

use anyhow::{anyhow, Result};
use std::{fs, path::Path, sync::{Arc, Mutex}};
use wasmi::{Caller, Engine, Func, Linker, Memory, MemoryType, Module, Store, TypedFunc, Val};

#[inline]
fn align_up(x: u32, align: u32) -> u32 {
    (x + (align - 1)) & !(align - 1)
}

/// Read a slice from guest memory.
fn read_slice(mem: &Memory, caller: &mut Caller<'_, ()>, ptr: u32, len: u32) -> Vec<u8> {
    let mut buf = vec![0u8; len as usize];
    // wasmi 0.51: Memory::read takes &Caller (or &mut Caller); both work.
    mem.read(&*caller, ptr as usize, &mut buf).expect("mem read");
    buf
}

/// Write a slice into guest memory.
fn write_slice(mem: &Memory, caller: &mut Caller<'_, ()>, ptr: u32, data: &[u8]) {
    mem.write(&mut *caller, ptr as usize, data).expect("mem write");
}

pub fn run_wasm_bytes(wasm_bytes: &[u8]) -> Result<()> {
    let engine = Engine::default();
    let module = Module::new(&engine, wasm_bytes)?;

    // Thread-safe cell to store the exported 'heap_ptr' Global after instantiation.
    let heap_ptr_cell: Arc<Mutex<Option<wasmi::Global>>> = Arc::new(Mutex::new(None));

    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // 1) Imported memory: env.memory
    let memory_ty = MemoryType::new(1, None); // not a Result in 0.51
    let memory = Memory::new(&mut store, memory_ty)?;
    linker.define("env", "memory", memory)?;

    // 2) env.log(ptr: i32, len: i32) -> ()
    {
        let mem = memory;
        linker.func_wrap("env", "log", move |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let bytes = read_slice(&mem, &mut caller, ptr as u32, len as u32);
            println!("{}", String::from_utf8_lossy(&bytes));
        })?;
    }

    // 3) str.to_str(n: i32) -> (ptr: i32, len: i32)
    {
        let mem = memory;
        let heap_cell = Arc::clone(&heap_ptr_cell);
        linker.func_wrap("str", "to_str", move |mut caller: Caller<'_, ()>, n: i32| -> (i32, i32) {
            let s = n.to_string();
            let bytes = s.as_bytes();

            let heap = {
                let guard = heap_cell.lock().unwrap();
                guard.as_ref().cloned().expect("heap_ptr global not set yet")
            };

            let cur = match heap.get(&caller) {
                Val::I32(v) => v as u32,
                _ => panic!("heap_ptr must be i32"),
            };
            let ptr = cur;

            write_slice(&mem, &mut caller, ptr, bytes);

            let next = align_up(ptr + bytes.len() as u32, 16);
            heap.set(&mut caller, Val::I32(next as i32)).expect("set heap_ptr");

            (ptr as i32, bytes.len() as i32)
        })?;
    }

    // 4) str.concat(s1_ptr,s1_len,s2_ptr,s2_len) -> (ptr,len)
    {
        let mem = memory;
        let heap_cell = Arc::clone(&heap_ptr_cell);
        linker.func_wrap(
            "str",
            "concat",
            move |mut caller: Caller<'_, ()>, p1: i32, l1: i32, p2: i32, l2: i32| -> (i32, i32) {
                let b1 = read_slice(&mem, &mut caller, p1 as u32, l1 as u32);
                let b2 = read_slice(&mem, &mut caller, p2 as u32, l2 as u32);

                let heap = {
                    let guard = heap_cell.lock().unwrap();
                    guard.as_ref().cloned().expect("heap_ptr global not set yet")
                };

                let cur = match heap.get(&caller) {
                    Val::I32(v) => v as u32,
                    _ => panic!("heap_ptr must be i32"),
                };
                let ptr = cur;

                write_slice(&mem, &mut caller, ptr, &b1);
                write_slice(&mem, &mut caller, ptr + b1.len() as u32, &b2);

                let total = (b1.len() + b2.len()) as u32;
                let next = align_up(ptr + total, 16);
                heap.set(&mut caller, Val::I32(next as i32)).expect("set heap_ptr");

                (ptr as i32, total as i32)
            },
        )?;
    }

    // Instantiate and run start (if any).
    let instance = linker.instantiate_and_start(&mut store, &module)?;

    // Fetch exported global 'heap_ptr' and store it for host funcs.
    let heap_global = instance
        .get_global(&store, "heap_ptr")
        .ok_or_else(|| anyhow!("export 'heap_ptr' not found"))?;
    *heap_ptr_cell.lock().unwrap() = Some(heap_global);

    // Call exported 'main'.
    let main_fn: TypedFunc<(), ()> = instance.get_typed_func(&store, "main")?;
    main_fn.call(&mut store, ())?;

    Ok(())
}

pub fn run_wasm_file<P: AsRef<Path>>(path: P) -> Result<()> {
    let bytes = fs::read(path)?;
    run_wasm_bytes(&bytes)
}
