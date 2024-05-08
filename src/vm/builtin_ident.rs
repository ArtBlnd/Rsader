use strum::IntoEnumIterator;

use crate::exchange::ExchangeKind;

use super::{builtin_types, context::VmContext, value::Value};

pub fn register_builtin_identifiers(context: &mut VmContext) {
    for ex in ExchangeKind::iter() {
        let value = Value::Struct(
            [("name".to_string(), Value::Str(ex.to_string()))]
                .try_into()
                .unwrap(),
            builtin_types::exchange(),
        );

        context.set_variable(ex.to_string().as_str(), value);
    }
}
