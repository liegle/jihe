pub(crate) trait AsUniformBytes {
    fn as_uniform_bytes(&self) -> Vec<u8>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsUniformBytes for T {
    fn as_uniform_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(vec![]);
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}

pub(crate) trait AsDynamicStorageBytes {
    fn as_dynamic_storage_bytes(&self) -> Vec<u8>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsDynamicStorageBytes for T {
    fn as_dynamic_storage_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::DynamicStorageBuffer::new(vec![]);
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}

#[cfg(feature = "profile")]
pub(super) fn log_profiler_recursive(
    results: &[wgpu_profiler::GpuTimerQueryResult],
    indent: usize,
) {
    for scope in results {
        log::info!(
            "{:>width$} {:.6}ms - {}",
            "*",
            match &scope.time {
                Some(time) => (time.end - time.start) * 1000.,
                None => f64::NAN,
            },
            scope.label,
            width = (indent + 1) * 4,
        );

        if !scope.nested_queries.is_empty() {
            log_profiler_recursive(&scope.nested_queries, indent + 1);
        }
    }
}
