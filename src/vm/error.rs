use std::{error::Error as StdError, fmt};

use crate::utils::maybe_trait::MaybeSend;

use rune::alloc::fmt::TryWrite;
use rune::runtime::{Formatter, VmResult};

pub fn install_module_error(context: &mut rune::Context) {
    let mut module = rune::Module::new();

    module.ty::<Error>().unwrap();
    module.function_meta(Error::display).unwrap();
    module.function_meta(Error::string_display).unwrap();

    context.install(module).unwrap();
}

#[cfg(any(target_arch = "wasm32"))]
#[derive(rune::Any)]
pub struct Error(Box<dyn std::error::Error>);

#[cfg(not(target_arch = "wasm32"))]
#[derive(rune::Any)]
pub struct Error(Box<dyn std::error::Error + Send>);
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    #[rune::function(instance)]
    pub fn display(&self) -> String {
        self.0.to_string()
    }

    #[rune::function(instance, protocol = STRING_DISPLAY)]
    fn string_display(&self, f: &mut Formatter) -> VmResult<()> {
        rune::vm_write!(f, "{}", self);
        VmResult::Ok(())
    }

    pub fn from_stderr(err: impl StdError + MaybeSend + 'static) -> Self {
        Self(Box::new(err))
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error({:?})", self.0)
    }
}
