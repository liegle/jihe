use std::{cmp::Ordering, collections::HashSet};

use crate::token::{Character, Kind, PATTERNS, Pattern};

#[derive(Clone, Copy, Debug)]
enum Stage {
    Matching { index: usize, count: usize },
    Out,
}

#[derive(Clone)]
pub(super) struct Match {
    stages: [Stage; PATTERNS.len()],
    matching_count: usize,
}

impl Match {
    pub(super) fn new() -> Self {
        Self {
            stages: [Stage::Matching { index: 0, count: 0 }; _],
            matching_count: 0,
        }
    }

    pub(super) fn any(&self) -> bool {
        self.matching_count != 0
    }

    pub(super) fn step(&mut self, c: char) {
        self.matching_count = 0;
        for (stage, Pattern { expression, .. }) in self.stages.iter_mut().zip(PATTERNS) {
            *stage = if let Stage::Matching { index, count } = *stage {
                // Try to consume as many chars in one sub pattern as possible
                if let Some((character, repeat)) = expression.get(index)
                    && character.contains(c)
                    && repeat.accepts(count + 1)
                {
                    Stage::Matching { index, count: count + 1 }
                } else {
                    let mut windows = expression[index..].windows(2).enumerate();
                    let mut prev_count = count;
                    loop {
                        if let Some((index_add, [(_, prev_repeat), (curr_character, _)])) =
                            windows.next()
                        {
                            if !prev_repeat.accepts(prev_count) {
                                break Stage::Out;
                            }
                            prev_count = 0;
                            // 1 must be accepted
                            if curr_character.contains(c) {
                                break Stage::Matching {
                                    index: index + index_add + 1,
                                    count: 1,
                                };
                            }
                        } else {
                            break Stage::Out;
                        }
                    }
                }
            } else {
                Stage::Out
            };
            if let Stage::Matching { .. } = stage {
                self.matching_count += 1;
            }
        }
    }

    pub(super) fn cmp_priority(&self) -> Vec<Kind> {
        let mut greatest_priority = 0;
        let mut matched = Vec::new();
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
