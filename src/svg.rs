use crate::parser::Program;
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, Function, FunctionSection,
    ImportSection, ExportSection, MemoryType, Module, TypeSection, ValType,
};

pub fn generate_wasm(prog: &Program) -> Vec<u8> {
    // -------- Types --------
    let mut types = TypeSection::new();

    // indices de types via types.len() avant insertion
    let ty_log = types.len();
    types.ty().function([ValType::I32, ValType::I32], []); // (i32,i32)->()

    let ty_void = types.len();
    types.ty().function([], []); // ()->()

    // -------- Imports --------
    let mut imports = ImportSection::new();
    // env.log : func (i32,i32)->()
    imports.import("env", "log", EntityType::Function(ty_log));
    // env.memory : memory importée (min 1 page). Pas d'export de mémoire.
    imports.import(
        "env",
        "memory",
        EntityType::Memory(MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        }),
    );

    // -------- Fonctions locales --------
    // Indices de fonctions (imports d'abord) :
    // 0: env.log     (import)
    // 1: hello_from_unit     (local)
    // 2: hello_from_utils    (local)
    // 3: main                (local)
    let mut functions = FunctionSection::new();
    functions.function(ty_void); // hello_from_unit
    functions.function(ty_void); // hello_from_utils
    functions.function(ty_void); // main

    // -------- Corps des fonctions --------
    let mut code = CodeSection::new();

    // hello_from_unit(): log(ptr=64, len=16)
    let mut f_unit = Function::new([]);
    f_unit
        .instructions()
        .i32_const(64)
        .i32_const(16)
        .call(0) // env.log
        .end();
    code.function(&f_unit);

    // hello_from_utils(): log(ptr=96, len=17)
    let mut f_utils = Function::new([]);
    f_utils
        .instructions()
        .i32_const(96)
        .i32_const(17)
        .call(0) // env.log
        .end();
    code.function(&f_utils);

    // main(): log "Hello from mpl !" puis appelle unit et utils
    let mut f_main = Function::new([]);
    f_main
        .instructions()
        .i32_const(0)   // ptr "Hello from mpl !"
        .i32_const(16)  // len
        .call(0)        // env.log
        .call(1)        // hello_from_unit
        .call(2)        // hello_from_utils
        .end();
    code.function(&f_main);

    // -------- Données (dans la mémoire importée 0) --------
    let mut data = DataSection::new();
    // Offsets choisis pour ne pas se chevaucher
    let hello_main: &'static [u8; 16] = b"Hello from mpl !"; // 16
    let hello_unit = b"hello from unit!"; // 16
    let hello_utils = b"hello from utils!"; // 17

    // place "Hello from mpl !" @ 0
    data.active(0, &ConstExpr::i32_const(0), hello_main.iter().copied());
    // place "hello from unit!" @ 64
    data.active(0, &ConstExpr::i32_const(64), hello_unit.iter().copied());
    // place "hello from utils!" @ 96
    data.active(0, &ConstExpr::i32_const(96), hello_utils.iter().copied());

    // -------- Exports --------
    let mut exports = ExportSection::new();
    // Exporter uniquement main (index de fonction = 3)
    exports.export("main", ExportKind::Func, 3);

    // -------- Assemblage --------
    let mut module = Module::new();
    module.section(&types);
    module.section(&imports);
    module.section(&functions);
    module.section(&exports);
    module.section(&code);
    module.section(&data);
    
    module.finish()
}
