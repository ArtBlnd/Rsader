use std::pin::Pin;

use super::{
    action::{ActionExecutor, TaskContext},
    context::VmContext,
    value::{IntegerValue, Value},
    VmError,
};
use crate::{
    ast::{
        BinaryArithmeticOp, BinaryBitwiseOp, BinaryComparisonOp, BinaryLogicalOp, BinaryOp, Expr,
        Ident, Literal, Ty,
    },
    ir::ty::{CompositeTypes, IrType, PrimitiveTypes},
    vm::action::AsyncFunctionCallAction,
    window::console::ConsolePipe,
};

use futures::Future;
use unwrap_let::unwrap_let;

pub struct Evaluate<'input> {
    pub expr: Expr<'input>,
    pub task_context: TaskContext,
}

#[cfg(not(target_arch = "wasm32"))]
type BoxFuture<'input> = Pin<Box<dyn Future<Output = Result<Value, VmError>> + Send + 'input>>;

#[cfg(any(target_arch = "wasm32"))]
type BoxFuture<'input> = Pin<Box<dyn Future<Output = Result<Value, VmError>> + 'input>>;

impl<'input> Evaluate<'input> {
    pub async fn eval(&self, context: &mut VmContext) -> Result<Value, VmError> {
        self.eval_expression(context, &self.expr).await
    }

