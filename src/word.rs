use crate::SmallVec;
use std::borrow::Cow;
use std::convert::TryInto;
use std::ops::{Deref, Range};
use std::slice;

pub struct Words<'a> {
    text: &'a str,
    loc: slice::Iter<'a, Range<u16>>,
}

impl<'a> Words<'a> {
    #[inline]
    pub fn new(text: &'a str, loc: &'a [Range<u16>]) -> Words<'a> {
        Words {
            text,
            loc: loc.iter(),
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
        self.loc
            .next()
            .map(|&Range { start, end }| &self.text[start.into()..end.into()])
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let Self { loc, text } = self;
        loc.fold(init, |acc, &Range { start, end }| {
            let item = &text[start.into()..end.into()];
            f(acc, item)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.loc.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Words<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.loc
            .next_back()
            .map(|&Range { start, end }| &self.text[start.into()..end.into()])
    }
}

impl<'a> ExactSizeIterator for Words<'a> {}

// Storing as u16 is sufficient because MAX_NAME_LEN is 1024.
//
// We could compress a bit further (especially given that the common case is much shorter)
// but it's not obviously worth the code complexity.
#[derive(Clone, Debug)]
pub struct Locations(SmallVec<[Range<u16>; 4]>);

impl Locations {
    #[inline]
    pub fn push(&mut self, loc: Range<usize>) {
        self.0
            .push(loc.start.try_into().unwrap()..loc.end.try_into().unwrap())
    }

    #[inline]
    pub fn with_capacity(size: usize) -> Locations {
        Locations(SmallVec::with_capacity(size))
    }

    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.0.shrink_to_fit();
    }
}

impl Deref for Locations {
    type Target = [Range<u16>];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
