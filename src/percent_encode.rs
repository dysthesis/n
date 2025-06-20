use std::{borrow::Cow, path::PathBuf};

use percent_encoding::{
    AsciiSet, CONTROLS, PercentEncode, percent_decode_str, utf8_percent_encode,
};

pub trait PathBufExt {
    fn percent_encode(&self) -> String;
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

    fn percent_encode<'a>(&'a self) -> Self
    where
        Self: Sized + From<PercentEncode<'a>>,
    {
        utf8_percent_encode(self.as_ref(), FRAGMENT).into()
    }
}

impl StringExt for Cow<'_, str> {}

impl PathBufExt for PathBuf {
    #[inline]
    fn percent_encode(&self) -> String {
        utf8_percent_encode(self.to_string_lossy().to_string().as_str(), FRAGMENT).to_string()
    }
}
