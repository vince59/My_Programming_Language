use crate::parser::{Program, Stadment, Function as ParserFunction};
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, ImportSection, MemoryType, Module, NameMap, NameSection, TypeSection, ValType,
};

use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Blob {
    ptr: u32,
    len: u32,
}

fn align_up(x: u32, align: u32) -> u32 {
    if align == 0 {
        return x;
    }
    let m = align - 1;
    (x + m) & !m
}

fn push_text(
    data: &mut DataSection,
    mem_index: u32,
    cursor: &mut u32,
    text: &str, // <- accepte &String aussi
    align: u32,
) -> Blob {
    *cursor = align_up(*cursor, align);
    let ptr = *cursor;
    // String -> bytes UTF-8
    let bytes = text.as_bytes();
    data.active(
        mem_index,
        &ConstExpr::i32_const(ptr as i32),
        bytes.iter().copied(),
    );
    *cursor += bytes.len() as u32;
    Blob {
        ptr,
        len: bytes.len() as u32,
    }
}

pub struct CodeGenerator {
    types: TypeSection,
    imports: ImportSection,
    exports: ExportSection,
    functions: FunctionSection,
    names: NameSection,
    code: CodeSection,
    data: DataSection,
    data_idx: u32,
    fn_names: NameMap,
    fn_idx: u32,
    fn_map: HashMap<String, i32>,
    ty_void: u32
}

impl CodeGenerator {
    pub fn new() -> Self {
        let types = TypeSection::new();
        let imports = ImportSection::new();
        let exports = ExportSection::new();
        let functions = FunctionSection::new();
        let names = NameSection::new();
        let code = CodeSection::new();
        let data = DataSection::new();
        let data_idx = 0u32;
        let fn_names = NameMap::new();
        let fn_idx = 0u32;
        let fn_map: HashMap<String, i32> = HashMap::new();
        let ty_void: u32 = 0;
        Self {
            types,
            imports,
            exports,
            functions,
            names,
            code,
            data,
            data_idx,
            fn_names,
            fn_idx,
            fn_map,
            ty_void
        }
    }

pub fn declare_function(&mut self, function: &ParserFunction) {
        self.fn_names.append(self.fn_idx, &function.name);
        self.fn_map
            .insert(function.name.clone(), self.fn_idx as i32);
        println!("{}",function.name);
        self.fn_idx += 1;
    }

    pub fn gen_function(&mut self, function: &ParserFunction) {
        self.functions.function(self.ty_void);
        let mut fnc = Function::new([]);
        let mut instr = fnc.instructions();
        for stdm in &function.body {
            match stdm {
                Stadment::Print { text } => {
                    let blob = push_text(&mut self.data, 0, &mut self.data_idx, &text, 16);
                    instr
                        .i32_const(blob.ptr as i32)
                        .i32_const(blob.len as i32)
                        .call(self.fn_map["log"] as u32);
                }
                Stadment::Call { name } => {
                    println!("xxxx {}",name);
                    instr.call(self.fn_map[name] as u32);
                }
            }
        }
        instr.end();
        self.code.function(&fnc);
    }

    pub fn generate_wasm(&mut self, prog_name: String, prog: &Program) -> Vec<u8> {
        self.names.module(&prog_name);

        // Common types
        // void
        
        self.types.ty().function([], []); // ()->()

        // Imported functions from js
        // env.log(ptr,len) : func (i32,i32)->()
        let ty_log = self.types.len();
        self.types.ty().function([ValType::I32, ValType::I32], []); // (i32,i32)->()
        self.imports
            .import("env", "log", EntityType::Function(ty_log));
        self.fn_names.append(self.fn_idx, "log");
        self.fn_map.insert("log".into(), self.fn_idx as i32);
        self.fn_idx += 1;

        // Imported memory from js
        self.imports.import(
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

        for function in &prog.functions {
            self.declare_function(function);
        }

        for function in &prog.main_program.functions {
            self.declare_function(function);
        }

        self.declare_function(&prog.main_program.main);

        for function in &prog.functions {
            self.gen_function(function);
        }

        for function in &prog.main_program.functions {
            self.gen_function(function);
        }

        self.gen_function(&prog.main_program.main);

        self.exports.export("main", ExportKind::Func, 3);

        // -------- Assemblage --------
        let mut module = Module::new();
        module.section(&self.types);
        module.section(&self.imports);
        module.section(&self.functions);
        module.section(&self.exports);
        module.section(&self.code);
        module.section(&self.data);
        module.section(&self.names); // ajoute la name section en fin de module
        module.finish()
    }
}