    fn eval_expression(
        &'input self,
        context: &'input mut VmContext,
        expr: &'input Expr<'input>,
    ) -> BoxFuture<'input> {
        Box::pin(async move {
            match &expr {
                Expr::Async(async_expr) => self.eval_async(context, async_expr).await,
                Expr::Await(await_expr) => self.eval_await(context, await_expr).await,
                Expr::BinOp { op, lhs, rhs } => self.eval_bin_op(context, op, lhs, rhs).await,
                Expr::Identifier { name } => self.eval_identifier_value(context, name).await,
                Expr::Call { expr, args0, args1 } => {
                    self.eval_call(context, expr, args0, args1).await
                }
                Expr::Tuple { items } => self.eval_tuple(context, items).await,
                Expr::Literal(literal) => self.eval_literal(literal).await,
                _ => unimplemented!(),
            }
        })
    }

    async fn eval_async(
        &self,
        context: &mut VmContext,
        expr: &Expr<'input>,
    ) -> Result<Value, VmError> {
        let Expr::Call { expr, args0, args1 } = expr else {
            return Err(VmError::ExpectedFunctionCall);
        };

        let value = self.eval_expression(context, expr).await?;
        let function_ty = value.ty();
        let Value::Function(function) = value else {
            return Err(VmError::ExpectedFunction(function_ty));
        };

        unwrap_let!(IrType::Composite(CompositeTypes::Function { ret, args, .. }) = function_ty);

        let mut args0_val = Vec::new();
        let mut args1_val = Vec::new();

        for arg in args0 {
            let value = self.eval_identifier_type(context, arg).await?;
            args0_val.push(value);
        }

        for (arg, ty) in args1.iter().zip(args) {
            let value = self.eval_expression(context, arg).await?;
            if value.ty() != ty {
                return Err(VmError::TypeMismatch(ty, value.ty()));
            }

            args1_val.push(value);
        }

        let future = ActionExecutor::new_with_type(
            AsyncFunctionCallAction {
                function,
                args0: Some(args0_val),
                args1: Some(args1_val),
                context: context.new_scope(vec![]),
            },
            ret.as_ref().clone(),
        )
        .spawn_rt(context.global_ctx(), ConsolePipe::instance());

        let uuid = future.uuid();
        let ty = future.ty().clone();
        context.global_ctx().insert_action(future);
        Ok(Value::Future(uuid, ty))
    }

    async fn eval_await(
        &self,
        context: &mut VmContext,
        expr: &Expr<'input>,
    ) -> Result<Value, VmError> {
        let value = self.eval_expression(context, expr).await?;

        if let Value::Future(token, _) = value {
            let future = context.global_ctx().remove_action(token);
            return Ok(future.join().await.unwrap());
        }

        Err(VmError::ExpectedFuture(value.ty()))
    }

    async fn eval_bin_op(
        &self,
        context: &mut VmContext,
        op: &BinaryOp,
        lhs: &Expr<'input>,
        rhs: &Expr<'input>,
    ) -> Result<Value, VmError> {
        match *op {
            BinaryOp::Assign => self.eval_bin_op_assign(context, lhs, rhs).await,
            BinaryOp::Arithmetic(op) => self.eval_bin_op_arithmetic(context, op, lhs, rhs).await,
            BinaryOp::Bitwise(op) => self.eval_bin_op_bitwise(context, op, lhs, rhs).await,
            BinaryOp::Logical(op) => self.eval_bin_op_logical(context, op, lhs, rhs).await,
            BinaryOp::Comparison(op) => self.eval_bin_op_comparison(context, op, lhs, rhs).await,
        }
    }

    async fn eval_bin_op_assign(
        &self,
        context: &mut VmContext,
        lhs: &Expr<'input>,
        rhs: &Expr<'input>,
    ) -> Result<Value, VmError> {
        let lhs = self.eval_expression(context, lhs).await?;
        let lhs_ty = lhs.ty();
        let rhs = self.eval_expression(context, rhs).await?;
        let rhs_ty = rhs.ty();

        if lhs_ty != rhs_ty {
            return Err(VmError::TypeMismatch(lhs_ty, rhs_ty));
        }

        let lvalue = lhs.as_variable().ok_or(VmError::InvalidAssignment)?;
        let rvalue = rhs;

        context.set_variable(lvalue, rvalue);
        Ok(Value::Tuple(vec![]))
    }

    async fn eval_bin_op_arithmetic(
        &self,
        context: &mut VmContext,
        op: BinaryArithmeticOp,
        lhs: &Expr<'input>,
        rhs: &Expr<'input>,
    ) -> Result<Value, VmError> {
        let lhs = self.eval_expression(context, lhs).await?;
        let rhs = self.eval_expression(context, rhs).await?;

        if lhs.ty() != IrType::Primitive(PrimitiveTypes::Decimal)
            || rhs.ty() != IrType::Primitive(PrimitiveTypes::Decimal)
        {
            return Err(VmError::TypeMismatch(
                IrType::Primitive(PrimitiveTypes::Decimal),
                lhs.ty(),
            ));
        }

        let lhs = lhs.as_decimal().unwrap();
        let rhs = rhs.as_decimal().unwrap();

        Ok(match op {
            BinaryArithmeticOp::Add => Value::Decimal(lhs + rhs),
            BinaryArithmeticOp::Sub => Value::Decimal(lhs - rhs),
            BinaryArithmeticOp::Mul => Value::Decimal(lhs * rhs),
            BinaryArithmeticOp::Div => Value::Decimal(lhs / rhs),
        })
    }

    async fn eval_bin_op_bitwise(
        &self,
        context: &mut VmContext,
        op: BinaryBitwiseOp,
        lhs: &Expr<'input>,
        rhs: &Expr<'input>,
    ) -> Result<Value, VmError> {
        let lhs = self.eval_expression(context, lhs).await?;
        let rhs = self.eval_expression(context, rhs).await?;

        if lhs.ty() != IrType::Primitive(PrimitiveTypes::Decimal)
            || rhs.ty() != IrType::Primitive(PrimitiveTypes::Decimal)
        {
            return Err(VmError::TypeMismatch(
                IrType::Primitive(PrimitiveTypes::Decimal),
                lhs.ty(),
            ));
        }

        if let (Value::Boolean(lhs), Value::Boolean(rhs)) = (&lhs, &rhs) {
            return Ok(match op {
                BinaryBitwiseOp::And => Value::Boolean(lhs & rhs),
                BinaryBitwiseOp::Or => Value::Boolean(lhs | rhs),
            });
        }

        Err(VmError::InvalidOperation(
            BinaryOp::Bitwise(op),
            lhs.ty(),
            rhs.ty(),
        ))
    }

    async fn eval_bin_op_logical(
        &self,
        context: &mut VmContext,
        op: BinaryLogicalOp,
        lhs: &Expr<'input>,
        rhs: &Expr<'input>,
    ) -> Result<Value, VmError> {
        match op {
            BinaryLogicalOp::And => {
                let lhs = self.eval_expression(context, lhs).await?;
                if !lhs.as_boolean().unwrap() {
                    return Ok(Value::Boolean(false));
                }

                let rhs = self.eval_expression(context, rhs).await?;
                Ok(Value::Boolean(rhs.as_boolean().unwrap()))
            }
            BinaryLogicalOp::Or => {
                let lhs = self.eval_expression(context, lhs).await?;
                if lhs.as_boolean().unwrap() {
                    return Ok(Value::Boolean(true));
                }

                let rhs = self.eval_expression(context, rhs).await?;
                Ok(Value::Boolean(rhs.as_boolean().unwrap()))
            }
        }
    }

    async fn eval_bin_op_comparison(
        &self,
        context: &mut VmContext,
        op: BinaryComparisonOp,
        lhs: &Expr<'input>,
        rhs: &Expr<'input>,
    ) -> Result<Value, VmError> {
        let lhs = self.eval_expression(context, lhs).await?;
        let rhs = self.eval_expression(context, rhs).await?;

        if lhs.ty() != rhs.ty() {
            return Err(VmError::TypeMismatch(lhs.ty(), rhs.ty()));
        }

        if let (Value::Decimal(lhs), Value::Decimal(rhs)) = (&lhs, &rhs) {
            return Ok(match op {
                BinaryComparisonOp::Eq => Value::Boolean(lhs == rhs),
                BinaryComparisonOp::Ne => Value::Boolean(lhs != rhs),
                BinaryComparisonOp::Lt => Value::Boolean(lhs < rhs),
                BinaryComparisonOp::Le => Value::Boolean(lhs <= rhs),
                BinaryComparisonOp::Gt => Value::Boolean(lhs > rhs),
                BinaryComparisonOp::Ge => Value::Boolean(lhs >= rhs),
            });
        };

        if let (Value::Boolean(lhs), Value::Boolean(rhs)) = (&lhs, &rhs) {
            return Ok(match op {
                BinaryComparisonOp::Eq => Value::Boolean(lhs == rhs),
                BinaryComparisonOp::Ne => Value::Boolean(lhs != rhs),
                BinaryComparisonOp::Lt => Value::Boolean(lhs < rhs),
                BinaryComparisonOp::Le => Value::Boolean(lhs <= rhs),
                BinaryComparisonOp::Gt => Value::Boolean(lhs > rhs),
                BinaryComparisonOp::Ge => Value::Boolean(lhs >= rhs),
            });
        };

        Err(VmError::InvalidOperation(
            BinaryOp::Comparison(op),
            lhs.ty(),
            rhs.ty(),
        ))
    }

    async fn eval_identifier_value(
        &self,
        context: &mut VmContext,
        Ident(name): &Ident<'input>,
    ) -> Result<Value, VmError> {
        if let Some(value) = context
            .get_variable(name)
            .ok_or_else(|| VmError::UndefinedVariable(name.to_string()))?
            .try_clone()
        {
            return Ok(value);
        }

        Ok(context.take_variable(name).unwrap())
    }

    async fn eval_identifier_type(
        &self,
        _context: &mut VmContext,
        Ty(_name): &Ty<'input>,
    ) -> Result<IrType, VmError> {
        unimplemented!()
    }

    async fn eval_call<'ctx>(
        &self,
        context: &mut VmContext,
        expr: &Expr<'input>,
        args0: &[Ty<'input>],
        args1: &[Expr<'input>],
    ) -> Result<Value, VmError> {
        let value = self.eval_expression(context, expr).await?;
        let function_ty = value.ty();
        let function = value
            .as_function()
            .ok_or(VmError::ExpectedFunction(function_ty.clone()))?;

        unwrap_let!(IrType::Composite(CompositeTypes::Function { args, ret, .. }) = &function_ty);

        let mut args0_val = Vec::new();
        let mut args1_val = Vec::new();

        for arg in args0 {
            let value = self.eval_identifier_type(context, arg).await?;
            args0_val.push(value);
        }

        for (arg, ty) in args1.iter().zip(args) {
            let value = self.eval_expression(context, arg).await?;
            if value.ty() != *ty {
                return Err(VmError::TypeMismatch(ty.clone(), value.ty()));
            }

            args1_val.push(value);
        }

        function
            .call(&self.task_context, args0_val, args1_val, context)
            .await
    }

    async fn eval_tuple(
        &self,
        context: &mut VmContext,
        items: &[Expr<'input>],
    ) -> Result<Value, VmError> {
        let mut values = Vec::new();
        for item in items {
            let value = self.eval_expression(context, item).await?;
            values.push(value);
        }

        Ok(Value::Tuple(values))
    }

    async fn eval_literal(&self, literal: &Literal) -> Result<Value, VmError> {
        Ok(match literal {
            Literal::String(literal) => Value::Str(literal.clone()),
            &Literal::Boolean(literal) => Value::Boolean(literal),
            &Literal::Decimal(literal) => Value::Decimal(literal),
            &Literal::Integer(literal) => Value::Integer(IntegerValue::U128(literal)),
            &Literal::Currency(literal) => Value::Currency(literal),
            &Literal::CurrencyPair((cur1, cur2)) => {
                Value::Tuple(vec![Value::Currency(cur1), Value::Currency(cur2)])
            }
        })
    }
}
