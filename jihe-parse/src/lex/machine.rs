use std::{cmp::Ordering, collections::HashSet};

use crate::{
    array::DynArray,
    token::{Character, Kind, PATTERN_COUNT, PATTERNS, Pattern, Repeat},
};

#[derive(Clone, Copy, Debug)]
enum Stage {
    Matching { index: usize, count: usize },
    Out,
}

impl Stage {
    fn step(&self, expression: &[(Character, Repeat)], c: char) -> Self {
        if let Stage::Matching { index, count } = *self {
            // Try to consume as many chars in one sub pattern as possible
            if let Some((character, repeat)) = expression.get(index)
                && character.contains(c)
                && repeat.accepts(count + 1)
            {
                return Stage::Matching { index, count: count + 1 };
            }

            let mut windows = expression[index..].windows(2).enumerate();
            let mut prev_count = count;

            while let Some((index_add, [(_, prev_repeat), (curr_character, _)])) = windows.next() {
                if !prev_repeat.accepts(prev_count) {
                    return Stage::Out;
                }
                prev_count = 0;
                // 1 must be accepted
                if curr_character.contains(c) {
                    return Stage::Matching {
                        index: index + index_add + 1,
                        count: 1,
                    };
                }
            }
        }

        Stage::Out
    }
}

#[derive(Clone)]
pub(super) struct Machine {
    stages: [Stage; PATTERN_COUNT],
    pub(super) last_matched_char: Option<char>,
}

impl Machine {
    pub(super) fn new() -> Self {
        Self {
            stages: [Stage::Matching { index: 0, count: 0 }; _],
            last_matched_char: None,
        }
    }

    pub(super) fn step(&self, c: char) -> Self {
        let mut next = Self::new();
        for ((next_stage, stage), Pattern { expression, .. }) in
            next.stages.iter_mut().zip(self.stages).zip(PATTERNS)
        {
            *next_stage = stage.step(expression, c);
            if let Stage::Matching { .. } = next_stage {
                next.last_matched_char = Some(c);
            }
        }
        next
    }

    pub(super) fn end(&self) -> DynArray<Kind, PATTERN_COUNT> {
        let mut greatest_priority = 0;
        let mut matched = DynArray::new();
        for (
            stage,
            Pattern {
                kind, priority, endable_index, ..
            },
        ) in self.stages.iter().zip(PATTERNS)
        {
            if !matches!(stage, Stage::Matching { index, .. } if *index >= *endable_index) {
                continue;
            }
            match priority.cmp(&greatest_priority) {
                Ordering::Greater => {
                    greatest_priority = *priority;
                    matched.clear();
                    matched.push(*kind);
                }
                Ordering::Equal => {
                    matched.push(*kind);
                }
                _ => {}
            }
        }
        matched
    }

    pub(super) fn gather_expected(&self) -> HashSet<Character> {
        let mut expected = HashSet::new();
        for (stage, Pattern { expression, .. }) in self.stages.iter().zip(PATTERNS) {
            let Stage::Matching { index, count } = *stage else {
                continue;
            };
            if let Some((character, repeat)) = expression.get(index)
                && repeat.accepts(count + 1)
            {
                expected.insert(*character);
            }
            let mut windows = expression[index..].windows(2);
            let mut prev_count = count;
            while let Some([(_, prev_repeat), (curr_character, _)]) = windows.next() {
                if prev_repeat.accepts(prev_count) {
                    expected.insert(*curr_character);
                }
                prev_count = 0;
            }
        }
        expected
    }
}
