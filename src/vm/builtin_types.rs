use crate::ir::ty::{CompositeTypes, IrType};

pub fn exchange() -> IrType {
    IrType::Composite(CompositeTypes::Struct {
        name: "ExchangeKind".to_string(),
        generics: vec![],
        fields: vec![("name".to_string(), IrType::string())],
    })
}

pub fn unit() -> IrType {
    IrType::Composite(CompositeTypes::Struct {
        name: "Unit".to_string(),
        generics: vec![],
        fields: vec![
            ("amount".to_string(), IrType::decimal()),
            ("price".to_string(), IrType::decimal()),
        ],
    })
}

pub fn option() -> IrType {
    todo!()
}
