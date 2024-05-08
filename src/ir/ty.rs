use std::{fmt::Display, sync::Arc};

use rust_decimal::Decimal;

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum IrType {
    /// Represents an unknown type
    /// The type is not known yet, will be resolved later using bidirectional type checking.
    /// If the type is not resolved, it will be an error.
    Unknown,
    Alias(String, Box<IrType>),
    Const(Box<IrType>),
    Primitive(PrimitiveTypes),
    Reference(Box<IrType>),
    GenericType(String),
    Array(Box<IrType>, usize),
    Future(Box<IrType>),
    Composite(CompositeTypes),
}

impl IrType {
    pub fn u8() -> Self {
        IrType::Primitive(PrimitiveTypes::U8)
    }

    pub fn u16() -> Self {
        IrType::Primitive(PrimitiveTypes::U16)
    }

    pub fn u32() -> Self {
        IrType::Primitive(PrimitiveTypes::U32)
    }

    pub fn u64() -> Self {
        IrType::Primitive(PrimitiveTypes::U64)
    }

    pub fn u128() -> Self {
        IrType::Primitive(PrimitiveTypes::U128)
    }

    pub fn i8() -> Self {
        IrType::Primitive(PrimitiveTypes::I8)
    }

    pub fn i16() -> Self {
        IrType::Primitive(PrimitiveTypes::I16)
    }

    pub fn i32() -> Self {
        IrType::Primitive(PrimitiveTypes::I32)
    }

    pub fn i64() -> Self {
        IrType::Primitive(PrimitiveTypes::I64)
    }

    pub fn i128() -> Self {
        IrType::Primitive(PrimitiveTypes::I128)
    }

    pub fn decimal() -> Self {
        IrType::Primitive(PrimitiveTypes::Decimal)
    }

    pub fn string() -> Self {
        IrType::Primitive(PrimitiveTypes::String)
    }

    pub fn boolean() -> Self {
        IrType::Primitive(PrimitiveTypes::Boolean)
    }

    pub fn currency() -> Self {
        IrType::Primitive(PrimitiveTypes::Currency)
    }

    pub fn tuple(types: impl IntoIterator<Item = IrType>) -> Self {
        IrType::Composite(CompositeTypes::Tuple(types.into_iter().collect()))
    }

    pub const fn void() -> Self {
        IrType::Composite(CompositeTypes::Tuple(vec![]))
    }
}

impl Display for IrType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IrType::Unknown => write!(f, "unknown"),
            IrType::Alias(name, t) => write!(f, "{}(${})", name, t),
            IrType::Const(c) => write!(f, "const {}", c),
            IrType::Primitive(p) => write!(f, "{}", p),
            IrType::Reference(r) => write!(f, "&{}", r),
            IrType::Composite(c) => write!(f, "{}", c),
            IrType::Array(t, s) => write!(f, "[{}; {}]", t, s),
            IrType::Future(t) => write!(f, "Future<{}>", t),
            IrType::GenericType(g) => write!(f, "{:?}", g),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum PrimitiveTypes {
    String,
    Currency,
    Decimal,
    Boolean,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
}

impl Display for PrimitiveTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PrimitiveTypes::String => write!(f, "string"),
            PrimitiveTypes::Currency => write!(f, "currency"),
            PrimitiveTypes::Decimal => write!(f, "decimal"),
            PrimitiveTypes::Boolean => write!(f, "bool"),
            PrimitiveTypes::U8 => write!(f, "u8"),
            PrimitiveTypes::U16 => write!(f, "u16"),
            PrimitiveTypes::U32 => write!(f, "u32"),
            PrimitiveTypes::U64 => write!(f, "u64"),
            PrimitiveTypes::U128 => write!(f, "u128"),
            PrimitiveTypes::I8 => write!(f, "i8"),
            PrimitiveTypes::I16 => write!(f, "i16"),
            PrimitiveTypes::I32 => write!(f, "i32"),
            PrimitiveTypes::I64 => write!(f, "i64"),
            PrimitiveTypes::I128 => write!(f, "i128"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum CompositeTypes {
    Function {
        args: Vec<IrType>,
        generics: Vec<CompositeTypes>,
        ret: Box<IrType>,
    },
    Tuple(Vec<IrType>),
    Struct {
        name: String,
        generics: Vec<CompositeTypes>,
        fields: Vec<(String, IrType)>,
    },
    Enum {
        name: Arc<String>,
        generics: Vec<CompositeTypes>,
        variants: Vec<(String, CompositeTypes)>,
    },
}

impl Display for CompositeTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositeTypes::Tuple(elem) => {
                write!(f, "(")?;
                for (i, e) in elem.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", e)?;
                }
                write!(f, ")")
            }
            CompositeTypes::Struct {
                name,
                generics,
                fields,
            } => {
                write!(f, "struct {}", name)?;
                if !generics.is_empty() {
                    write!(f, "<")?;
                    for (i, g) in generics.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", g)?;
                    }
                    write!(f, ">")?;
                }
                write!(f, " {{")?;
                for (i, (name, ty)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, ty)?;
                }
                write!(f, "}}")
            }
            CompositeTypes::Function { args, ret, .. } => {
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ") -> {}", ret)
            }
            CompositeTypes::Enum {
                name,
                generics,
                variants,
            } => {
                write!(f, "enum {}", name)?;
                if !generics.is_empty() {
                    write!(f, "<")?;
                    for (i, g) in generics.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", g)?;
                    }
                    write!(f, ">")?;
                }
                write!(f, " {{")?;
                for (i, (name, ty)) in variants.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", name, ty)?;
                }
                write!(f, "}}")
            }
        }
    }
}

pub trait TypeOf {
    fn type_of(&self) -> IrType;
}

impl TypeOf for () {
    fn type_of(&self) -> IrType {
        IrType::void()
    }
}

impl TypeOf for Decimal {
    fn type_of(&self) -> IrType {
        IrType::Primitive(PrimitiveTypes::Decimal)
    }
}

impl<T> TypeOf for Option<T>
where
    T: TypeOf,
{
    fn type_of(&self) -> IrType {
        todo!()
    }
}
