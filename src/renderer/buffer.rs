pub trait AsUniformBytes {
    fn as_uniform_bytes(&self) -> Vec<u8>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsUniformBytes for T {
    fn as_uniform_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::UniformBuffer::new(vec![]);
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}

pub trait AsDynamicStorageBytes {
    fn as_dynamic_storage_bytes(&self) -> Vec<u8>;
}

impl<T: encase::ShaderType + encase::internal::WriteInto> AsDynamicStorageBytes for T {
    fn as_dynamic_storage_bytes(&self) -> Vec<u8> {
        let mut buffer = encase::DynamicStorageBuffer::new(vec![]);
        buffer.write(self).unwrap();
        buffer.into_inner()
    }
}
