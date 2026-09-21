use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    convert,
    rc::Rc,
};

use crate::lex::token::{Character, Repeat};

pub(super) struct Automata {
    map: Vec<HashMap<Character, u8>>,
    exits: IntSet,
}

pub(super) type Stage = Option<u8>;

struct Graph<T, E> {
    nodes: Vec<Node<T, E>>,
}

struct Node<T, E> {
    data: T,
    edges: Vec<(usize, E)>,
}

fn node<T, E>(data: T, edges: Vec<(usize, E)>) -> Node<T, E> {
    Node { data, edges }
}

struct Subgraph {
    nodes: IntSet,
    alphabet: HashSet<Character>,
}

impl Subgraph {
    fn new() -> Self {
        Self {
            nodes: IntSet::new(),
            alphabet: HashSet::new(),
        }
    }

    fn merge(&mut self, other: &Self) {
        self.nodes.extend(other.nodes);
        self.alphabet.extend(&other.alphabet);
    }
}

type Nondeterminstic = Graph<Rc<RefCell<Subgraph>>, Option<Character>>; // data = epsilon closure
type Determinstic = Graph<IntSet, Character>; // data = n state indices contained in this d state

impl Automata {
    pub(super) fn new(expression: &[(Character, Repeat)]) -> Automata {
        assert!(!expression.is_empty(), "Expression shouldn't be empty");

        let nondeterminstic = Nondeterminstic::new(expression);
        let epsilon_closure_0 = nondeterminstic.nodes[0].data.borrow();
        let mut determinstic = Determinstic {
            nodes: vec![node(epsilon_closure_0.nodes, Vec::new())],
        };
        let mut rev_determinstic = HashMap::new();
        rev_determinstic.insert(epsilon_closure_0.nodes, 0);

        let mut queue = epsilon_closure_0
            .alphabet
            .iter()
            .map(|trans| (0, *trans))
            .collect::<VecDeque<_>>();
        while let Some((from_d_node_index, trans)) = queue.pop_front() {
            let mut to_subgraph = Subgraph::new();
            for from_n_node in determinstic.nodes[from_d_node_index].data.into_iter() {
                to_subgraph.merge(&nondeterminstic.reachables(from_n_node, trans));
            }

            let to_d_node_index = rev_determinstic
                .get(&to_subgraph.nodes)
                .copied()
                .unwrap_or_else(|| {
                    let index = determinstic.nodes.len();
                    determinstic.nodes.push(node(to_subgraph.nodes, Vec::new()));
                    rev_determinstic.insert(to_subgraph.nodes, index);
                    queue.extend(to_subgraph.alphabet.iter().map(|e| (index, *e)));
                    index
                });
            determinstic.nodes[from_d_node_index]
                .edges
                .push((to_d_node_index, trans));
        }

        let mut map = vec![HashMap::new(); determinstic.nodes.len()];
        let mut exits = IntSet::new();
        let exit = (nondeterminstic.nodes.len() - 1) as u8;

        for (from_index, node) in determinstic.nodes.iter().enumerate() {
            for (to_index, ch) in &node.edges {
                map[from_index].insert(*ch, *to_index as u8);
            }
            if node.data.contains(exit).is_some_and(convert::identity) {
                exits.insert(from_index as u8);
            }
        }

        Self { map, exits }
    }

    pub(super) fn step(&self, stage: Stage, c: char) -> Stage {
        match stage {
            None => stage,
            Some(stage) => match self.map.get(stage as usize) {
                None => None,
                Some(map) => match map.get(&c.into()) {
                    None => None,
                    Some(stage) => Some(*stage),
                },
            },
        }
    }

    pub(super) fn is_exit(&self, stage: Stage) -> bool {
        match stage {
            None => false,
            Some(stage) => self.exits.contains(stage).is_some_and(convert::identity),
        }
    }

    pub(super) fn expected(&self, stage: Stage) -> HashSet<Character> {
        match stage {
            None => HashSet::new(),
            Some(stage) => match self.map.get(stage as usize) {
                None => HashSet::new(),
                Some(map) => map.keys().copied().collect(),
            },
        }
    }
}

