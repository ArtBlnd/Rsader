use crate::ast::{Expr, Ident, Item, Let, NameAndTy, Stmt, Ty};

use super::{
    arena::IrArena,
    basic_block::BasicBlock,
    function::IrFunction,
    instruction::{IrBinaryOp, IrInstr},
    scope::Scope,
    ty::{IrType, PrimitiveTypes},
    value::{IrValue, IrVariable},
    Ir,
};

#[derive(thiserror::Error, Debug, PartialEq, Eq, Clone, Hash)]
pub enum IrLifterError {
    #[error("cannot find declaration `{0}` in this scope")]
    CannotFindDeclaration(String),
}

/// The IrLifter is responsible for lifting the AST into the IR.
pub struct IrLifter<'ir> {
    arena: &'ir IrArena<'ir>,

    ir: Ir<'ir>,
    scope: Option<Scope<'ir>>,
    current_fn: Option<&'ir mut IrFunction<'ir>>,
    current_bb: Option<&'ir mut BasicBlock<'ir>>,

    // Pre-allocated types
    ty_unknown: &'ir IrType,
    ty_void: &'ir IrType,
    ty_bool: &'ir IrType,
    ty_string: &'ir IrType,
    ty_currency: &'ir IrType,
    ty_u8: &'ir IrType,
    ty_u16: &'ir IrType,
    ty_u32: &'ir IrType,
    ty_u64: &'ir IrType,
    ty_u128: &'ir IrType,
    ty_i8: &'ir IrType,
    ty_i16: &'ir IrType,
    ty_i32: &'ir IrType,
    ty_i64: &'ir IrType,
    ty_i128: &'ir IrType,

    errors: Vec<((usize, usize), IrLifterError)>,
}

impl<'ir> IrLifter<'ir> {
    pub fn new(arena: &'ir IrArena<'ir>, ir: Ir<'ir>) -> Self {
        Self {
            arena,

            ir,
            scope: Some(Scope::new_scope(None)),
            current_fn: None,
            current_bb: None,

            ty_unknown: arena.alloc_type(IrType::Unknown),
            ty_void: arena.alloc_type(IrType::void()),
            ty_bool: arena.alloc_type(IrType::boolean()),
            ty_string: arena.alloc_type(IrType::string()),
            ty_currency: arena.alloc_type(IrType::currency()),
            ty_u8: arena.alloc_type(IrType::u8()),
            ty_u16: arena.alloc_type(IrType::u16()),
            ty_u32: arena.alloc_type(IrType::u32()),
            ty_u64: arena.alloc_type(IrType::u64()),
            ty_u128: arena.alloc_type(IrType::u128()),
            ty_i8: arena.alloc_type(IrType::i8()),
            ty_i16: arena.alloc_type(IrType::i16()),
            ty_i32: arena.alloc_type(IrType::i32()),
            ty_i64: arena.alloc_type(IrType::i64()),
            ty_i128: arena.alloc_type(IrType::i128()),

            errors: Vec::new(),
        }
    }

