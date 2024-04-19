use loaned::{take, LoanedMut};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum Tree {
  #[default]
  Null,
  Leaf(i32),
  Node(Box<Tree>, Box<Tree>),
}

#[cfg_attr(test, test)]
fn main() {
  // First, we create a single node `a`, which has two holes, `b` and `c`.
  // ```text
  //    a
  //   / \
  // ?b   ?c
  // ```
  // `a` is a `LoanedMut<Tree>`, whilst `b` and `c` are `&mut Tree`s.
  let (a, b, c) = new_node();

  // Next, we fill one of those holes, `b`, with a leaf:
  // ```text
  //    a
  //   / \
  //  1   ?c
  // ```
  *b = Tree::Leaf(1);

  // Now, we create another node, `x`, with holes `y` and `z`:
  // ```text
  //   a        x
  //  / \      / \
  // 1   ?c  ?y   ?z
  // ```
  let (x, y, z) = new_node();

  // We fill `y` with another leaf:
  // ```text
  //   a        x
  //  / \      / \
  // 1   ?c   2   ?z
  // ```
  *y = Tree::Leaf(2);

  // Now, we fill the hole `c` with the node `x`.
  // ```text
  //   a
  //  / \
  // 1  / \
  //   2   ?z
  // ```
  // We still have a mutable reference to the hole `z`, even though we just
  // moved ownership of `x` – this is the key power of `Loaned` values.
  x.place(c); // this is like `*c = x`, except it accepts `Loaned` values.

  // Finally, we can fill the hole `z` (which is a mutable reference to data now owned by `a`):
  // ```text
  //   a
  //  / \
  // 1  / \
  //   2   3
  // ```
  *z = Tree::Leaf(3);

  // All of the borrows have expired, so we can now take the tree out of `a`:
  let a = take!(a);

  println!("{a:?}");
  assert_eq!(format!("{a:?}"), "Node(Leaf(1), Node(Leaf(2), Leaf(3)))");

  // If we tried to use one of the borrows now, we would get an error from the borrow checker.
  // *z = Tree::Leaf(0xBAD);
}

fn new_node<'t>() -> (LoanedMut<'t, Tree>, &'t mut Tree, &'t mut Tree) {
  let ((left, right), root) = LoanedMut::loan_with(
    Tree::Node(Default::default(), Default::default()),
    |tree, l| {
      let Tree::Node(left, right) = tree else {
        unreachable!()
      };
      (l.loan_mut(left), l.loan_mut(right))
    },
  );
  (root, left, right)
}
