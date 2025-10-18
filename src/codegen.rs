use crate::parser::{BinOp, Expr, Function as ParserFunction, Program, Stadment, StrExpr};
use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, Function,
    FunctionSection, GlobalSection, GlobalType, ImportSection, MemoryType, Module, NameMap,
    NameSection, TypeSection, ValType,
};

use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Blob {
    ptr: u32,
    len: u32,
}

// Aligne 'x' vers le multiple supérieur de 'align'
fn align_up(x: u32, align: u32) -> u32 {
    if align == 0 {
        return x;
    }
    let m = align - 1;
    (x + m) & !m
}

// Place du texte dans la DataSection à la position 'cursor', alignée sur 'align'
fn push_text(
    data: &mut DataSection,
    mem_index: u32,
    cursor: &mut u32,
    text: &str,
    align: u32,
) -> Blob {
    *cursor = align_up(*cursor, align);
    let ptr = *cursor;
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

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Ty {
    I32,
    F64,
}
pub struct CodeGenerator {
    // sections
    types: TypeSection,
    imports: ImportSection,
    functions: FunctionSection,
    code: CodeSection,
    data: DataSection,
    exports: ExportSection,
    names: NameSection,
    globals: GlobalSection,

    // bookkeeping
    fn_names: NameMap,
    fn_idx: u32,
    fn_map: HashMap<String, i32>,
    data_idx: u32,
    ty_void: u32,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            types: TypeSection::new(),
            imports: ImportSection::new(),
            functions: FunctionSection::new(),
            code: CodeSection::new(),
            data: DataSection::new(),
            exports: ExportSection::new(),
            names: NameSection::new(),
            globals: GlobalSection::new(),

            fn_names: NameMap::new(),
            fn_idx: 0,
            fn_map: HashMap::new(),
            data_idx: 0,
            ty_void: 0, // sera 0 après ajout de ()->()
        }
    }

    // Enregistre un nom de fonction pour la NameSection et map nom -> index
    pub fn declare_function(&mut self, function: &ParserFunction) {
        self.fn_names.append(self.fn_idx, &function.name);
        self.fn_map
            .insert(function.name.clone(), self.fn_idx as i32);
        self.fn_idx += 1;
    }

    // Decide the resulting type of an expression.
    // Rule: if any side is F64, result is F64; otherwise I32.
    fn infer_type(&self, e: &Expr) -> Ty {
        match e {
            Expr::Int(_) => Ty::I32,
            Expr::Real(_) => Ty::F64,
            Expr::Binary { left, right, .. } => {
                let lt = self.infer_type(left);
                let rt = self.infer_type(right);
                if lt == Ty::F64 || rt == Ty::F64 {
                    Ty::F64
                } else {
                    Ty::I32
                }
            }
        }
    }

    // Emit `expr` as `target` type, inserting implicit casts as needed.
    // Allowed: i32 -> f64 (widen) and f64 -> i32 (narrow via trunc toward zero).
    fn gen_expression_as(
        &mut self,
        expr: &Expr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
        target: Ty,
    ) {
        match expr {
            Expr::Int(i) => {
                instr.i32_const(*i);
                if target == Ty::F64 {
                    // signed i32 -> f64
                    instr.f64_convert_i32_s();
                }
            }
            Expr::Real(r) => {
                match target {
                    Ty::F64 => {
                        instr.f64_const((*r).into());
                    }
                    Ty::I32 => {
                        // f64 -> i32 (trunc toward zero, traps on NaN or out-of-range)
                        instr.f64_const((*r).into());
                        instr.i32_trunc_f64_s();
                    }
                }
            }
            Expr::Binary { op, left, right } => {
                let target_ty = target;
                // make both operands the same target type
                self.gen_expression_as(left, instr, target_ty);
                self.gen_expression_as(right, instr, target_ty);

                match (op, target_ty) {
                    (BinOp::Add, Ty::I32) => instr.i32_add(),
                    (BinOp::Sub, Ty::I32) => instr.i32_sub(),
                    (BinOp::Mul, Ty::I32) => instr.i32_mul(),
                    (BinOp::Div, Ty::I32) => instr.i32_div_s(), // signed division

                    (BinOp::Add, Ty::F64) => instr.f64_add(),
                    (BinOp::Sub, Ty::F64) => instr.f64_sub(),
                    (BinOp::Mul, Ty::F64) => instr.f64_mul(),
                    (BinOp::Div, Ty::F64) => instr.f64_div(),
                };
            }
        }
    }

    // Public entry: generate code and return the resulting type.
    pub fn gen_expression(
        &mut self,
        expr: &Expr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
    ) -> Ty {
        let target = self.infer_type(expr);
        self.gen_expression_as(expr, instr, target);
        target
    }
    // Expressions de chaînes (littéral -> Blob; conversion/concat -> valeurs déjà sur la pile)
    fn gen_str_expression(
        &mut self,
        expr: &StrExpr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
    ) -> Option<Blob> {
        match expr {
            StrExpr::Str(s) => {
                // place la chaîne dans la mémoire importée via DataSection
                let blob = push_text(&mut self.data, 0, &mut self.data_idx, s, 16);
                Some(blob)
            }
            StrExpr::NumToStr(inner) => {
                let inner = &**inner; 
                match self.gen_expression(inner, instr) { // push n
                    Ty::I32 => {
                        instr.call(self.fn_map["to_str_i32"] as u32); // (i32)->(i32,i32): [ptr,len]
                    }
                    Ty::F64 => {
                        instr.call(self.fn_map["to_str_f64"] as u32); // (f64)->(i32,i32): [ptr,len]
                    }
                }
                None
            }
        }
    }

    // print([...]) -> construit (ptr,len) puis appelle env.log(ptr,len)
    pub fn gen_print(
        &mut self,
        str_expr: &Vec<StrExpr>,
        instr: &mut wasm_encoder::InstructionSink<'_>,
    ) {
        match str_expr.as_slice() {
            [] => {}
            [only] => {
                if let Some(blob) = self.gen_str_expression(only, instr) {
                    instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32);
                }
            }
            [first, rest @ ..] => {
                if let Some(blob) = self.gen_str_expression(first, instr) {
                    instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32);
                }
                for e in rest {
                    if let Some(blob) = self.gen_str_expression(e, instr) {
                        instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32);
                    }
                    // stack: ... s1_ptr s1_len s2_ptr s2_len -> concat -> s_ptr s_len
                    instr.call(self.fn_map["concat"] as u32);
                }
            }
        }
        instr.call(self.fn_map["log"] as u32);
    }

    pub fn gen_function(&mut self, function: &ParserFunction) {
        self.functions.function(self.ty_void); // ()->()
        let mut fnc = Function::new([]); // pas de locals
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

    // Déclare une fonction importée (module, name, (params)->(results))
    pub fn push_imported_function(
        &mut self,
        module: &str,
        name: &str,
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

    pub fn generate_wasm(&mut self, prog_name: String, prog: &Program) -> Vec<u8> {
        self.names.module(&prog_name);

        // 1) Types: ()->() en type 0
        self.types.ty().function([], []); // () -> ()
        self.ty_void = 0;

        // 2) Imports (fonctions + mémoire)
        // env.log(ptr,len) -> ()
        self.push_imported_function("env", "log", &[ValType::I32, ValType::I32], &[]);
        // str.to_str_i32(n) -> (ptr,len)
        self.push_imported_function(
            "str",
            "to_str_i32",
            &[ValType::I32],
            &[ValType::I32, ValType::I32],
        );
        // str.to_str_f64(n) -> (ptr,len)
        self.push_imported_function(
            "str",
            "to_str_f64",
            &[ValType::F64],
            &[ValType::I32, ValType::I32],
        );
        // str.concat(s1_ptr,s1_len,s2_ptr,s2_len) -> (ptr,len)
        self.push_imported_function(
            "str",
            "concat",
            &[ValType::I32, ValType::I32, ValType::I32, ValType::I32],
            &[ValType::I32, ValType::I32],
        );

        // Mémoire importée: env.memory
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

        // 3) Déclarations des fonctions (lib + programme + main)
        for f in &prog.functions {
            self.declare_function(f);
        }
        for f in &prog.main_program.functions {
            self.declare_function(f);
        }
        self.declare_function(&prog.main_program.main);

        // 4) Noms
        self.names.functions(&self.fn_names);

        // 5) Génération du code
        for f in &prog.functions {
            self.gen_function(f);
        }
        for f in &prog.main_program.functions {
            self.gen_function(f);
        }
        self.gen_function(&prog.main_program.main);

        // 6) Export de main (dernier index déclaré dans notre mapping)
        self.exports.export(
            "main",
            ExportKind::Func,
            self.fn_map.len().saturating_sub(1).try_into().unwrap(),
        );

        // 7) Global 'heap_ptr' exporté
        //
        //     - valeur initiale = fin de la zone de données (alignée à 16)
        //     - mutable: l'hôte Wasmi mettra à jour ce pointeur (bump allocator)
        //
        let heap_start = align_up(self.data_idx, 16);
        self.globals.global(
            GlobalType {
                val_type: ValType::I32,
                mutable: true,
                shared: false,
            },
            &ConstExpr::i32_const(heap_start as i32),
        );
        // C’est le premier (et unique) global => index 0.
        self.exports.export("heap_ptr", ExportKind::Global, 0);

        // 8) Module final
        let mut module = Module::new();
        module.section(&self.types);
        module.section(&self.imports);
        module.section(&self.functions);
        module.section(&self.globals);
        module.section(&self.exports);
        module.section(&self.code);
        module.section(&self.data);
        module.section(&self.names);
        module.finish()
    }
}
