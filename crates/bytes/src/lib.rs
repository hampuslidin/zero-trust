use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    mem::{self, MaybeUninit},
};

#[doc(hidden)]
pub use derive_deftly;
use derive_deftly::define_derive_deftly;

derive_deftly::template_export_semver_check!("1.0.1");

pub trait Bytes {
    fn to_bytes(&self) -> Box<[u8]> {
        let mut writer = BytesWriter::new(self.required_size());
        self.write(&mut writer);
        writer.finish()
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, BytesError>
    where
        Self: Sized,
    {
        let mut reader = BytesReader::new(bytes);
        let output = Self::read(&mut reader)?;
        reader.finish()?;
        Ok(output)
    }

    fn required_size(&self) -> usize;
    fn write(&self, writer: &mut BytesWriter);
    fn read(reader: &mut BytesReader) -> Result<Self, BytesError>
    where
        Self: Sized;
}

#[derive(Debug)]
pub enum BytesError {
    EndOfData(usize),
    TrailingData(usize),
    UsizeTooSmall,
}

impl Error for BytesError {}

impl Display for BytesError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::EndOfData(pos) => write!(f, "end of data at position {pos}"),
            Self::TrailingData(pos) => write!(f, "trailing data at position {pos}"),
            Self::UsizeTooSmall => write!(f, "data could not fit into `usize`"),
        }
    }
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

pub struct BytesReader<'a> {
    data: &'a [u8],
    read: usize,
}

impl<'a> BytesReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            data: bytes,
            read: 0,
        }
    }

    fn read(&mut self, bytes: &mut [MaybeUninit<u8>]) -> Result<(), BytesError> {
        if self.read + bytes.len() > self.data.len() {
            return Err(BytesError::EndOfData(self.read));
        } 

        // SAFETY: `MaybeUninit<u8>` has the same size and layout as `u8`.
        bytes.copy_from_slice(unsafe { mem::transmute(&self.data[self.read..self.read + bytes.len()]) });

        self.read += bytes.len();

        Ok(())
    }

    fn finish(self) -> Result<(), BytesError> {
        if self.read == self.data.len() {
            Ok(())
        } else {
            Err(BytesError::TrailingData(self.read))
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

            fn read(reader: &mut BytesReader) -> Result<Self, BytesError> {
                let mut bytes = [MaybeUninit::uninit(); size_of::<$ty>()];
                reader.read(&mut bytes)?;

                // SAFETY: `bytes` is fully initialized by the reader.
                Ok(<$ty>::from_le_bytes(unsafe { mem::transmute_copy(&bytes) }))
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

    fn read(reader: &mut BytesReader) -> Result<Self, BytesError> {
        u64::read(reader)?.try_into().map_err(|_| BytesError::UsizeTooSmall)
    }
}

macro_rules! impl_bytes_for_tuple {
    ($(($i:tt, $t:ident)),+) => {
        impl<$($t),+> Bytes for ($($t),+) 
        where
            $($t: Bytes,)+
        {
            fn required_size(&self) -> usize {
                $(self.$i.required_size() +)+ 0
            }

            fn write(&self, writer: &mut BytesWriter) {
                $(self.$i.write(writer);)+
            }

            fn read(reader: &mut BytesReader) -> Result<Self, BytesError> {
                Ok(($($t::read(reader)?),+))
            }
        }
    };
}

impl_bytes_for_tuple!((0, T1), (1, T2));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7), (7, T8));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7), (7, T8), (8, T9));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7), (7, T8), (8, T9), (9, T10));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7), (7, T8), (8, T9), (9, T10), (10, T11));
impl_bytes_for_tuple!((0, T1), (1, T2), (2, T3), (3, T4), (4, T5), (5, T6), (6, T7), (7, T8), (8, T9), (9, T10), (10, T11), (11, T12));

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

    fn read(reader: &mut BytesReader) -> Result<Self, BytesError> {
        let mut elems = [const { MaybeUninit::uninit() }; N];
        for i in 0..N {
            elems[i].write(T::read(reader)?);
        }

        // SAFETY: `elems` is fully initialized by the reader.
        Ok(unsafe { mem::transmute_copy(&elems) })
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
        let len = self.len();
        (len as u64).write(writer);

        for elem in self {
            elem.write(writer);
        }
    }

    fn read(reader: &mut BytesReader) -> Result<Self, BytesError> {
        let len = u64::read(reader)?.try_into().map_err(|_| BytesError::UsizeTooSmall)?;
        let mut elems = Box::new_uninit_slice(len);
        for i in 0..len {
            elems[i].write(T::read(reader)?);
        }

        // SAFETY: `elems` is fully initialized by the reader.
        Ok(unsafe { elems.assume_init() })
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
        let len = self.len();
        (len as u64).write(writer);

        for elem in self {
            elem.write(writer);
        }
    }

    fn read(reader: &mut BytesReader) -> Result<Self, BytesError> {
        let len = u64::read(reader)?.try_into().map_err(|_| BytesError::UsizeTooSmall)?;
        let mut elems = Vec::with_capacity(len);
        for _ in 0..len {
            elems.push(T::read(reader)?);
        }

        Ok(elems)
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
        fn read(reader: &mut $crate::BytesReader) -> Result<Self, $crate::BytesError> {
            Ok(Self {
                $($fname: <$ftype as $crate::Bytes>::read(reader)?,)
            })
        }
    }
}
