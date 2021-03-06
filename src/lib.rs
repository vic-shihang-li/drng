//! A disjoint range implementation.

#![warn(missing_docs)]

#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

mod range;
pub use range::{range_compare, Range, RangeCompareResult, SimpleRange};

/// An explanation of why an overlap error occurred while inserting to a
/// [`DisjointRange`].
#[allow(dead_code)]
#[derive(Debug)]
pub struct OverlapError {
    kind: RangeCompareResult,
}

/// Possible errors when inserting a range to a [`DisjointRange`].
#[derive(Debug)]
pub enum AddError {
    /// Insertion erred because the range to insert overlaps with an existing
    /// range in [`DisjointRange`].
    OverlapRange(OverlapError),
    /// Insertion failed because the provided parameters does not form a vaild
    /// range (e.g., when the start value is greater than the end value).
    BadRange,
}

/// An enumeration of inclusive / exclusiveness on both ends of a range.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum RangeMode {
    /// Range where the start and end are inclusive (i.e., [start..end]).
    Inclusive,
    /// Range where the start and end are exclusive (i.e., (start..end)).
    Exclusive,
    /// Range where the start is exclusive and the end is inclusive
    /// (i.e., (start..end]).
    StartExclusive,
    /// Range where the start is inclusive and the end is exclusive
    /// (i.e., [start..end)).
    EndExclusive,
}

/// A data structure holding a disjoint set of ranges.
///
/// The following code snippet represents a set of ranges:
///
/// ```rust
/// use drng::{DisjointRange, RangeMode};
/// let mut dr = DisjointRange::new(RangeMode::Inclusive);
///
/// // not drawn to scale
/// // 0.......[35..90][91..110]..[115, 120]
///
/// dr.add(35, 90).unwrap();
/// dr.add(91, 110).unwrap();
/// dr.add(115, 120).unwrap();
///
/// assert!(dr.includes(35));
/// assert!(!dr.includes(113));
/// ```
pub struct DisjointRange {
    inner: DisjointRangeInner<SimpleRange>,
    mode: RangeMode,
}

impl DisjointRange {
    /// Constructs a new [`DisjointRange`].
    pub fn new(mode: RangeMode) -> Self {
        Self {
            inner: DisjointRangeInner::default(),
            mode,
        }
    }

    /// Adds a range to this [`DisjointRange`].
    pub fn add(&mut self, min: usize, max: usize) -> Result<(), AddError> {
        match SimpleRange::new(min, max, self.mode) {
            Err(_) => Err(AddError::BadRange),
            Ok(r) => self.inner.add(r),
        }
    }

    /// Checks whether the given value falls into one of the ranges.
    pub fn includes(&self, value: usize) -> bool {
        self.inner.includes(value)
    }

    /// Checks if the data structure contains the provided value.
    ///
    /// If yes, return the range that contains this value.
    pub fn lookup(&self, value: usize) -> Option<&SimpleRange> {
        self.inner.lookup(value)
    }

    /// Iterates this [`DisjointRange`] in range-ascending order.
    pub fn iter(&self) -> DisjointRangeIter<'_, SimpleRange> {
        self.inner.iter()
    }
}

struct DisjointRangeInner<R: Range> {
    ranges: Vec<R>,
}

impl<R: Range> Default for DisjointRangeInner<R> {
    fn default() -> Self {
        Self { ranges: Vec::new() }
    }
}

impl<R: Range> DisjointRangeInner<R> {
    fn add(&mut self, range: R) -> Result<(), AddError> {
        for (i, range_curr) in self.ranges.iter().enumerate() {
            match range_compare(&range, range_curr) {
                RangeCompareResult::GreaterNoOverlap => continue,
                RangeCompareResult::LessThanNoOverlap => {
                    self.ranges.insert(i, range);
                    return Ok(());
                }
                r => return Err(AddError::OverlapRange(OverlapError { kind: r })),
            }
        }

        self.ranges.push(range);

        Ok(())
    }

    fn includes(&self, value: usize) -> bool {
        self.lookup(value).is_some()
    }

    fn lookup(&self, value: usize) -> Option<&R> {
        self.ranges.iter().find(|r| r.includes(value))
    }

    fn iter(&self) -> DisjointRangeIter<'_, R> {
        DisjointRangeIter::new(self)
    }
}

/// Iterates a [`DisjointRange`].
pub struct DisjointRangeIter<'a, R: Range> {
    dr: &'a DisjointRangeInner<R>,
    curr: usize,
}

