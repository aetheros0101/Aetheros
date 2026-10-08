// ============================================================
// src/runtime/bootstrap.rs
//
// Faz 7 Güncellemesi:
//
// runtime_handle() eklendi → RuntimeHandle döndürür.
// RuntimeHandle, API katmanının task submit için
// kullandığı tek giriş noktası.
//
// Tam sistem başlatma örneği:
//
//   let bootstrap = RuntimeBootstrap::build(config)?;
//   let handle    = bootstrap.runtime_handle();
//   let events    = bootstrap.runtime().events();
//   let persists  = bootstrap.runtime().persistence();
//
//   tokio::spawn(async move {
//       bootstrap.runtime().start().await
//   });
//
//   ApiServer::new(handle, events, persists, addr)
//       .serve()
//       .await?;
// ============================================================

use tokio::sync::mpsc;

use crate::errors::runtime::RuntimeError;
use crate::runtime::api::RuntimeHandle;
use crate::runtime::config::RuntimeConfig;
use crate::runtime::runtime::Runtime;
use crate::task::task::TaskDefinition;

pub struct RuntimeBootstrap {
    runtime: Runtime,
    sender: mpsc::Sender<TaskDefinition>,
}

impl RuntimeBootstrap {
    pub fn build(config: RuntimeConfig) -> Result<Self, RuntimeError> {
        let (sender, receiver) = mpsc::channel(config.task_channel_capacity);

        let runtime = Runtime::new(config, receiver)?;

        Ok(Self { runtime, sender })
    }

    /// API katmanı için RuntimeHandle.
    ///
    /// RuntimeHandle, mpsc::Sender'ı sarar.
    /// Clone'lanabilir — ApiServer, Agent, Workflow
    /// hepsi aynı handle'ı kullanabilir.
    pub fn runtime_handle(&self) -> RuntimeHandle {
        RuntimeHandle::new(self.sender.clone())
    }

    pub fn runtime(self) -> Runtime {
        self.runtime
    }

    /// Ham sender — geriye dönük uyumluluk için.
    pub fn sender(&self) -> mpsc::Sender<TaskDefinition> {
        self.sender.clone()
    }
}
