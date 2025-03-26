#[doc(hidden)]
pub use derive_deftly;
use derive_deftly::define_derive_deftly;
use sha2::{
    Sha256,
    digest::{Output, OutputSizeUser},
};
use std::mem::ManuallyDrop;
use std::{array, mem, mem::MaybeUninit};

derive_deftly::template_export_semver_check!("1.0.1");

pub trait Bytes {
    fn to_bytes(&self) -> Box<[u8]> {
        let mut writer = BytesWriter::new(self.required_size());
        self.write(&mut writer);
        writer.finish()
    }

    fn from_bytes(bytes: &[u8]) -> Self
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(bytes);
        Self::read(&mut reader)
    }

    fn required_size(&self) -> usize;
    fn write(&self, writer: &mut BytesWriter);
    fn read(reader: &mut BytesReader) -> Self
    where
        Self: Sized;
}

pub struct BytesWriter {
    data: Box<[MaybeUninit<u8>]>,
    written: usize,
}

impl BytesWriter {
    fn new(capacity: usize) -> Self {
        Self {
            data: Box::new_uninit_slice(capacity),
            written: 0,
        }
    }

    fn write(&mut self, bytes: &[u8]) {
        assert!(self.written + bytes.len() <= self.data.len());

        // SAFETY: `MaybeUninit<u8>` has the same size and layout as `u8`.
        self.data[self.written..self.written + bytes.len()]
            .copy_from_slice(unsafe { &*(bytes as *const _ as *const _) });

        self.written += bytes.len();
    }

    fn finish(self) -> Box<[u8]> {
        assert_eq!(self.written, self.data.len());

        // SAFETY: Since `self.written` is equal to the data length, then the data has been fully
        // initialized:
        unsafe { self.data.assume_init() }
    }
}

pub struct BytesReader<'a>(&'a [u8]);

impl<'a> BytesReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    fn read(&mut self, bytes: &mut [MaybeUninit<u8>]) {
        assert!(bytes.len() <= self.0.len());

        // SAFETY: `MaybeUninit<u8>` has the same size and layout as `u8`.
        bytes.copy_from_slice(unsafe { &*(&self.0[..bytes.len()] as *const _ as *const _) });

        self.0 = &self.0[bytes.len()..];
    }
}

define_derive_deftly! {
    export Bytes:

    impl<$tgens> $crate::Bytes for $ttype
    where
        $($ftype: $crate::Bytes,)
    {
        fn required_size(&self) -> usize {
            let mut size = 0;
            $(
                size += <$ftype as $crate::Bytes>::required_size(&self.$fname);
            )
            size
        }

        #[allow(unused)]
        fn write(&self, writer: &mut $crate::BytesWriter) {
            $(<$ftype as $crate::Bytes>::write(&self.$fname, writer);)
        }

        #[allow(unused)]
        fn read(reader: &mut $crate::BytesReader) -> Self {
            Self {
                $($fname: <$ftype as $crate::Bytes>::read(reader),)
            }
        }
    }
}

macro_rules! impl_bytes_for_uint {
    ($ty:ty) => {
        impl Bytes for $ty {
            fn required_size(&self) -> usize {
                size_of::<$ty>()
            }

            fn write(&self, writer: &mut BytesWriter) {
                writer.write(&self.to_le_bytes());
            }

            fn read(reader: &mut BytesReader) -> Self {
                let mut bytes = [MaybeUninit::uninit(); size_of::<$ty>()];
                reader.read(&mut bytes);

                // SAFETY: `bytes` is fully initialized by the reader.
                unsafe { *(&bytes as *const _ as *const _) }
            }
        }
    };
}

impl_bytes_for_uint!(u8);
impl_bytes_for_uint!(u64);

impl Bytes for usize {
    fn required_size(&self) -> usize {
        u64::required_size(&(*self as u64))
    }

    fn write(&self, writer: &mut BytesWriter) {
        u64::write(&(*self as u64), writer);
    }

    fn read(reader: &mut BytesReader) -> Self {
        let size = u64::read(reader);
        size.try_into().expect("size too large")
    }
}

impl<const N: usize, T> Bytes for [T; N]
where
    T: Bytes,
{
    fn required_size(&self) -> usize {
        self.iter().map(|elem| elem.required_size()).sum()
    }

    fn write(&self, writer: &mut BytesWriter) {
        for elem in self {
            elem.write(writer);
        }
    }

    fn read(reader: &mut BytesReader) -> Self {
        array::from_fn(|_| T::read(reader))
    }
}

impl<T> Bytes for Box<[T]>
where
    T: Bytes,
{
    fn required_size(&self) -> usize {
        8 + self.iter().map(|elem| elem.required_size()).sum::<usize>()
    }

    fn write(&self, writer: &mut BytesWriter) {
        let size = self.len();
        (size as u64).write(writer);

        for elem in self {
            elem.write(writer);
        }
    }

    fn read(reader: &mut BytesReader) -> Self {
        let len = u64::read(reader);
        let mut elems = Box::new_uninit_slice(len.try_into().expect("size too large"));
        for i in 0..len as usize {
            elems[i].write(T::read(reader));
        }

        // SAFETY: `elems` is fully initialized by the reader.
        unsafe { elems.assume_init() }
    }
}

impl<T> Bytes for Vec<T>
where
    T: Bytes,
{
    fn required_size(&self) -> usize {
        8 + self.iter().map(|elem| elem.required_size()).sum::<usize>()
    }

    fn write(&self, writer: &mut BytesWriter) {
        let size = self.len();
        (size as u64).write(writer);

        for elem in self {
            elem.write(writer);
        }
    }

    fn read(reader: &mut BytesReader) -> Self {
        let len = u64::read(reader);
        let mut elems = Vec::with_capacity(len.try_into().expect("size to large"));
        for _ in 0..len as usize {
            elems.push(T::read(reader));
        }

        elems
    }
}

impl Bytes for Output<Sha256> {
    fn required_size(&self) -> usize {
        <Sha256 as OutputSizeUser>::output_size()
    }

    fn write(&self, writer: &mut BytesWriter) {
        writer.write(self.as_slice());
    }

    fn read(reader: &mut BytesReader) -> Self {
        let mut bytes = Box::new_uninit_slice(<Sha256 as OutputSizeUser>::output_size());
        reader.read(&mut bytes);

        // SAFETY: `bytes` has the same size and layout as `Output<Sha256>`.
        unsafe {
            let bytes = ManuallyDrop::new(bytes);
            mem::transmute_copy(&bytes)
        }
    }
}
