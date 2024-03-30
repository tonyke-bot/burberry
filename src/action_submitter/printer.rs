use std::fmt::Debug;

use crate::ActionSubmitter;

#[derive(Debug, Clone)]
pub struct ActionPrinter<A> {
    _phantom: std::marker::PhantomData<A>,
}

impl<A> Default for ActionPrinter<A> {
    fn default() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<A> ActionSubmitter<A> for ActionPrinter<A>
where
    A: Send + Clone + Debug + Sync + 'static,
{
    fn submit(&self, a: A) {
        tracing::info!("action: {a:?}");
    }
}
