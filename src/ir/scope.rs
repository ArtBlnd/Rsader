use std::collections::HashMap;

use super::{ty::IrType, value::IrVariable};

/// A scope is a collection of definions that are in scope at a given point in the program.
/// Those definitions can be variables, functions, types, etc.
#[derive(Debug, Clone)]
pub struct Scope<'ir> {
    pub(super) variables: HashMap<String, IrVariable<'ir>>,
    pub(super) types: HashMap<String, &'ir IrType>,

    varaible_index: usize,

    parent: Option<Box<Scope<'ir>>>,
}

impl<'ir> Scope<'ir> {
    pub fn new_scope(parent: Option<Scope<'ir>>) -> Self {
        Self {
            variables: HashMap::new(),
            types: HashMap::new(),

            varaible_index: parent.as_ref().map_or(0, |p| p.varaible_index),

            parent: parent.map(Box::new),
        }
    }

    pub fn pop_scope(&mut self) -> Option<Scope<'ir>> {
        self.parent.take().map(|f| *f)
    }

    pub fn new_type(&mut self, name: impl AsRef<str>, ty: &'ir IrType) {
        self.types.insert(name.as_ref().to_string(), ty);
    }

    pub fn get_type(&self, name: impl AsRef<str>) -> Option<&'ir IrType> {
        self.types.get(name.as_ref()).copied().or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_type(name))
        })
    }

    pub fn new_variable(&mut self, name: impl AsRef<str>, ty: &'ir IrType) -> IrVariable<'ir> {
        let var = IrVariable::new(self.varaible_index, ty);
        self.variables.insert(name.as_ref().to_string(), var);
        self.varaible_index += 1;

        var
    }

    pub fn get_variable(&self, name: impl AsRef<str>) -> Option<IrVariable<'ir>> {
        self.variables.get(name.as_ref()).copied().or_else(|| {
            self.parent
                .as_ref()
                .and_then(|parent| parent.get_variable(name))
        })
    }
}
