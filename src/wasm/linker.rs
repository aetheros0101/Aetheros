use wasmtime::Linker;

use crate::wasm::host::{
    register_host_functions,
    HostContext,
};

pub fn create_linker(
    engine: &wasmtime::Engine,
) -> Result<
    Linker<HostContext>,
    wasmtime::Error,
> {
    let mut linker = Linker::new(engine);

    register_host_functions(&mut linker)?;

    Ok(linker)
}
