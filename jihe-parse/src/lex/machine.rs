use std::{cmp::Ordering, collections::HashSet};

use crate::lex::{
    TokenSet,
    automata::Stage,
    token::{Character, PATTERN_COUNT, PATTERNS, Pattern},
};

#[derive(Clone)]
pub(super) struct Machine {
    stages: [Stage; PATTERN_COUNT],
    pub(super) last_matched_char: Option<char>,
}

impl Machine {
    pub(super) fn new() -> Self {
        Self {
            stages: [Stage::init(); _],
            last_matched_char: None,
        }
    }

    pub(super) fn step(&self, c: char) -> Self {
        let mut next = Self::new();
        for ((next_stage, stage), Pattern { automata, .. }) in
            next.stages.iter_mut().zip(self.stages).zip(PATTERNS.iter())
        {
            *next_stage = automata.step(stage, c);
            if !next_stage.is_empty() {
                next.last_matched_char = Some(c);
            }
        }
        next
    }

    pub(super) fn end(&self) -> TokenSet {
        let mut greatest_priority = 0;
        let mut matched = TokenSet::new();
        for (stage, pattern) in self.stages.iter().zip(PATTERNS.iter()) {
            let Pattern { token: kind, priority, automata } = pattern;
            if !automata.is_exit(*stage) {
                continue;
            }
            match priority.cmp(&greatest_priority) {
                Ordering::Greater => {
                    greatest_priority = *priority;
                    matched.clear();
                    matched.insert(*kind);
                }
                Ordering::Equal => {
                    matched.insert(*kind);
                }
                _ => {}
            }
        }
        matched
    }

    pub(super) fn gather_expected(&self) -> HashSet<Character> {
        let mut expected = HashSet::new();
        for (stage, Pattern { automata, .. }) in self.stages.iter().zip(PATTERNS.iter()) {
            expected.extend(automata.expected(*stage));
        }
        expected
    }
}
