use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use rust_decimal::Decimal;

use super::{action::ActionToken, builtin_types, function::Function};
use crate::{
    currency::Currency,
    exchange::Unit,
    ir::ty::{CompositeTypes, IrType, PrimitiveTypes},
};

pub enum Value {
    Function(Function),
    Tuple(Vec<Value>),
    Struct(HashMap<String, Value>, IrType),
    Enum(String, EnumValue, IrType),
    Variable(String, IrType),
    Integer(IntegerValue),
    Str(String),
    Decimal(Decimal),
    Currency(Currency),
    Boolean(bool),
    Future(uuid::Uuid, IrType),
}

impl Value {
    pub fn ty(&self) -> IrType {
        match self {
            Value::Function(f) => f.ty(),
            Value::Struct(_, ty) => ty.clone(),
            Value::Enum(_, _, ty) => ty.clone(),
            Value::Variable(_, ty) => ty.clone(),
            Value::Integer(integer) => match integer {
                IntegerValue::U8(_) => IrType::Primitive(PrimitiveTypes::U8),
                IntegerValue::U16(_) => IrType::Primitive(PrimitiveTypes::U16),
                IntegerValue::U32(_) => IrType::Primitive(PrimitiveTypes::U32),
                IntegerValue::U64(_) => IrType::Primitive(PrimitiveTypes::U64),
                IntegerValue::U128(_) => IrType::Primitive(PrimitiveTypes::U128),
                IntegerValue::I8(_) => IrType::Primitive(PrimitiveTypes::I8),
                IntegerValue::I16(_) => IrType::Primitive(PrimitiveTypes::I16),
                IntegerValue::I32(_) => IrType::Primitive(PrimitiveTypes::I32),
                IntegerValue::I64(_) => IrType::Primitive(PrimitiveTypes::I64),
                IntegerValue::I128(_) => IrType::Primitive(PrimitiveTypes::I128),
            },
            Value::Tuple(values) => IrType::Composite(CompositeTypes::Tuple(
                values.iter().map(|v| v.ty()).collect(),
            )),
            Value::Str(_) => IrType::Primitive(PrimitiveTypes::String),
            Value::Decimal(_) => IrType::Primitive(PrimitiveTypes::Decimal),
            Value::Currency(_) => IrType::Primitive(PrimitiveTypes::Currency),
            Value::Boolean(_) => IrType::Primitive(PrimitiveTypes::Boolean),
            Value::Future(_, ty) => ty.clone(),
        }
    }

    #[async_recursion::async_recursion]
    pub async fn eval_and_display(&self) -> String {
        match self {
            Value::Function(f) => f.display_name(),
            Value::Struct(fields, _) => {
                let mut result = String::from("{");
                for (i, (name, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }

                    result.push_str(&name);
                    result.push_str(": ");
                    result.push_str(&value.eval_and_display().await);
                }
                result.push('}');
                result
            }
            Value::Enum(variant_name, value, _) => {
                let mut result = String::from(variant_name);
                match value {
                    EnumValue::Unit => {}
                    EnumValue::Tuple(values) => {
                        result.push('(');
                        for (i, value) in values.iter().enumerate() {
                            if i > 0 {
                                result.push_str(", ");
                            }

                            result.push_str(&value.eval_and_display().await);
                        }
                        result.push(')');
                    }
                    EnumValue::Struct(fields) => {
                        result.push('{');
                        for (i, (name, value)) in fields.iter().enumerate() {
                            if i > 0 {
                                result.push_str(", ");
                            }

                            result.push_str(&name);
                            result.push_str(": ");
                            result.push_str(&value.eval_and_display().await);
                        }
                        result.push('}');
                    }
                }
                result
            }
            Value::Variable(name, _) => name.clone(),
            Value::Integer(integer) => match integer {
                IntegerValue::U8(v) => v.to_string(),
                IntegerValue::U16(v) => v.to_string(),
                IntegerValue::U32(v) => v.to_string(),
                IntegerValue::U64(v) => v.to_string(),
                IntegerValue::U128(v) => v.to_string(),
                IntegerValue::I8(v) => v.to_string(),
                IntegerValue::I16(v) => v.to_string(),
                IntegerValue::I32(v) => v.to_string(),
                IntegerValue::I64(v) => v.to_string(),
                IntegerValue::I128(v) => v.to_string(),
            },
            Value::Str(s) => s.clone(),
            Value::Decimal(d) => d.to_string(),
            Value::Currency(c) => c.to_string(),
            Value::Tuple(values) => {
                let mut result = String::from("(");
                for (i, value) in values.iter().enumerate() {
                    if i > 0 {
                        result.push_str(", ");
                    }

                    result.push_str(&value.eval_and_display().await);
                }
                result.push(')');
                result
            }
            Value::Boolean(b) => b.to_string(),
            Value::Future(uuid, ty) => format!("Future<{ty}>({uuid})"),
        }
    }

