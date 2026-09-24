use std::collections::{HashMap, HashSet, VecDeque};

use crate::lex::{
    automata::IntSet,
    token::{Character, Repeat},
};

#[derive(Debug)]
pub(super) struct Graph<T, E> {
    pub(super) nodes: Vec<Node<T, E>>,
}

#[derive(Debug)]
pub(super) struct Node<T, E> {
    pub(super) data: T,
    pub(super) edges: Vec<(usize, E)>,
}

#[derive(Debug)]
pub(super) struct Subgraph {
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

pub(super) type Nondeterminstic = Graph<Subgraph, Option<Character>>; // data = epsilon closure
pub(super) type Determinstic = Graph<IntSet, Character>; // data = n state indices contained in this d state

fn thompson(expression: &[(Character, Repeat)]) -> Vec<Vec<(usize, Option<Character>)>> {
    let mut node_edges = Vec::new();
    for (ch, rpt) in expression {
        let ch = *ch;
        let curr = node_edges.len();
        match rpt {
            Repeat::ZeroOrOne => {
                // curr -e-> +1 -c-> +2
                //   \                ^
                //    \_______e______/
                node_edges.push(vec![(curr + 1, None), (curr + 2, None)]);
                node_edges.push(vec![(curr + 2, Some(ch))]);
            }
            Repeat::One => {
                // curr -e-> +1
                node_edges.push(vec![(curr + 1, Some(ch))]);
            }
            Repeat::OneOrMore => {
                // curr -c-> +1 -e-> +2
                //          /  ^
                //         /__c_\
                node_edges.push(vec![(curr + 1, Some(ch))]);
                node_edges.push(vec![(curr + 1, Some(ch)), (curr + 2, None)]);
            }
            Repeat::ZeroOrMore => {
                // curr -e-> +1 -e-> +2
                //          /  ^
                //         /__c_\
                node_edges.push(vec![(curr + 1, None)]);
                node_edges.push(vec![(curr + 1, Some(ch)), (curr + 2, None)]);
            }
        }
    }
    node_edges.push(Vec::new());
    node_edges
}

impl Nondeterminstic {
    pub(super) fn new(expression: &[(Character, Repeat)]) -> Self {
        let node_edges = thompson(expression);

        let len = node_edges.len();
        assert!(len <= IntSet::CAPACITY as usize, "Expression is too long");

        let mut epsilon_closures: Vec<Subgraph> = Vec::new();
        for (index, node) in node_edges.iter().enumerate().rev() {
            let mut epsilon_closure = Subgraph {
                nodes: IntSet::with_values([index as u8]),
                alphabet: HashSet::new(),
            };
            for edge in node {
                if let Some(ch) = edge.1 {
                    epsilon_closure.alphabet.insert(ch);
                } else {
                    epsilon_closure.merge(&epsilon_closures[len - 1 - edge.0]);
                }
            }
            epsilon_closures.push(epsilon_closure);
        }
        epsilon_closures.reverse();

        let nodes = epsilon_closures
            .into_iter()
            .zip(node_edges)
            .map(|(data, edges)| Node { data, edges })
            .collect();

        Self { nodes }
    }

    fn reachables(&self, index: u8, transition: Character) -> Subgraph {
        let mut subgraph = Subgraph::new();
        for (to_index, trans) in &self.nodes[index as usize].edges {
            if trans.is_some_and(|trans| trans == transition) {
                subgraph.merge(&self.nodes[*to_index].data);
            }
        }
        subgraph
    }
}

impl Determinstic {
    pub(super) fn new(nondeterminstic: &Nondeterminstic) -> Self {
        let epsilon_closure_0 = &nondeterminstic.nodes[0].data;
        let mut determinstic = Determinstic {
            nodes: vec![Node {
                data: epsilon_closure_0.nodes,
                edges: Vec::new(),
            }],
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
                    determinstic.nodes.push(Node {
                        data: to_subgraph.nodes,
                        edges: Vec::new(),
                    });
                    rev_determinstic.insert(to_subgraph.nodes, index);
                    queue.extend(to_subgraph.alphabet.iter().map(|e| (index, *e)));
                    index
                });
            determinstic.nodes[from_d_node_index]
                .edges
                .push((to_d_node_index, trans));
        }
        determinstic
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

        let nondeterminstic = Nondeterminstic::new(expression);
        assert_eq!(nondeterminstic.nodes.len(), 4);

        let edges_0 = &nondeterminstic.nodes[0].edges;
        assert_eq!(edges_0.len(), 1);
        assert_eq!(edges_0[0], (1, Some(Character::Single('0'))));

        let edges_1 = &nondeterminstic.nodes[1].edges;
        assert_eq!(edges_1.len(), 2);
        assert_eq!(edges_1[0], (2, None));
        assert_eq!(edges_1[1], (3, None));

        let edges_2 = &nondeterminstic.nodes[2].edges;
        assert_eq!(edges_2.len(), 1);
        assert_eq!(edges_2[0], (3, Some(Character::Single('1'))));

        let edges_3 = &nondeterminstic.nodes[3].edges;
        assert_eq!(edges_3.len(), 0);

        let subgraph_0 = &nondeterminstic.nodes[0].data;
        assert_eq!(subgraph_0.nodes, IntSet::with_values([0]));
        assert_eq!(subgraph_0.alphabet.len(), 1);
        assert!(subgraph_0.alphabet.contains(&Character::Single('0')));

