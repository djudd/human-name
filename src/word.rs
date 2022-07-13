use std::borrow::Cow;
use std::convert::TryInto;
use std::iter::Peekable;
use std::ops::{Deref, Range};
use std::slice;
use SmallVec;

pub struct Words<'a> {
    text: &'a str,
    indices: Peekable<slice::Iter<'a, Range<u16>>>,
}

impl<'a> Words<'a> {
    #[inline]
    pub fn new(text: &'a str, indices: &'a [Range<u16>]) -> Words<'a> {
        Words {
            text,
            indices: indices.iter().peekable(),
        }
    }

    pub fn join(mut self) -> Cow<'a, str> {
        match self.len() {
            0 => Cow::Borrowed(""),
            1 => Cow::Borrowed(self.next().unwrap()),
            _ => Cow::Owned(self.collect::<SmallVec<[&str; 4]>>().join(" ")),
        }
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.indices
            .next()
            .map(|&Range { start, end }| &self.text[start.into()..end.into()])
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Words<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.indices
            .next_back()
            .map(|&Range { start, end }| &self.text[start.into()..end.into()])
    }
}

impl<'a> ExactSizeIterator for Words<'a> {}

#[derive(Clone, Debug)]
pub struct WordIndices(SmallVec<[Range<u16>; 4]>);

impl WordIndices {
    #[inline]
    pub fn new() -> Self {
        WordIndices(SmallVec::new())
    }

    #[inline]
    pub fn push(&mut self, indices: Range<usize>) {
        self.0
            .push(indices.start.try_into().unwrap()..indices.end.try_into().unwrap())
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }
}

impl Deref for WordIndices {
    type Target = [Range<u16>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