    pub fn is_clonable(&self) -> bool {
        match self {
            Value::Function(_)
            | Value::Struct(_, _)
            | Value::Enum(_, _, _)
            | Value::Variable(_, _)
            | Value::Integer(_)
            | Value::Str(_)
            | Value::Decimal(_)
            | Value::Currency(_)
            | Value::Tuple(_)
            | Value::Boolean(_) => true,
            Value::Future(_, _) => false,
        }
    }

    pub fn try_clone(&self) -> Option<Value> {
        if !self.is_clonable() {
            return None;
        }

        match self {
            Value::Function(f) => Some(Value::Function(f.clone())),
            Value::Tuple(values) => {
                let values: Vec<Value> = values.iter().map(|v| v.try_clone().unwrap()).collect();
                Some(Value::Tuple(values))
            }
            Value::Struct(fields, ty) => {
                let fields = fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.try_clone().unwrap()))
                    .collect();

                Some(Value::Struct(fields, ty.clone()))
            }
            Value::Enum(variant_name, value, ty) => {
                let value = match value {
                    EnumValue::Unit => EnumValue::Unit,
                    EnumValue::Tuple(values) => {
                        let values = values.iter().map(|v| v.try_clone().unwrap()).collect();
                        EnumValue::Tuple(values)
                    }
                    EnumValue::Struct(fields) => {
                        let fields = fields
                            .iter()
                            .map(|(name, value)| (name.clone(), value.try_clone().unwrap()))
                            .collect();
                        EnumValue::Struct(fields)
                    }
                };
                Some(Value::Enum(variant_name.clone(), value, ty.clone()))
            }
            Value::Variable(name, ty) => Some(Value::Variable(name.clone(), ty.clone())),
            Value::Integer(integer) => Some(Value::Integer(integer.clone())),
            Value::Str(s) => Some(Value::Str(s.clone())),
            Value::Decimal(d) => Some(Value::Decimal(*d)),
            Value::Currency(c) => Some(Value::Currency(*c)),

            Value::Boolean(b) => Some(Value::Boolean(*b)),
            Value::Future(_, _) => None,
        }
    }

    pub fn as_function(&self) -> Option<&Function> {
        match self {
            Value::Function(f) => Some(f),
            _ => None,
        }
    }

    pub fn as_variable(&self) -> Option<&str> {
        match self {
            Value::Variable(name, _) => Some(name),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Value::Decimal(d) => Some(*d),
            _ => None,
        }
    }

    pub fn as_currency(&self) -> Option<Currency> {
        match self {
            Value::Currency(c) => Some(*c),
            _ => None,
        }
    }

    pub fn as_tuple(&self) -> Option<&[Value]> {
        match self {
            Value::Tuple(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

// #[derive(Clone)]
// pub struct VariableRef(Arc<RwLock<Value>>);
// impl VariableRef {
//     pub async fn lock(&self) -> VariableRefLocked {
//         VariableRefLocked(self.0.clone().read_owned().await)
//     }
// }

// #[derive(Clone)]
// pub struct VariableMut(Arc<RwLock<Value>>);
// impl VariableMut {
//     pub async fn lock(&self) -> VariableMutLocked {
//         VariableMutLocked(self.0.clone().write_owned().await)
//     }
// }

// pub struct VariableRefLocked(OwnedRwLockReadGuard<Value>);
// impl Deref for VariableRefLocked {
//     type Target = Value;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// pub struct VariableMutLocked(OwnedRwLockWriteGuard<Value>);
// impl Deref for VariableMutLocked {
//     type Target = Value;

//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }

// impl DerefMut for VariableMutLocked {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.0
//     }
// }

pub enum EnumValue {
    Unit,
    Tuple(Vec<Value>),
    Struct(HashMap<String, Value>),
}

#[derive(Debug, Clone)]
pub enum IntegerValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
}

pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl IntoValue for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValue for () {
    fn into_value(self) -> Value {
        Value::Tuple(vec![])
    }
}

impl IntoValue for Decimal {
    fn into_value(self) -> Value {
        Value::Decimal(self)
    }
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Boolean(self)
    }
}

impl<T> IntoValue for Option<T>
where
    T: IntoValue,
{
    fn into_value(self) -> Value {
        match self {
            Some(value) => Value::Enum(
                "Some".to_string(),
                EnumValue::Tuple(vec![value.into_value()]),
                builtin_types::option(),
            ),
            None => Value::Enum("None".to_string(), EnumValue::Unit, builtin_types::option()),
        }
    }
}

impl IntoValue for Unit {
    fn into_value(self) -> Value {
        Value::Struct(
            [
                ("amount".to_string(), self.amount.into_value()),
                ("price".to_string(), self.price.into_value()),
            ]
            .try_into()
            .unwrap(),
            builtin_types::unit(),
        )
    }
}