        let subgraph_1 = &nondeterminstic.nodes[1].data;
        assert_eq!(subgraph_1.nodes, IntSet::with_values([1, 2, 3]));
        assert_eq!(subgraph_1.alphabet.len(), 1);
        assert!(subgraph_1.alphabet.contains(&Character::Single('1')));

        let subgraph_2 = &nondeterminstic.nodes[2].data;
        assert_eq!(subgraph_2.nodes, IntSet::with_values([2]));
        assert_eq!(subgraph_2.alphabet.len(), 1);
        assert!(subgraph_2.alphabet.contains(&Character::Single('1')));

        let subgraph_3 = &nondeterminstic.nodes[3].data;
        assert_eq!(subgraph_3.nodes, IntSet::with_values([3]));
        assert_eq!(subgraph_3.alphabet.len(), 0);
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

        let subgraph_0 = &nondeterminstic.nodes[0].data;
        assert_eq!(subgraph_0.nodes, IntSet::with_values([0]));
        assert_eq!(subgraph_0.alphabet.len(), 1);
        assert!(subgraph_0.alphabet.contains(&Character::Single('0')));

        let subgraph_1 = &nondeterminstic.nodes[1].data;
        assert_eq!(subgraph_1.nodes, IntSet::with_values([1]));
        assert_eq!(subgraph_1.alphabet.len(), 0);
    }

    #[test]
    fn test_one_or_more() {
        let expression = &[(Character::Single('0'), Repeat::OneOrMore)];

        let nondeterminstic = Nondeterminstic::new(expression);
        assert_eq!(nondeterminstic.nodes.len(), 3);

        let edges_0 = &nondeterminstic.nodes[0].edges;
        assert_eq!(edges_0.len(), 1);
        assert_eq!(edges_0[0], (1, Some(Character::Single('0'))));

        let edges_1 = &nondeterminstic.nodes[1].edges;
        assert_eq!(edges_1.len(), 2);
        assert_eq!(edges_1[0], (1, Some(Character::Single('0'))));
        assert_eq!(edges_1[1], (2, None));

        let edges_2 = &nondeterminstic.nodes[2].edges;
        assert_eq!(edges_2.len(), 0);

        let subgraph_0 = &nondeterminstic.nodes[0].data;
        assert_eq!(subgraph_0.nodes, IntSet::with_values([0]));
        assert_eq!(subgraph_0.alphabet.len(), 1);
        assert!(subgraph_0.alphabet.contains(&Character::Single('0')));

        let subgraph_1 = &nondeterminstic.nodes[1].data;
        assert_eq!(subgraph_1.nodes, IntSet::with_values([1, 2]));
        assert_eq!(subgraph_1.alphabet.len(), 1);
        assert!(subgraph_1.alphabet.contains(&Character::Single('0')));

        let subgraph_2 = &nondeterminstic.nodes[2].data;
        assert_eq!(subgraph_2.nodes, IntSet::with_values([2]));
        assert_eq!(subgraph_2.alphabet.len(), 0);
    }

    #[test]
    fn test_zero_or_more() {
        let expression = &[
            (Character::Single('0'), Repeat::One),
            (Character::Single('1'), Repeat::ZeroOrMore),
        ];

        let nondeterminstic = Nondeterminstic::new(expression);
        assert_eq!(nondeterminstic.nodes.len(), 4);

        let edges_0 = &nondeterminstic.nodes[0].edges;
        assert_eq!(edges_0.len(), 1);
        assert_eq!(edges_0[0], (1, Some(Character::Single('0'))));

        let edges_1 = &nondeterminstic.nodes[1].edges;
        assert_eq!(edges_1.len(), 1);
        assert_eq!(edges_1[0], (2, None));

        let edges_2 = &nondeterminstic.nodes[2].edges;
        assert_eq!(edges_2.len(), 2);
        assert_eq!(edges_2[0], (2, Some(Character::Single('1'))));
        assert_eq!(edges_2[1], (3, None));

        let edges_3 = &nondeterminstic.nodes[3].edges;
        assert_eq!(edges_3.len(), 0);

        let subgraph_0 = &nondeterminstic.nodes[0].data;
        assert_eq!(subgraph_0.nodes, IntSet::with_values([0]));
        assert_eq!(subgraph_0.alphabet.len(), 1);
        assert!(subgraph_0.alphabet.contains(&Character::Single('0')));

        let subgraph_1 = &nondeterminstic.nodes[1].data;
        assert_eq!(subgraph_1.nodes, IntSet::with_values([1, 2, 3]));
        assert_eq!(subgraph_1.alphabet.len(), 1);
        assert!(subgraph_1.alphabet.contains(&Character::Single('1')));

        let subgraph_2 = &nondeterminstic.nodes[2].data;
        assert_eq!(subgraph_2.nodes, IntSet::with_values([2, 3]));
        assert_eq!(subgraph_2.alphabet.len(), 1);
        assert!(subgraph_2.alphabet.contains(&Character::Single('1')));

        let subgraph_3 = &nondeterminstic.nodes[3].data;
        assert_eq!(subgraph_3.nodes, IntSet::with_values([3]));
        assert_eq!(subgraph_3.alphabet.len(), 0);
    }
}
