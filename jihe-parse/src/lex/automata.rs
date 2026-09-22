use std::collections::{HashMap, HashSet};

use crate::lex::{
    automata::{
        graph::{Determinstic, Nondeterminstic},
        set::IntSet,
    },
    token::{Character, Repeat},
};

mod graph;
mod set;

#[derive(Debug)]
pub(super) struct Automata {
    maps: Vec<HashMap<Character, u8>>,
    exits: IntSet,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Stage(IntSet);

impl Automata {
    pub(super) fn new(expression: &[(Character, Repeat)]) -> Self {
        assert!(!expression.is_empty(), "Expression shouldn't be empty");

        let nondeterminstic = Nondeterminstic::new(expression);
        let determinstic = Determinstic::new(&nondeterminstic);

        let mut maps = vec![HashMap::new(); determinstic.nodes.len()];
        let mut exits = IntSet::new();
        let exit = (nondeterminstic.nodes.len() - 1) as u8;

        for (from_index, (map, node)) in maps.iter_mut().zip(determinstic.nodes).enumerate() {
            for (to_index, ch) in &node.edges {
                map.insert(*ch, *to_index as u8);
            }
            map.shrink_to_fit();
            if matches!(node.data.contains(exit), Some(true)) {
                exits.insert(from_index as u8);
            }
        }
        maps.shrink_to_fit();

        Self { maps, exits }
    }

    pub(super) fn step(&self, Stage(nodes): Stage, c: char) -> Stage {
        let mut next_nodes = IntSet::new();
        if !nodes.is_empty() {
            for node in nodes {
                if let Some(map) = self.maps.get(node as usize) {
                    if let Some(next_node) = map.get(&Character::Single(c)) {
                        next_nodes.insert(*next_node);
                    }
                    if let Some(next_node) = map.get(&c.into()) {
                        next_nodes.insert(*next_node);
                    }
                }
            }
        }
        Stage(next_nodes)
    }

    pub(super) fn is_exit(&self, Stage(nodes): Stage) -> bool {
        !nodes.is_empty()
            && nodes
                .into_iter()
                .any(|node| matches!(self.exits.contains(node), Some(true)))
    }

    pub(super) fn expected(&self, Stage(nodes): Stage) -> HashSet<Character> {
        let mut expected = HashSet::new();
        if !nodes.is_empty() {
            for node in nodes {
                if let Some(map) = self.maps.get(node as usize) {
                    expected.extend(map.keys());
                }
            }
        }
        expected
    }
}

impl Stage {
    pub(super) fn init() -> Self {
        Self(IntSet::with_values(&[0]))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_zero_or_one() {
        let expression = &[
            (Character::Single('0'), Repeat::One),
            (Character::Single('1'), Repeat::ZeroOrOne),
        ];

        let automata = Automata::new(expression);
        assert_eq!(automata.maps.len(), 3);
        assert_eq!(automata.maps[0].len(), 1);
        assert_eq!(automata.maps[0].get(&Character::Single('0')), Some(&1));
        assert_eq!(automata.maps[1].len(), 1);
        assert_eq!(automata.maps[1].get(&Character::Single('1')), Some(&2));
        assert_eq!(automata.maps[2].len(), 0);
        assert_eq!(automata.exits, IntSet::with_values(&[1, 2]));

        let mut stage = Stage::init();
        assert!(!automata.is_exit(automata.step(stage, '1')));
        stage = automata.step(stage, '0');
        assert!(automata.is_exit(stage));
        assert!(!automata.is_exit(automata.step(stage, '0')));
        stage = automata.step(stage, '1');
        assert!(automata.is_exit(stage));
        stage = automata.step(stage, '1');
        assert!(!automata.is_exit(stage));
    }

    #[test]
    fn test_one() {
        let expression = &[(Character::Single('0'), Repeat::One)];

        let automata = Automata::new(expression);
        assert_eq!(automata.maps.len(), 2);
        assert_eq!(automata.maps[0].len(), 1);
        assert_eq!(automata.maps[0].get(&Character::Single('0')), Some(&1));
        assert_eq!(automata.maps[1].len(), 0);
        assert_eq!(automata.exits, IntSet::with_values(&[1]));

        let mut stage = Stage::init();
        assert!(!automata.is_exit(automata.step(stage, '1')));
        stage = automata.step(stage, '0');
        assert!(automata.is_exit(stage));
        stage = automata.step(stage, '0');
        assert!(!automata.is_exit(stage));
    }

    #[test]
    fn test_one_or_more() {
        let expression = &[(Character::Single('0'), Repeat::OneOrMore)];

        let automata = Automata::new(expression);
        assert_eq!(automata.maps.len(), 2);
        assert_eq!(automata.maps[0].len(), 1);
        assert_eq!(automata.maps[0].get(&Character::Single('0')), Some(&1));
        assert_eq!(automata.maps[1].len(), 1);
        assert_eq!(automata.maps[1].get(&Character::Single('0')), Some(&1));
        assert_eq!(automata.exits, IntSet::with_values(&[1]));

        let mut stage = Stage::init();
        assert!(!automata.is_exit(automata.step(stage, '1')));
        stage = automata.step(stage, '0');
        assert!(automata.is_exit(stage));
        stage = automata.step(stage, '0');
        assert!(automata.is_exit(stage));
    }

    #[test]
    fn test_zero_or_more() {
        let expression = &[
            (Character::Single('0'), Repeat::One),
            (Character::Single('1'), Repeat::ZeroOrMore),
        ];

        let automata = Automata::new(expression);
        assert_eq!(automata.maps.len(), 3);
        assert_eq!(automata.maps[0].len(), 1);
        assert_eq!(automata.maps[0].get(&Character::Single('0')), Some(&1));
        assert_eq!(automata.maps[1].len(), 1);
        assert_eq!(automata.maps[1].get(&Character::Single('1')), Some(&2));
        assert_eq!(automata.maps[2].len(), 1);
        assert_eq!(automata.maps[2].get(&Character::Single('1')), Some(&2));
        assert_eq!(automata.exits, IntSet::with_values(&[1, 2]));

        let mut stage = Stage::init();
        assert!(!automata.is_exit(automata.step(stage, '1')));
        stage = automata.step(stage, '0');
        assert!(automata.is_exit(stage));
        assert!(!automata.is_exit(automata.step(stage, '0')));
        stage = automata.step(stage, '1');
        assert!(automata.is_exit(stage));
        stage = automata.step(stage, '1');
        assert!(automata.is_exit(stage));
    }
}
