use std::fmt::Debug;

use crate::ActionSubmitter;

pub struct ActionSubmitterMap<A1, A2, F> {
    submitter: Box<dyn ActionSubmitter<A2>>,
    f: F,
    _phantom: std::marker::PhantomData<A1>,
}

impl<A1, A2, F> ActionSubmitterMap<A1, A2, F> {
    pub fn new(submitter: Box<dyn ActionSubmitter<A2>>, f: F) -> Self {
        Self {
            submitter,
            f,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<A1, A2, F> ActionSubmitter<A1> for ActionSubmitterMap<A1, A2, F>
where
    A1: Send + Sync + Clone + Debug + 'static,
    A2: Send + Sync + Clone + Debug + 'static,
    F: Fn(A1) -> Option<A2> + Send + Sync + Clone + 'static,
{
    fn submit(&self, a: A1) {
        let a = match (self.f)(a) {
            Some(a) => a,
            None => return,
        };

        self.submitter.submit(a);
    }
}
