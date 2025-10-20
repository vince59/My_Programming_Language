use crate::parser::{
    BinOp, Expr, Function as ParserFunction, NumExpr, ParseError, Program, Stadment, StrExpr,
    Variable,
};

use wasm_encoder::{
    CodeSection, ConstExpr, DataSection, EntityType, ExportKind, ExportSection, FunctionSection,
    GlobalSection, GlobalType, ImportSection, IndirectNameMap, MemoryType, Module, NameMap,
    NameSection, TypeSection, ValType,
};

use std::collections::HashMap;

#[derive(Clone, Copy)]
struct Blob {
    ptr: u32,
    len: u32,
}

#[inline]
fn align_up(x: u32, align: u32) -> u32 {
    // Align `x` up to the next multiple of `align` (assumes align >= 1, power-of-two in practice).
    if align <= 1 {
        return x;
    }
    let a = align - 1;
    (x + a) & !a
}

/// Insert `text` into the DataSection if needed, returning its (ptr,len).
/// - De-duplicates via `interner` (text -> Blob).
/// - Respects `align`. If an existing entry for the same text does not meet the requested
///   alignment (`ptr % align != 0`), a new copy is emitted at a properly aligned address.
pub fn push_text(
    data: &mut DataSection,
    mem_index: u32,
    cursor: &mut u32,
    text: &str,
    align: u32,
    interner: &mut HashMap<String, Blob>,
) -> Blob {
    // If we already have this text and it satisfies the requested alignment, reuse it.
    if let Some(&blob) = interner.get(text) {
        if align <= 1 || blob.ptr % align == 0 {
            return blob;
        }
        // Otherwise, fall through and allocate a new, stricter-aligned copy.
    }

    // Allocate a new slice with proper alignment.
    *cursor = align_up(*cursor, align.max(1));
    let ptr = *cursor;
    let bytes = text.as_bytes();

    // Emit active data segment at `ptr`.
    data.active(
        mem_index,
        &ConstExpr::i32_const(ptr as i32),
        bytes.iter().copied(),
    );

    // Advance the cursor.
    *cursor += bytes.len() as u32;

    let blob = Blob {
        ptr,
        len: bytes.len() as u32,
    };

    // Update the best-known location for this text (now satisfies `align`).
    interner.insert(text.to_owned(), blob);

    blob
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
    string_interner: HashMap<String, Blob>, // Maps string literals to their memory locations (prevents duplicates).

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
            string_interner: HashMap::new(),
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
    fn infer_type(&self, e: &NumExpr) -> Ty {
        match e {
            NumExpr::Int(_) => Ty::I32,
            NumExpr::Float(_) => Ty::F64,
            NumExpr::Binary { left, right, .. } => {
                let lt = self.infer_type(left);
                let rt = self.infer_type(right);
                if lt == Ty::F64 || rt == Ty::F64 {
                    Ty::F64
                } else {
                    Ty::I32
                }
            }
            NumExpr::Var { var, pos } => var.ty,
            NumExpr::Neg(inner) => self.infer_type(inner),
        }
    }

    // Emit `expr` as `target` type, inserting implicit casts as needed.
    // Allowed: i32 -> f64 (widen) and f64 -> i32 (narrow via trunc toward zero).
    fn gen_expression_as(
        &mut self,
        expr: &NumExpr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
        target: Ty,
        function: &ParserFunction,
    ) -> Result<(), ParseError> {
        match expr {
            NumExpr::Neg(inner) => {
                // Generate the inner value as the target type, then negate.
                match target {
                    Ty::F64 => {
                        // f64: direct unary neg instruction
                        self.gen_expression_as(inner, instr, Ty::F64, function)?;
                        instr.f64_neg(); // stack: [-inner]
                    }
                    Ty::I32 => {
                        // i32: there is no i32.neg; compute 0 - x
                        instr.i32_const(0); // stack: [0]
                        self.gen_expression_as(inner, instr, Ty::I32, function)?;
                        instr.i32_sub(); // stack: [0 - x]
                    }
                }
                return Ok(());
            }
            NumExpr::Int(i) => {
                instr.i32_const(*i);
                if target == Ty::F64 {
                    // signed i32 -> f64
                    instr.f64_convert_i32_s();
                }
                Ok(())
            }
            NumExpr::Float(r) => {
                match target {
                    Ty::F64 => {
                        instr.f64_const((*r).into());
                        Ok(())
                    }
                    Ty::I32 => {
                        // f64 -> i32 (trunc toward zero, traps on NaN or out-of-range)
                        instr.f64_const((*r).into());
                        instr.i32_trunc_f64_s();
                        Ok(())
                    }
                }
            }
            NumExpr::Binary { op, left, right } => {
                let target_ty = target;
                // make both operands the same target type
                self.gen_expression_as(left, instr, target_ty, function)?;
                self.gen_expression_as(right, instr, target_ty, function)?;

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
                Ok(())
            }
            NumExpr::Var { var, pos } => {
                let idx = match crate::parser::find_variable_index(&function.variables, &var.name) {
                    Some(i) => i as u32,
                    None => {
                        return Err(ParseError::Generator {
                            pos: pos.clone(),
                            msg: format!("unknown variable '{}'", var.name),
                        });
                    }
                };
                match var.ty {
                    Ty::I32 => {
                        instr.local_get(idx);
                        if target == Ty::F64 {
                            instr.f64_convert_i32_s();
                        }
                    }
                    Ty::F64 => {
                        instr.local_get(idx);
                        if target == Ty::I32 {
                            instr.i32_trunc_f64_s();
                        }
                    }
                }
                Ok(())
            }
        }
    }

    // Public entry: generate code and return the resulting type.
    pub fn gen_expression(
        &mut self,
        expr: &NumExpr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
        function: &ParserFunction,
    ) -> Result<Ty, ParseError> {
        let target = self.infer_type(expr);
        self.gen_expression_as(expr, instr, target, function)?;
        Ok(target)
    }

    // String expressions (literal -> Blob; conversion/concat -> values already on stack)
    fn gen_str_expression(
        &mut self,
        expr: &StrExpr,
        instr: &mut wasm_encoder::InstructionSink<'_>,
        function: &ParserFunction,
    ) -> Result<Option<Blob>, ParseError> {
        match expr {
            StrExpr::Str(s) => {
                // push string literal into data section
                let blob = push_text(
                    &mut self.data,
                    0,
                    &mut self.data_idx,
                    s,
                    16,
                    &mut self.string_interner,
                );
                Ok(Some(blob))
            }
            StrExpr::Nl => {
                let blob = push_text(
                    &mut self.data,
                    0,
                    &mut self.data_idx,
                    "\n",
                    16,
                    &mut self.string_interner,
                );
                Ok(Some(blob))
            }
            StrExpr::NumToStr(inner) => {
                let inner = &**inner;
                match self.gen_expression(inner, instr, function)? {
                    // push n
                    Ty::I32 => {
                        instr.call(self.fn_map["to_str_i32"] as u32); // (i32)->(i32,i32): [ptr,len]
                    }
                    Ty::F64 => {
                        instr.call(self.fn_map["to_str_f64"] as u32); // (f64)->(i32,i32): [ptr,len]
                    }
                }
                Ok(None)
            }
        }
    }

    // print([...]) -> build (ptr,len) then call env.log(ptr,len)
    pub fn gen_print(
        &mut self,
        str_expr: &Vec<StrExpr>,
        instr: &mut wasm_encoder::InstructionSink<'_>,
        function: &ParserFunction,
        nl: bool,
    ) -> Result<(), ParseError> {
        match str_expr.as_slice() {
            [] => {}
            [only] => {
                if let Some(blob) = self.gen_str_expression(only, instr, function)? {
                    instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32);
                }
            }
            [first, rest @ ..] => {
                if let Some(blob) = self.gen_str_expression(first, instr, function)? {
                    instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32);
                }
                for e in rest {
                    if let Some(blob) = self.gen_str_expression(e, instr, function)? {
                        instr.i32_const(blob.ptr as i32).i32_const(blob.len as i32);
                    }
                    // stack: ... s1_ptr s1_len s2_ptr s2_len -> concat -> s_ptr s_len
                    instr.call(self.fn_map["concat"] as u32);
                }
            }
        }
        if nl {
            // append newline
            let nl_blob = push_text(
                &mut self.data,
                0,
                &mut self.data_idx,
                "\n",
                16,
                &mut self.string_interner,
            );
            instr
                .i32_const(nl_blob.ptr as i32)
                .i32_const(nl_blob.len as i32);
            // stack: ... s_ptr s_len nl_ptr nl_len -> concat -> s_ptr s_len
            instr.call(self.fn_map["concat"] as u32);
        }
        instr.call(self.fn_map["log"] as u32);
        Ok(())
    }

    pub fn gen_variables(
        &mut self,
        variables: &[Variable],
        fn_id: u32,
        param_count: u32, // <- passe 0 si pas de paramètres
    ) -> Vec<(u32, ValType)> {
        // 1) Prépare la map de noms pour cette fonction
        let mut fn_locals = NameMap::new();

        // 2) Construit la liste des locals (en groupes (count, type))
        let mut locals: Vec<(u32, ValType)> = Vec::with_capacity(variables.len());

        // index logique des locals dans la fonction = params d'abord, puis locals
        let mut local_index = param_count;

        for var in variables {
            let val_ty = match var.ty {
                Ty::I32 => ValType::I32,
                Ty::F64 => ValType::F64,
            };
            locals.push((1, val_ty));
            fn_locals.append(local_index, &var.name);
            local_index += 1;
        }

        // 3) Ajoute ces noms à la NameSection
        let mut local_names = IndirectNameMap::new();
        local_names.append(fn_id, &fn_locals);
        self.names.locals(&local_names);

        locals
    }

    pub fn gen_function(&mut self, function: &ParserFunction) -> Result<(), ParseError> {
        self.functions.function(self.ty_void); // () -> ()

        let fn_id = self.fn_map[&function.name] as u32;

        // si ta fonction n'a pas de paramètres :
        let param_count = 0;

        let locals = self.gen_variables(&function.variables, fn_id, param_count);

        let mut fnc = wasm_encoder::Function::new(locals);
        let mut instr = fnc.instructions();

        for stdm in &function.body {
            match stdm {
                Stadment::Print(str_expr) => self.gen_print(str_expr, &mut instr, function, false)?,
                Stadment::Println(str_expr) => self.gen_print(str_expr, &mut instr, function, true)?,
                Stadment::Call { name, pos } => {
                    if let Some(fid) = self.fn_map.get(name) {
                        instr.call(*fid as u32);
                    } else {
                        return Err(ParseError::Generator {
                            pos: pos.clone(),
                            msg: format!("unknown function '{}'", name),
                        });
                    }
                }
                Stadment::Assignment { var, expr, pos } => {
                    // generate expression
                    match &expr {
                        Expr::Num(num_expr) => {
                            self.gen_expression_as(&num_expr, &mut instr, var.ty, function)?;
                        }
                        _ => {
                            return Err(ParseError::Generator {
                                pos: pos.clone(),
                                msg: "only numeric expressions are supported in assignments"
                                    .to_string(),
                            });
                        }
                    }

                    // find local index
                    let idx =
                        match crate::parser::find_variable_index(&function.variables, &var.name) {
                            Some(i) => i as u32,
                            None => {
                                return Err(ParseError::Generator {
                                    pos: pos.clone(),
                                    msg: format!("unknown variable '{}'", var.name),
                                });
                            }
                        };
                    // set local
                    instr.local_set(idx);
                }
            }
        }

        instr.end();
        self.code.function(&fnc);
        Ok(())
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

    pub fn generate_wasm(
        &mut self,
        prog_name: String,
        prog: &Program,
    ) -> Result<Vec<u8>, ParseError> {
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
            self.gen_function(&f)?;
        }
        for f in &prog.main_program.functions {
            self.gen_function(&f)?;
        }
        self.gen_function(&prog.main_program.main)?;

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
        Ok(module.finish())
    }
}
