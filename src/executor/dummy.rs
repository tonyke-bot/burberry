use crate::Executor;

pub struct Dummy;

#[async_trait::async_trait]
impl<A: Send + Sync + 'static> Executor<A> for Dummy {
    fn name(&self) -> &str {
        "Dummy"
    }

    async fn execute(&self, action: A) -> eyre::Result<()> {
        let _ = action;

        Ok(())
    }
}