impl<'a, R: Range> DisjointRangeIter<'a, R> {
    fn new(dr: &'a DisjointRangeInner<R>) -> DisjointRangeIter<'a, R> {
        Self { dr, curr: 0 }
    }
}

impl<'a, R: Range> Iterator for DisjointRangeIter<'a, R> {
    type Item = &'a R;

    fn next(&mut self) -> Option<Self::Item> {
        match self.dr.ranges.get(self.curr) {
            Some(range) => {
                self.curr += 1;
                Some(range)
            }
            None => None,
        }
    }
}

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};
#[cfg(test)]
use rand::Rng;

#[cfg(test)]
impl Arbitrary for RangeMode {
    fn arbitrary(_g: &mut Gen) -> RangeMode {
        let num: usize = rand::thread_rng().gen_range(0..3);
        match num {
            0 => RangeMode::Inclusive,
            1 => RangeMode::Exclusive,
            2 => RangeMode::StartExclusive,
            3 => RangeMode::EndExclusive,
            _ => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, TestResult};
    use rand::{thread_rng, Rng};

    #[quickcheck]
    fn prop_add_valid_range(min: usize, max: usize) -> TestResult {
        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        match dr.add(min, max) {
            Ok(_) => TestResult::from_bool(min <= max),
            Err(_) => TestResult::discard(),
        }
    }

    #[quickcheck]
    fn prop_add_valid_ranges(ranges: Vec<(usize, usize)>) -> TestResult {
        if ranges.len() < 10_000 {
            return TestResult::discard();
        }

        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        for (min, max) in ranges {
            if dr.add(min, max).is_err() {
                return TestResult::discard();
            }
        }

        TestResult::from_bool(
            dr.iter()
                .zip(dr.iter().skip(1))
                .all(|(prev, curr)| curr.min() > prev.max()),
        )
    }

    #[quickcheck]
    fn prop_find_range_value(min: usize, max: usize) -> TestResult {
        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        match dr.add(min, max) {
            Err(_) => TestResult::discard(),
            Ok(_) => {
                if max == usize::MAX {
                    return TestResult::discard();
                }
                const NUM_SAMPLES: usize = 100;
                let mut rng = thread_rng();
                TestResult::from_bool(
                    (0..NUM_SAMPLES)
                        .map(|_| rng.gen_range(min..(max + 1)))
                        .all(|x| dr.lookup(x).is_some()),
                )
            }
        }
    }

    #[test]
    fn create_basic_dr() {
        let sorted_sequence = [
            [100, 200],
            [222, 235],
            [20000, 22322],
            [34330, 50000],
            [60000, 700000],
        ];
        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        insert_ranges(&mut dr, &sorted_sequence);
        assert_range_sequence(&dr, &sorted_sequence);
    }

    #[test]
    fn add_overlapping_range_errs() {
        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        dr.add(100, 200).unwrap();
        assert_insert_overlaps(dr.add(90, 111), RangeCompareResult::OverlapLower);
        assert_insert_overlaps(dr.add(100, 123), RangeCompareResult::Contained);
        assert_insert_overlaps(dr.add(140, 160), RangeCompareResult::Contained);
        assert_insert_overlaps(dr.add(188, 200), RangeCompareResult::Contained);
        assert_insert_overlaps(dr.add(100, 200), RangeCompareResult::Equal);
        assert_insert_overlaps(dr.add(100, 300), RangeCompareResult::Contains);
        assert_insert_overlaps(dr.add(73, 2000), RangeCompareResult::Contains);
        assert_insert_overlaps(dr.add(180, 222), RangeCompareResult::OverlapUpper);
        assert_insert_overlaps(dr.add(200, 222), RangeCompareResult::OverlapUpper);
    }

    #[test]
    fn add_invalid_inclusive_range() {
        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        assert_add_bad_range_errs(dr.add(100, 80));
        assert!(dr.add(50, 50).is_ok());
    }

    #[test]
    fn add_invalid_end_exclusive_range() {
        let mut dr = DisjointRange::new(RangeMode::EndExclusive);
        assert!(dr.add(9, 10).is_ok());
        assert_add_bad_range_errs(dr.add(100, 80));
        assert_add_bad_range_errs(dr.add(50, 50));
    }

    #[test]
    fn add_invalid_start_exclusive_range() {
        let mut dr = DisjointRange::new(RangeMode::StartExclusive);
        assert!(dr.add(9, 10).is_ok());
        assert_add_bad_range_errs(dr.add(100, 80));
        assert_add_bad_range_errs(dr.add(50, 50));
    }

    #[test]
    fn add_invalid_exclusive_range() {
        let mut dr = DisjointRange::new(RangeMode::Exclusive);
        assert_add_bad_range_errs(dr.add(100, 80));
        assert_add_bad_range_errs(dr.add(50, 50));
        assert_add_bad_range_errs(dr.add(49, 50));
        assert!(dr.add(48, 50).is_ok());
    }

    #[test]
    fn add_unordered_ranges() {
        let mut dr = DisjointRange::new(RangeMode::Inclusive);
        insert_ranges(
            &mut dr,
            &[
                [100, 200],
                [30, 50],
                [1, 2],
                [60, 66],
                [500, 555],
                [343, 444],
            ],
        );
        assert_range_sequence(
            &dr,
            &[
                [1, 2],
                [30, 50],
                [60, 66],
                [100, 200],
                [343, 444],
                [500, 555],
            ],
        );
    }

    #[test]
    fn end_exclusive_dr() {
        let mut dr = DisjointRange::new(RangeMode::EndExclusive);
        insert_ranges(
            &mut dr,
            &[
                [100, 200],
                [99, 100],
                [98, 99],
                [30, 50],
                [50, 70],
                [70, 90],
                [211, 212],
                [209, 211],
            ],
        );
        assert_range_sequence(
            &dr,
            &[
                [30, 50],
                [50, 70],
                [70, 90],
                [98, 99],
                [99, 100],
                [100, 200],
                [209, 211],
                [211, 212],
            ],
        );
    }

    #[test]
    fn find_value_in_range() {
        let mut dr = DisjointRange::new(RangeMode::EndExclusive);

        dr.add(100, 200).unwrap();
        assert!(dr.includes(100));
        assert!(dr.includes(105));
        assert!(dr.includes(199));
        assert!(!dr.includes(200));
        assert!(!dr.includes(201));
        assert!(!dr.includes(3));
        assert!(!dr.includes(30000));
    }

    #[test]
    fn lookup_range() {
        let mut dr = DisjointRange::new(RangeMode::EndExclusive);

        insert_ranges(&mut dr, &[[100, 200], [300, 4500]]);

        let found_fst = dr.lookup(199).unwrap();
        assert_range_min_max(found_fst, 100, 200);

        assert!(dr.lookup(200).is_none());
        assert!(dr.lookup(248).is_none());

        let found_snd = dr.lookup(344).unwrap();
        assert_range_min_max(found_snd, 300, 4500);
    }

    fn insert_ranges(dr: &mut DisjointRange, seq: &[[usize; 2]]) {
        seq.iter().for_each(|[min, max]| {
            dr.add(*min, *max)
                .unwrap_or_else(|_| panic!("failed to insert min: {}, max: {}", min, max));
        })
    }

    fn assert_range_sequence(dr: &DisjointRange, seq: &[[usize; 2]]) {
        let mut dr_iter = dr.iter();
        let mut deq_iter = seq.iter();

        loop {
            match dr_iter.next() {
                None => match deq_iter.next() {
                    None => break,
                    Some(r) => panic!(
                        "Reached disjoint range end while the expected \
                               next range is {:?}",
                        r
                    ),
                },
                Some(got) => match deq_iter.next() {
                    None => panic!(
                        "Disjoint range longer than expected; \
                                   got {:?} while expected sequence ended",
                        got
                    ),
                    Some([expected_min, expected_max]) => {
                        assert_range_min_max(got, *expected_min, *expected_max)
                    }
                },
            }
        }
    }

    fn assert_range_min_max(range: &dyn Range, min: usize, max: usize) {
        assert_eq!(range.min(), min);
        assert_eq!(range.max(), max);
    }

    fn assert_add_bad_range_errs(add_result: Result<(), AddError>) {
        match add_result {
            Ok(_) => panic!("add bad range should fail"),
            Err(e) => match e {
                AddError::BadRange => (),
                _ => panic!("expected bad range, got {:?}", e),
            },
        }
    }

    fn assert_insert_overlaps(add_result: Result<(), AddError>, kind: RangeCompareResult) {
        match add_result {
            Ok(_) => panic!("expected add to fail"),
            Err(e) => match e {
                AddError::OverlapRange(x) => assert_eq!(x.kind, kind),
                _ => panic!("expected overlap range error, got {:?}", e),
            },
        }
    }
}
