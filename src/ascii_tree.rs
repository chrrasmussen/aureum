// Source code copied from the crates.io package: ascii_tree v0.1.1
// Original author: d.maetzke@bpressure.net
// License: MIT

//! Write an ascii tree

use std::fmt;
use std::fmt::Write;

#[derive(Clone)]
pub enum Tree {
    Node(String, Vec<Tree>),
    Leaf(Vec<String>),
}

#[inline]
/// writes a tree in an ascii tree to the writer
pub fn write_tree(f: &mut dyn Write, tree: &Tree) -> fmt::Result {
    write_tree_element(f, tree, &vec![])
}

fn write_tree_element(f: &mut dyn Write, tree: &Tree, level: &Vec<usize>) -> fmt::Result {
    use Tree::*;
    const EMPTY: &str = "   ";
    const EDGE: &str = "└─ ";
    const PIPE: &str = "│  ";
    const BRANCH: &str = "├─ ";

    let maxpos = level.len();
    let mut second_line = String::new();
    for (pos, l) in level.iter().enumerate() {
        let last_row = pos == maxpos - 1;
        if *l == 1 {
            if !last_row {
                write!(f, "{}", EMPTY)?
            } else {
                write!(f, "{}", EDGE)?
            }
            second_line.push_str(EMPTY);
        } else {
            if !last_row {
                write!(f, "{}", PIPE)?
            } else {
                write!(f, "{}", BRANCH)?
            }
            second_line.push_str(PIPE);
        }
    }

    match tree {
        Node(title, children) => {
            let mut d = children.len();
            write!(f, "{}\n", title)?;
            for s in children {
                let mut lnext = level.clone();
                lnext.push(d);
                d -= 1;
                write_tree_element(f, s, &lnext)?;
            }
        }
        Leaf(lines) => {
            for (i, s) in lines.iter().enumerate() {
                match i {
                    0 => writeln!(f, "{}", s)?,
                    _ => writeln!(f, "{}{}", second_line, s)?,
                }
            }
        }
    }

    Ok(())
}
