use crate::SmallVec;
use std::borrow::Cow;
use std::convert::TryInto;
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub struct Location {
    start: u16,
    end: u16,
}

impl Location {
    #[inline]
    pub fn range(&self) -> Range<usize> {
        Range {
            start: self.start.into(),
            end: self.end.into(),
        }
    }

    pub fn new(range: Range<usize>) -> Option<Self> {
        let start = range.start.try_into().ok()?;
        let end = range.end.try_into().ok()?;
        Some(Self { start, end })
    }
}

pub struct Words<'a, I>
where
    I: Iterator<Item = Location>,
{
    text: &'a str,
    locations: I,
}

impl<'a, I> Words<'a, I>
where
    I: Iterator<Item = Location>,
{
    #[inline]
    pub fn new(text: &'a str, locations: I) -> Words<'a, I> {
        Words { text, locations }
    }
}

impl<'a, I> Words<'a, I>
where
    I: Iterator<Item = Location> + ExactSizeIterator,
{
    pub fn join(mut self) -> Cow<'a, str> {
        match self.len() {
            0 => Cow::Borrowed(""),
            1 => Cow::Borrowed(self.next().unwrap()),
            _ => Cow::Owned(self.collect::<SmallVec<[&str; 4]>>().join(" ")),
        }
    }
}

impl<'a, I> Iterator for Words<'a, I>
where
    I: Iterator<Item = Location>,
{
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        self.locations.next().map(|loc| &self.text[loc.range()])
    }

    #[inline]
    fn fold<B, F>(self, init: B, mut f: F) -> B
    where
        F: FnMut(B, Self::Item) -> B,
    {
        let Self { locations, text } = self;
        locations.fold(init, |acc, loc| {
            let item = &text[loc.range()];
            f(acc, item)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.locations.size_hint()
    }
}

impl<'a, I> DoubleEndedIterator for Words<'a, I>
where
    I: Iterator<Item = Location> + DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<&'a str> {
        self.locations
            .next_back()
            .map(|loc| &self.text[loc.range()])
    }
}

impl<'a, I> ExactSizeIterator for Words<'a, I> where I: Iterator<Item = Location> + ExactSizeIterator
{}
