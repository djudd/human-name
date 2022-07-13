use std::borrow::Cow;
use std::collections::VecDeque;
use std::convert::TryInto;
use std::ops::Range;

pub struct Words<'a> {
    text: &'a str,
    indices: WordIndices,
}

impl<'a> Words<'a> {
    pub fn new(text: &'a str, indices: &WordIndices, range: Range<usize>) -> Words<'a> {
        let mut indices = indices.clone();

        for _ in 0..range.start {
            indices.next();
        }
        while indices.len() > range.len() {
            indices.next_back();
        }

        Words { text, indices }
    }

    pub fn join(mut self) -> Cow<'a, str> {
        match self.len() {
            0 => Cow::Borrowed(""),
            1 => Cow::Borrowed(self.next().unwrap()),
            _ => Cow::Owned(self.collect::<Vec<_>>().join(" ")),
        }
    }
}

impl<'a> Iterator for Words<'a> {
    type Item = &'a str;
    fn next(&mut self) -> Option<&'a str> {
        self.indices.next().map(|i| &self.text[i])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.indices.size_hint()
    }
}

impl<'a> DoubleEndedIterator for Words<'a> {
    fn next_back(&mut self) -> Option<&'a str> {
        self.indices.next_back().map(|i| &self.text[i])
    }
}

impl<'a> ExactSizeIterator for Words<'a> {}

#[derive(Clone, Debug)]
pub enum WordIndices {
    Short { starts: u64, ends: u64 },
    Unbounded(VecDeque<Range<usize>>),
}

impl WordIndices {
    pub fn new() -> Self {
        WordIndices::Short { starts: 0, ends: 0 }
    }

    pub fn push(&mut self, indices: Range<usize>) {
        match self {
            WordIndices::Unbounded(data) => {
                data.push_back(indices);
            }
            WordIndices::Short { starts, ends } => {
                if indices.end >= 64 {
                    let mut unbounded = WordIndices::Unbounded(self.collect());
                    unbounded.push(indices);
                    let _ = std::mem::replace(self, unbounded);
                } else {
                    *starts |= 1 << indices.start;
                    *ends |= 1 << indices.end;
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        match self {
            WordIndices::Unbounded(data) => data.len(),
            WordIndices::Short { starts, .. } => starts.count_ones().try_into().unwrap(),
        }
    }

    pub fn get(&self, i: usize) -> Option<Range<usize>> {
        match self {
            WordIndices::Unbounded(data) => data.get(i).cloned(),
            WordIndices::Short { .. } => self.clone().nth(i),
        }
    }
}

impl Iterator for WordIndices {
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Range<usize>> {
        match self {
            WordIndices::Unbounded(data) => data.pop_front(),
            WordIndices::Short { starts, ends } => {
                if *starts == 0 {
                    return None;
                }
                assert!(*ends > 0);

                let start = starts.trailing_zeros();
                let end = ends.trailing_zeros();

                *starts ^= 1 << start;
                *ends ^= 1 << end;

                Some(start.try_into().unwrap()..end.try_into().unwrap())
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let size = match self {
            WordIndices::Unbounded(data) => data.len(),
            WordIndices::Short { starts, .. } => starts.count_ones().try_into().unwrap(),
        };
        (size, Some(size))
    }
}

impl DoubleEndedIterator for WordIndices {
    fn next_back(&mut self) -> Option<Range<usize>> {
        match self {
            WordIndices::Unbounded(data) => data.pop_back(),
            WordIndices::Short { starts, ends } => {
                if *starts == 0 {
                    return None;
                }
                assert!(*ends > 0);

                let start = 63 - starts.leading_zeros();
                let end = 63 - ends.leading_zeros();

                *starts ^= 1 << start;
                *ends ^= 1 << end;

                Some(start.try_into().unwrap()..end.try_into().unwrap())
            }
        }
    }
}

impl ExactSizeIterator for WordIndices {}
