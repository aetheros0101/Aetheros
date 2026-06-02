use std::sync::Arc;

use async_trait::async_trait;


use crate::errors::runtime::RuntimeError;

use crate::orchestration::graph::{
    ExecutionNode,
    ExecutionNodeKind,
};

use crate::remote::scheduler::RemoteScheduler;

use crate::remote::transport::RemoteTransport;

use crate::runtime::dispatcher::Dispatcher;



#[async_trait]
pub trait NodeDispatcher:
    Send + Sync
{
    async fn dispatch(
        &self,
        node: ExecutionNode,
    ) -> Result<(), RuntimeError>;
}

pub struct ExecutionDispatcher<T>
where
    T: RemoteTransport,
{
    transport: Arc<T>,
    dispatcher:
            Arc<Dispatcher>,
}

impl<T> ExecutionDispatcher<T>
where
    T: RemoteTransport,
{
    pub fn new(
        transport: Arc<T>,
    
        dispatcher:
            Arc<Dispatcher>,
    ) -> Self {
        Self {
            transport,
            dispatcher,
        }
    }
}

#[async_trait]
impl<T> NodeDispatcher
    for ExecutionDispatcher<T>
where
    T: RemoteTransport,
{
    async fn dispatch(
        &self,
        node: ExecutionNode,
    ) -> Result<(), RuntimeError>
    {
        match node.kind {
            ExecutionNodeKind::Task
            | ExecutionNodeKind::Wasm => {
                let _ =
                    &self.dispatcher;
            }

            ExecutionNodeKind::RemoteTask => {
                let _ =
                    RemoteScheduler;
            }

            ExecutionNodeKind::Agent => {}
            ExecutionNodeKind::Workflow => {}
            ExecutionNodeKind::AiInference => {}
            ExecutionNodeKind::Plugin => {}
            
            
        }

        let _ = &self.transport;

        Ok(())
    }
}
