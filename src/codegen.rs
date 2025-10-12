use crate::parser::{BinOp, Expr, Function as ParserFunction, Program, Stadment, StrExpr};
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

// Align 'x' up to the next multiple of 'align'
fn align_up(x: u32, align: u32) -> u32 {
    if align == 0 {
        return x;
    }
    let m = align - 1;
    (x + m) & !m
}

// Push a text into the data section at the current cursor position, aligned to 'align' bytes
fn push_text(
    data: &mut DataSection,
    mem_index: u32,
    cursor: &mut u32,
    text: &str,
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
    fn_names: NameMap,
    functions: FunctionSection,
    code: CodeSection,
    data: DataSection,
    data_idx: u32,
    fn_idx: u32,
    fn_map: HashMap<String, i32>,
    ty_void: u32,
    types: TypeSection,
    imports: ImportSection,
}

impl CodeGenerator {
    pub fn new() -> Self {
        let functions = FunctionSection::new();
        let fn_names = NameMap::new();
        let code = CodeSection::new();
        let data = DataSection::new();
        let data_idx = 0u32;
        let fn_idx = 0u32;
        let fn_map: HashMap<String, i32> = HashMap::new();
        let ty_void: u32 = 0;
        let types = TypeSection::new();
        let imports = ImportSection::new();
        Self {
            functions,
            fn_names,
            code,
            data,
            data_idx,
            fn_idx,
            fn_map,
            ty_void,
            types,
            imports,
        }
    }

    // Register a function name and its index in the function map
    pub fn declare_function(&mut self, function: &ParserFunction) {
        self.fn_names.append(self.fn_idx, &function.name);
        self.fn_map
            .insert(function.name.clone(), self.fn_idx as i32);
        self.fn_idx += 1;
    }

    // Generate code for integer expressions
    pub fn gen_expression(&mut self, expr: &Expr, instr: &mut wasm_encoder::InstructionSink<'_>) {
        match expr {
            Expr::Int(i) => {
                instr.i32_const(*i);
            }
            Expr::Binary { op, left, right } => {
                self.gen_expression(left, instr);
                self.gen_expression(right, instr);
                match op {
                    BinOp::Add => {
                        instr.i32_add();
                    }
                    BinOp::Sub => {
                        instr.i32_sub();
                    }
                    BinOp::Mul => {
                        instr.i32_mul();
                    }
                    BinOp::Div => {
                        instr.i32_div_u();
                    }
                }
            }
        }
    }

    // Generate code for string expressions, returning a Blob if the result is a new string
    fn gen_str_expression(
        &mut self,
        expr: &StrExpr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
    ) -> Option<Blob> {
        match expr {
            StrExpr::Str(s) => {
                let blob = push_text(&mut self.data, 0, &mut self.data_idx, &s, 16);
                Some(blob)
            }
            StrExpr::NumToStr(inner) => {
                let inner = &**inner; // dereference the Box
                self.gen_expression(inner, instr); // push the number on the stack
                instr.call(self.fn_map["to_str"] as u32); // convert number to string
                None
            }
        }
    }

    // Generate code for print statement
    pub fn gen_print(
        &mut self,
        str_expr: &Vec<StrExpr>,
        instr: &mut wasm_encoder::InstructionSink<'_>,
    ) {
        match str_expr.as_slice() {
            [] => { /* nothing */ }
            [only] => {
                // if there is only one string to print
                if let Some(blob) = self.gen_str_expression(only, instr) {
                    instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32); // push ptr,len
                }
            }
            [first, rest @ ..] => {
                // if there are multiple strings to print
                if let Some(blob) = self.gen_str_expression(first, instr) {
                    instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32); // push ptr,len
                }
                for e in rest {
                    if let Some(blob) = self.gen_str_expression(e, instr) {
                        instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32); // push ptr,len
                    }
                    instr.call(self.fn_map["concat"] as u32); // concat the two top strings on the stack
                }
            }
        }
        instr.call(self.fn_map["log"] as u32);
    }

    // Generate code for a function
    pub fn gen_function(&mut self, function: &ParserFunction) {
        self.functions.function(self.ty_void);
        let mut fnc = Function::new([]);
        let mut instr = fnc.instructions();
        for stdm in &function.body {
            match stdm {
                Stadment::Print(str_expr) => {
                    self.gen_print(str_expr, &mut instr);
                }
                Stadment::Call { name } => {
                    instr.call(self.fn_map[name] as u32);
                }
            }
        }
        instr.end();
        self.code.function(&fnc);
    }

    // Register an imported function from js
    pub fn push_imported_function(
        &mut self,
        module: &str,
        name: &str, // function name
        params: &[ValType],
        results: &[ValType],
    ) {
        let fn_type = self.types.len();
        self.types
            .ty()
            .function(params.iter().copied(), results.iter().copied());
        self.imports
            .import(module, name, EntityType::Function(fn_type));
        self.fn_names.append(self.fn_idx, name);
        self.fn_map.insert(name.into(), self.fn_idx as i32);
        self.fn_idx += 1;
    }

    // Generate the complete wasm module
    pub fn generate_wasm(&mut self, prog_name: String, prog: &Program) -> Vec<u8> {
        let mut exports = ExportSection::new();
        let mut names = NameSection::new();

        names.module(&prog_name);

        // Types definition
        // void
        self.types.ty().function([], []); // ()->()

        // Imported functions from js

        // env.log(ptr,len) : func (i32,i32)->()
        self.push_imported_function("env", "log", &[ValType::I32, ValType::I32], &[]);
        // str.to_str(n) : func (i32) -> (i32, i32) : return [ptr, len]
        self.push_imported_function(
            "str",
            "to_str",
            &[ValType::I32],
            &[ValType::I32, ValType::I32],
        );
        // str.concat(s1_ptr,s1_len,s2_ptr,s2_len) : func (i32,i32,i32,i32) -> (i32, i32) : return [ptr, len]
        self.push_imported_function(
            "str",
            "concat",
            &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            &[ValType::I32, ValType::I32],
        );

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

        names.functions(&self.fn_names);

        for function in &prog.functions {
            self.gen_function(function);
        }

        for function in &prog.main_program.functions {
            self.gen_function(function);
        }

        self.gen_function(&prog.main_program.main);
        exports.export(
            "main",
            ExportKind::Func,
            self.fn_map.len().saturating_sub(1).try_into().unwrap(),
        );

        // -------- Assemblage --------
        let mut module = Module::new();
        module.section(&self.types);
        module.section(&self.imports);
        module.section(&self.functions);
        module.section(&exports);
        module.section(&self.code);
        module.section(&self.data);
        module.section(&names);
        module.finish()
    }
}