impl Nondeterminstic {
    fn new(expression: &[(Character, Repeat)]) -> Self {
        let mut node_edges = Vec::new();
        for (ch, rpt) in expression {
            let ch = *ch;
            let curr = node_edges.len();
            match rpt {
                Repeat::Any => {
                    node_edges.push(vec![(curr + 1, None)]);
                    node_edges.push(vec![(curr + 1, Some(ch)), (curr + 2, None)]);
                }
                Repeat::NoneOrOnce => {
                    node_edges.push(vec![(curr + 1, None), (curr + 2, None)]);
                    node_edges.push(vec![(curr + 2, Some(ch))]);
                }
                Repeat::Once => {
                    node_edges.push(vec![(curr + 1, Some(ch))]);
                }
                Repeat::OnceOrMultiple => {
                    node_edges.push(vec![(curr + 1, Some(ch))]);
                    node_edges.push(vec![(curr + 1, Some(ch)), (curr + 2, None)]);
                }
            }
        }
        node_edges.push(Vec::new());

        let len = node_edges.len();
        assert!(len <= IntSet::CAPACITY as usize, "Expression is too long");

        let mut epsilon_closures: Vec<Rc<RefCell<Subgraph>>> = Vec::new();
        for node in node_edges.iter_mut().rev() {
            let mut epsilon_closure = Rc::new(RefCell::new(Subgraph::new()));
            for edge in node {
                match edge.1 {
                    Some(ch) => {
                        epsilon_closure.borrow_mut().alphabet.insert(ch);
                    }
                    None => {
                        let to = &epsilon_closures
                            .get(len - 1 - edge.0)
                            .expect("Edges could never point backwards");
                        to.borrow_mut().merge(&epsilon_closure.borrow());
                        epsilon_closure = Rc::clone(to);
                    }
                }
            }
            epsilon_closures.push(epsilon_closure);
        }
        epsilon_closures.reverse();

        let nodes = epsilon_closures
            .into_iter()
            .zip(node_edges)
            .map(|(data, edges)| node(data, edges))
            .collect();

        Self { nodes }
    }

    fn reachables(&self, index: u8, transition: Character) -> Subgraph {
        let mut subgraph = Subgraph::new();
        for (to_index, trans) in &self.nodes[index as usize].edges {
            if trans.is_some_and(|trans| trans == transition) {
                let eps = &self.nodes[*to_index].data;
                subgraph.merge(&eps.borrow());
            }
        }
        subgraph
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct IntSet(u32);

impl IntSet {
    const CAPACITY: u8 = u32::BITS as u8;

    fn new() -> Self {
        Self(0)
    }

    fn clear(&mut self) {
        self.0 = 0;
    }

    fn insert(&mut self, value: u8) -> Option<bool> {
        if value >= Self::CAPACITY {
            None
        } else {
            let value = 1 << value;
            if self.0 & value == 0 {
                self.0 |= value;
                Some(true)
            } else {
                Some(false)
            }
        }
    }

    fn contains(&self, value: u8) -> Option<bool> {
        if value >= Self::CAPACITY {
            None
        } else {
            Some(self.0 & (1 << value) != 0)
        }
    }
}

impl Extend<u8> for IntSet {
    fn extend<T: IntoIterator<Item = u8>>(&mut self, iter: T) {
        for value in iter {
            self.insert(value);
        }
    }
}

impl<'a> Extend<&'a u8> for IntSet {
    fn extend<T: IntoIterator<Item = &'a u8>>(&mut self, iter: T) {
        for value in iter {
            self.insert(*value);
        }
    }
}

impl IntoIterator for IntSet {
    type Item = u8;
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter(self.0, 0)
    }
}

struct IntoIter(u32, u8);

impl Iterator for IntoIter {
    type Item = u8;

    fn next(&mut self) -> Option<Self::Item> {
        while let 0 = self.0 & 1 {
            if self.0 == 0 {
                return None;
            }
            self.0 >>= 1;
            self.1 += 1;
        }
        let result = Some(self.1);
        self.0 >>= 1;
        self.1 += 1;
        result
    }
}

#[cfg(test)]
mod set_test {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_basic() {
        let mut set = IntSet::new();
        assert_matches!(set.insert(32), None);
        assert_matches!(set.insert(31), Some(true));
        assert_matches!(set.insert(31), Some(false));
        set.clear();
        assert_matches!(set.insert(0), Some(true));
    }

    #[test]
    fn test_iter() {
        let mut set = IntSet::new();
        set.insert(0);
        set.insert(9);
        set.insert(7);
        set.insert(6);
        set.extend([9, 4, 32]);
        let mut iter = set.into_iter();
        assert_matches!(iter.next(), Some(0));
        assert_matches!(iter.next(), Some(4));
        assert_matches!(iter.next(), Some(6));
        assert_matches!(iter.next(), Some(7));
        assert_matches!(iter.next(), Some(9));
        assert_matches!(iter.next(), None);
    }
}
