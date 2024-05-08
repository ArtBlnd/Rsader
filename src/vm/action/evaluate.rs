use crate::{
    ast::{ExprParser, Lexer},
    console,
    ir::ty::{IrType, TypeOf},
    vm::{context::VmContext, eval_ast::Evaluate, VmError},
};

use super::{Action, ActionBehavior, ActionState, TaskContext};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EvaluateActionState {
    #[default]
    Evaluating,
}

impl ActionState for EvaluateActionState {
    fn state(&self) -> &'static str {
        match self {
            EvaluateActionState::Evaluating => "EVALUATING",
        }
    }
}

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum EvaluateActionError {
    #[error("Failed to evaluate input: {0}")]
    EvaluateError(#[from] VmError),

    #[error("Failed to parse input")]
    ParseError,
}

pub struct EvaluateAction {
    pub input: String,
    pub context: VmContext,
}

impl TypeOf for EvaluateAction {
    fn type_of(&self) -> IrType {
        IrType::void()
    }
}

impl Action for EvaluateAction {
    const NAME: &'static str = "evaluate";

    type ExecutionResult = ();
    type State = EvaluateActionState;
    type Error = EvaluateActionError;

    async fn execute(
        &mut self,
        context: &TaskContext,
        _state: &mut Self::State,
    ) -> Result<ActionBehavior<Self::ExecutionResult>, Self::Error> {
        let lexer = Lexer::new(&self.input);
        let Ok(expr) = ExprParser::new().parse(lexer) else {
            return Err(EvaluateActionError::ParseError);
        };

        let value = Evaluate {
            expr,
            task_context: context.clone(),
        }
        .eval(&mut self.context)
        .await?;

        console!("{}", value.eval_and_display().await);
        Ok(ActionBehavior::Stop(()))
    }

    async fn execute_fallback(
        &mut self,
        _context: &TaskContext,
        _state: Self::State,
        _error: Option<Self::Error>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}
