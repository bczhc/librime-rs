#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[macro_export]
macro_rules! rime_struct {
    ($var:ident : $t:ty) => {
        let $var = std::mem::MaybeUninit::<$t>::zeroed();
        let mut $var = unsafe { $var.assume_init() };
        $var.data_size =
            (std::mem::size_of::<$t>() - std::mem::size_of_val(&$var.data_size)) as std::ffi::c_int;
    };
}

#[cfg(test)]
mod test {
    use std::mem;

    use crate::RimeTraits;

    #[test]
    fn test() {
        rime_struct!(traits: RimeTraits);
        assert_eq!(
            traits.data_size as usize,
            mem::size_of::<RimeTraits>() - mem::size_of_val(&traits.data_size)
        );
    }
}
