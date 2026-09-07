use wasmtime::{Caller, Linker, StoreLimits};

pub struct HostContext {
    pub limits: StoreLimits,
}

pub fn register_host_functions(linker: &mut Linker<HostContext>) -> Result<(), wasmtime::Error> {
    linker.func_wrap(
        "aether",
        "log",
        |_caller: Caller<'_, HostContext>, level: i32, value: i32| {
            let _ = (level, value);
        },
    )?;

    Ok(())
}
