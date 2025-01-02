// Copied from https://stackoverflow.com/questions/51344951/how-do-you-unwrap-a-result-on-ok-or-return-from-the-function-on-err
#[macro_export]
macro_rules! unwrap_or_return {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return,
        }
    };
    ($e:expr, $r:expr) => {
        match $e {
            Some(x) => x,
            None => return $r,
        }
    };
}

// Copied from https://users.rust-lang.org/t/can-i-conveniently-compile-bytes-into-a-rust-program-with-a-specific-alignment/24049/2
#[repr(C)] // guarantee 'bytes' comes after '_align'
pub struct AlignedAs<Align, Bytes: ?Sized> {
    pub _align: [Align; 0],
    pub bytes: Bytes,
}

#[macro_export]
macro_rules! include_bytes_align_as {
    ($align_ty:ty, $path:expr) => {{
        // const block expression to encapsulate the static
        use $crate::macros::AlignedAs;

        // this assignment is made possible by CoerceUnsized
        static ALIGNED: &AlignedAs<$align_ty, [u8]> = &AlignedAs {
            _align: [],
            bytes: *include_bytes!($path),
        };

        &ALIGNED.bytes
    }};
}

#[macro_export]
macro_rules! scalar_enum {
    {
        #[repr($ty:ty)]
        $(#[$meta:meta])*
        $vi:vis enum $name:ident {
            $($variant:ident,)*
        }
    } => {
        #[repr($ty)]
        $(#[$meta])?
        $vi enum $name {
            $($variant),*
        }

        impl TryFrom<$ty> for $name {
            type Error = ();

            fn try_from(value: $ty) -> Result<Self, Self::Error> {
                match value {
                    $(
                        ${index()} => Ok(Self::$variant),
                    )*
                    _ => Err(())
                }
            }
        }
    }
}
