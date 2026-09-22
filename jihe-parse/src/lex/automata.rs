use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    convert,
    rc::Rc,
};

use crate::lex::token::{Character, Repeat};

#[derive(Debug)]
pub(super) struct Automata {
    map: Vec<HashMap<Character, u8>>,
    exits: IntSet,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Stage(IntSet);

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

    pub(super) fn step(&self, Stage(nodes): Stage, c: char) -> Stage {
        let mut next_nodes = IntSet::new();
        if !nodes.is_empty() {
            for node in nodes {
                if let Some(map) = self.map.get(node as usize) {
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
                .any(|node| self.exits.contains(node).is_some_and(convert::identity))
    }

    pub(super) fn expected(&self, Stage(nodes): Stage) -> HashSet<Character> {
        let mut expected = HashSet::new();
        if !nodes.is_empty() {
            for node in nodes {
                if let Some(map) = self.map.get(node as usize) {
                    expected.extend(map.keys());
                }
            }
        }
        expected
    }
}

impl Stage {
    pub(super) fn init() -> Self {
        Self(IntSet(1))
    }

    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Nondeterminstic {
    fn new(expression: &[(Character, Repeat)]) -> Self {
        let mut node_edges = Vec::new();
        for (ch, rpt) in expression {
            let ch = *ch;
            let curr = node_edges.len();
            match rpt {
                Repeat::ZeroOrMore => {
                    node_edges.push(vec![(curr + 1, None)]);
                    node_edges.push(vec![(curr + 1, Some(ch)), (curr + 2, None)]);
                }
                Repeat::ZeroOrOne => {
                    node_edges.push(vec![(curr + 1, None), (curr + 2, None)]);
                    node_edges.push(vec![(curr + 2, Some(ch))]);
                }
                Repeat::One => {
                    node_edges.push(vec![(curr + 1, Some(ch))]);
                }
                Repeat::OneOrMore => {
                    node_edges.push(vec![(curr + 1, Some(ch))]);
                    node_edges.push(vec![(curr + 1, Some(ch)), (curr + 2, None)]);
                }
            }
        }
        node_edges.push(Vec::new());

        let len = node_edges.len();
        assert!(len <= IntSet::CAPACITY as usize, "Expression is too long");

        let mut epsilon_closures: Vec<Rc<RefCell<Subgraph>>> = Vec::new();
        for (index, node) in node_edges.iter_mut().enumerate().rev() {
            let mut epsilon_closure = Rc::new(RefCell::new({
                let mut subgraph = Subgraph::new();
                subgraph.nodes.insert(index as u8);
                subgraph
            }));
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

    fn is_empty(&self) -> bool {
        self.0 == 0
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
mod test {
    use std::assert_matches;

    use super::*;

    #[test]
    fn test_set_basic() {
        let mut set = IntSet::new();
        assert_matches!(set.insert(32), None);
        assert_matches!(set.insert(31), Some(true));
        assert_matches!(set.insert(31), Some(false));
        assert_matches!(set.insert(0), Some(true));
    }

    #[test]
    fn test_set_iter() {
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

    #[test]
    fn test_one() {
        let expression = &[(Character::Single('0'), Repeat::One)];

        let nondeterminstic = Nondeterminstic::new(expression);
        assert_eq!(nondeterminstic.nodes.len(), 2);

        let edges_0 = &nondeterminstic.nodes[0].edges;
        assert_eq!(edges_0.len(), 1);
        assert_eq!(edges_0[0], (1, Some(Character::Single('0'))));

        let edges_1 = &nondeterminstic.nodes[1].edges;
        assert_eq!(edges_1.len(), 0);

        let subgraph_0 = nondeterminstic.nodes[0].data.borrow();
        assert_eq!(subgraph_0.nodes, IntSet(1));
        assert_eq!(subgraph_0.alphabet.len(), 1);
        assert!(subgraph_0.alphabet.contains(&Character::Single('0')));

        let subgraph_1 = nondeterminstic.nodes[1].data.borrow();
        assert_eq!(subgraph_1.nodes, IntSet(2));
        assert_eq!(subgraph_1.alphabet.len(), 0);

        let automata = Automata::new(expression);
        assert_eq!(automata.map.len(), 2);
        assert_eq!(automata.map[0].len(), 1);
        assert_eq!(automata.map[0].get(&Character::Single('0')), Some(&1));
        assert_eq!(automata.map[1].len(), 0);
        assert_eq!(automata.exits, IntSet(2));
    }

    #[test]
    fn test_one_or_more() {}
}