    pub fn into_ir(self) -> Ir<'ir> {
        self.ir
    }

    pub fn visit_statement(&mut self, stmt: &Stmt<'_>) {
        match stmt {
            Stmt::Let(stmt) => self.visit_let(stmt),
            Stmt::Item(stmt) => self.visit_item(stmt),
            Stmt::Expr(expr) => {
                self.visit_expr(expr);
            }
            Stmt::Empty => {}
        }
    }

    /* ==============================================
    =           Scope Management Functions          =
    ===============================================*/
    fn new_scope(&mut self) {
        let scope = Scope::new_scope(self.scope.take());
        self.scope = Some(scope);
    }

    fn pop_scope(&mut self) {
        self.scope = self.scope.take().unwrap().pop_scope();
    }

    fn new_ty(&mut self, name: impl AsRef<str>, ty: IrType) {
        self.scope
            .as_mut()
            .unwrap()
            .new_type(name, &self.arena.alloc_type(ty));
    }

    fn ty<T>(&self, name: Option<T>) -> Option<&'ir IrType>
    where
        T: AsRef<str>,
    {
        let Some(name) = name else {
            return Some(self.ty_unknown);
        };

        match name.as_ref() {
            "()" => Some(self.ty_void),
            "bool" => Some(self.ty_bool),
            "string" => Some(self.ty_string),
            "currency" => Some(self.ty_currency),
            "u8" => Some(self.ty_u8),
            "u16" => Some(self.ty_u16),
            "u32" => Some(self.ty_u32),
            "u64" => Some(self.ty_u64),
            "u128" => Some(self.ty_u128),
            "i8" => Some(self.ty_i8),
            "i16" => Some(self.ty_i16),
            "i32" => Some(self.ty_i32),
            "i64" => Some(self.ty_i64),
            "i128" => Some(self.ty_i128),
            _ => self.scope.as_ref().unwrap().get_type(name),
        }
    }

    fn new_variable(&mut self, name: impl AsRef<str>, ty: &'ir IrType) -> IrVariable<'ir> {
        self.scope.as_mut().unwrap().new_variable(name, ty)
    }

    fn variable(&self, name: impl AsRef<str>) -> Option<IrVariable<'ir>> {
        self.scope.as_ref().unwrap().get_variable(name)
    }

    fn new_function(
        &mut self,
        name: impl AsRef<str>,
        function: IrFunction<'ir>,
    ) -> &'ir mut IrFunction<'ir> {
        todo!();
    }

    fn function(&self, name: impl AsRef<str>) -> Option<&'ir IrFunction<'ir>> {
        todo!();
    }

    fn new_bb(&mut self) -> &'ir mut BasicBlock<'ir> {
        todo!();
    }

    /* ==============================================
    =           Function Management Functions       =
    ===============================================*/
    fn emit_isntr(&mut self, instr: IrInstr<'ir>) {
        self.current_bb.as_mut().unwrap().push(instr);
    }

    /* ==============================================
    =            Visit Statement Functions          =
    ===============================================*/
    fn visit_let(&mut self, let_stmt: &Let<'_>) {
        let Let {
            name: Ident(name),
            ty,
            expr,
        } = let_stmt;

        let Some(ty) = self.ty(ty.as_ref().map(|Ty(ty_name)| ty_name)) else {
            todo!()
        };

        let var = self.new_variable(name, ty);
        let Some(expr) = expr.as_ref().map(|expr| self.visit_expr(expr)) else {
            return;
        };

        self.emit_isntr(IrInstr::Assign {
            dst: var,
            src: expr,
        });
    }

    fn visit_item(&mut self, item: &Item<'_>) {
        match item {
            Item::Function {
                name,
                args0,
                args1,
                body,
            } => {
                self.new_scope();
                self.visit_fn_item(name, args0, args1, body);
                self.pop_scope();
            }
            Item::Struct { name, fields } => todo!(),
            Item::Enum { name, variants } => todo!(),
        }
    }

    fn visit_fn_item(
        &mut self,
        Ident(name): &Ident<'_>,
        params0: &[Ty<'_>],
        params1: &[NameAndTy<'_>],
        body: &[Stmt<'_>],
    ) {
        let params0 = params0
            .iter()
            .map(|Ty(ty_name)| {
                let Some(ty) = self.ty(Some(ty_name)) else {
                    todo!()
                };

                ty
            })
            .collect::<Vec<_>>();

        let params1 = params1
            .iter()
            .map(
                |NameAndTy {
                     name: Ident(name),
                     ty: Ty(ty_name),
                 }| {
                    let Some(ty) = self.ty(Some(ty_name)) else {
                        todo!()
                    };

                    self.new_variable(name, ty)
                },
            )
            .collect::<Vec<_>>();

        let function = self.new_function(name, IrFunction::new(params0, params1, name));
        let old_fn = self.current_fn.take().unwrap();
        self.current_fn = Some(function);

        let bb = self.new_bb();
        let old_bb = self.current_bb.take().unwrap();
        self.current_bb = Some(bb);

        for stmt in body {
            self.visit_statement(stmt);
        }

        self.current_fn = Some(old_fn);
        self.current_bb = Some(old_bb);
    }

    /* ==============================================
    =            Visit Expression Functions         =
    ===============================================*/

    fn visit_expr(&mut self, expr: &Expr<'_>) -> IrValue<'ir> {
        todo!()
    }
}
