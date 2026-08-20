#[cfg(all(target_os = "macos", feature = "gpu"))]
pub use metal::Buffer as MetalBufferType;

#[cfg(not(all(target_os = "macos", feature = "gpu")))]
#[derive(Clone, Debug)]
pub struct MetalBufferType;

#[cfg(not(all(target_os = "macos", feature = "gpu")))]
pub mod metal {
    #[derive(Clone, Debug)]
    pub struct ComputeCommandEncoderRef;
    #[derive(Clone, Debug)]
    pub struct Buffer;

    impl ComputeCommandEncoderRef {
        pub fn set_buffer(
            &self,
            _index: u64,
            _buffer: Option<&super::MetalBufferType>,
            _offset: u64,
        ) {
        }
    }
}

pub trait MetalLayout {
    fn get_layout() -> String;
}

pub trait MetalPipeline {
    fn get_signature(kernel_name: &str) -> String;

    #[cfg(target_os = "macos")]
    fn bind(&self, encoder: &metal::ComputeCommandEncoderRef);
}

#[derive(Clone, Debug)]
pub struct DeviceConstRef<T>(pub MetalBufferType, pub std::marker::PhantomData<T>);

#[derive(Clone, Debug)]
pub struct ConstantRef<T>(pub MetalBufferType, pub std::marker::PhantomData<T>);

#[derive(Clone, Debug)]
pub struct DeviceConstPtr<T>(pub MetalBufferType, pub std::marker::PhantomData<T>);

#[derive(Clone, Debug)]
pub struct DevicePtr<T>(pub MetalBufferType, pub std::marker::PhantomData<T>);

#[derive(Clone, Debug)]
pub struct DeviceAtomicPtr<T>(pub MetalBufferType, pub std::marker::PhantomData<T>);

impl<T> DeviceConstRef<T> {
    pub fn new(b: MetalBufferType) -> Self {
        Self(b, std::marker::PhantomData)
    }
}

impl<T> ConstantRef<T> {
    pub fn new(b: MetalBufferType) -> Self {
        Self(b, std::marker::PhantomData)
    }
}

impl<T> DeviceConstPtr<T> {
    pub fn new(b: MetalBufferType) -> Self {
        Self(b, std::marker::PhantomData)
    }
}

impl<T> DevicePtr<T> {
    pub fn new(b: MetalBufferType) -> Self {
        Self(b, std::marker::PhantomData)
    }
}

impl<T> DeviceAtomicPtr<T> {
    pub fn new(b: MetalBufferType) -> Self {
        Self(b, std::marker::PhantomData)
    }
}
