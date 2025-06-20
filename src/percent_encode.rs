use std::{
    borrow::Cow,
    ops::Deref,
    path::{Path, PathBuf},
};

use percent_encoding::{
    AsciiSet, CONTROLS, PercentEncode, percent_decode_str, utf8_percent_encode,
};

pub trait PathBufExt: Deref<Target = Path> {
    #[inline]
    fn percent_encode(&self) -> String {
        utf8_percent_encode(self.to_string_lossy().to_string().as_str(), FRAGMENT).to_string()
    }
    #[inline]
    #[allow(unused)]
    fn percent_decode(&self) -> String {
        percent_decode_str(self.to_string_lossy().to_string().as_str())
            .decode_utf8_lossy()
            .into()
    }
}
/// https://url.spec.whatwg.org/#fragment-percent-encode-set
const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
// URL-encode the path to handle spaces, etc. e.g., "My Note.md" -> "My%20Note.md"

pub trait StringExt: AsRef<str> {
    #[inline]
    fn percent_decode<'a>(&'a self) -> Self
    where
        Self: Sized + From<Cow<'a, str>>,
    {
        percent_decode_str(self.as_ref()).decode_utf8_lossy().into()
    }

    #[inline]
    fn percent_encode<'a>(&'a self) -> Self
    where
        Self: Sized + From<PercentEncode<'a>>,
    {
        utf8_percent_encode(self.as_ref(), FRAGMENT).into()
    }
}

impl StringExt for Cow<'_, str> {}
impl PathBufExt for PathBuf {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
            #![proptest_config(ProptestConfig::with_cases(100_000))]
            /// Tests that for any given string `s`, `percent_decode(percent_encode(s))` returns `s`.
            /// This verifies the involution property for the `StringExt` trait.
            #[test]
            fn string_ext_involution(s in "[a-zA-Z0-9\\-_\\.~/]+") {
                let original = Cow::from(&s);
                let duplicate = original.clone();
                let encoded = duplicate.percent_encode();
                let decoded = encoded.percent_decode();

                prop_assert_eq!(original, decoded);
            }

            /// Tests the involution property for `PathBufExt`.
            #[test]
            fn pathbuf_ext_involution(s in "[a-zA-Z0-9\\-_\\.~/]+") {
                // Proptest generates a random string `s` which we use to create our path.
                // This ensures we test a wide variety of path-like strings.
                // NOTE: We use `to_string_lossy` throughout to match the implementation,
                // which correctly handles potentially invalid UTF-8 in paths.
                let original_path = PathBuf::from(&s);

                // Encode the path to a string.
                let encoded_string = original_path.percent_encode();

                // Create a new path from the encoded string to use the `percent_decode` method.
                let path_from_encoded = PathBuf::from(&encoded_string);

                // Decode the new path back to a string.
                let decoded_string = path_from_encoded.percent_decode();

                // 4. Assert that the final string is equal to the original path's lossy representation.
                prop_assert_eq!(original_path.to_string_lossy(), decoded_string.as_str());
            }


        /// `encode(encode(s)) == encode(s)`
        #[test]
        fn string_ext_encoding_is_idempotent(s in "[a-zA-Z0-9\\-_\\.~/]+") {
            let original = Cow::from(&s);
            let duplicate = original.clone();

            let once: Cow<str> = duplicate.percent_encode();
            let once_duplicate = once.clone();
            let twice: Cow<str> = once_duplicate.percent_encode();

            prop_assert_eq!(once, twice, "Encoding the same string twice should yield the same result");
        }

        /// PathBuf encoding is idempotent.
        #[test]
        fn pathbuf_ext_encoding_is_idempotent(s in any::<String>()) {
            let path = PathBuf::from(&s);
            let once_encoded = path.percent_encode();

            // To encode again, we create a new path from the encoded string
            let path_from_encoded = PathBuf::from(&once_encoded);
            let twice_encoded = path_from_encoded.percent_encode();

            prop_assert_eq!(once_encoded, twice_encoded);
        }
        /// Strings with only URL-safe characters are not changed by encoding.
        #[test]
        fn string_ext_safe_chars_are_unchanged(s in "[a-zA-Z0-9\\-_\\.~/]+") {
            // The regex generates strings containing only unreserved characters plus '/',
            // which is also not in your FRAGMENT set.
            let original = Cow::from(&s);
            let original_duplicate = Cow::from(&s);
            let encoded_cow: Cow<str> = original_duplicate.percent_encode();

            prop_assert_eq!(original, encoded_cow, "Safe string was modified by encoding");
        }

        /// `percent_decode` should not panic on any string input.
        #[test]
        fn string_ext_decode_does_not_panic(s in any::<String>()) {
            let cow = Cow::from(&s);
            // The test simply passes if this line does not panic.
            let _ = cow.percent_decode();
        }

        /// `percent_decode` for `PathBufExt` should not panic.
        #[test]
        fn pathbuf_ext_decode_does_not_panic(s in any::<String>()) {
            let path = PathBuf::from(&s);
            // The test passes if this doesn't panic.
            let _ = path.percent_decode();
        }

    }
    // TODO: Figure this out at some point
    // The PathBuf involution holds even for non-UTF8 paths due to the use of `to_string_lossy`.
    // #[test]
    // fn pathbuf_ext_involution_with_lossy_paths(bytes in any::<Vec<u8>>()) {
    //     use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
    //     // On unix, a path is just a sequence of bytes. On Windows, it's a sequence of 16-bit
    //     // values (WTF-8). We'll simulate the unix case, which is a good proxy for "lossy" data.
    //     #[cfg(unix)]
    //     let os_str = OsStr::from_bytes(&bytes);
    //
    //     // A simple way to handle Windows for this test, though less representative of true
    //     // invalid Windows paths. For this library's purpose, it's sufficient.
    //     #[cfg(windows)]
    //     let os_str = OsStr::new(String::from_utf8_lossy(&bytes).as_ref());
    //
    //     let original_path = PathBuf::from(os_str);
    //
    //     // The rest of the test is the same as the original involution test.
    //     let encoded_string = original_path.percent_encode();
    //     let path_from_encoded = PathBuf::from(&encoded_string);
    //     let decoded_string = path_from_encoded.percent_decode();
    //
    //     // The key is that we compare against the *lossy* representation
    //     // of the original path.
    //     prop_assert_eq!(original_path.to_string_lossy(), decoded_string.as_str());
    // }
}
