use std::cell::RefCell;
// shim for PathMap methods
use pathmap::alloc::{Allocator};
pub use pathmap::{PathMap, zipper::ReadZipperUntracked};
pub use pathmap::zipper::{ZipperMoving, Zipper};
/*
    def descend_or_next(self):
        # descend one, and if you can't descend anymore, find the next possible value
        if self.current.descend_first() is not None:
            self.current = self.current.descend_first()
        else:
            while self.current is not None and not self.has_sibling():
                self.current = self.ascend()
            if self.current is None:
                print(f"descend None from {self.name}")
                return None
            self.current = self.next_sibling()
            if self.current is None:
                return None
        print(f"descend {self.current.path} from {self.name}")
        return self
*/

// descend first byte or visit next branch anywhere up the trie
// TODO(igor): This function clones
pub fn descend_or_next<'r, 'a, 'path, V, A>
    (s: &'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>)
     -> Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    let mut current = s.borrow_mut();
    if !current.descend_first_byte() {
        loop {
            if current.to_next_sibling_byte() {
                break;
            }
            if !current.ascend_byte() {
                return None;
            }
        }
    }
    Some(s)
}

pub fn prefix_of<'r, 'a, 'path, V, A>(
    a: &Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>,
    b: &Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
) -> bool
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    // def prefix_of(self, other): other.path().startswith(self.path())
    match (a, b) {
        (Some(a), Some(b)) => b.borrow().path().starts_with(a.borrow().path()),
        _ => false,
    }
}

pub fn argmin<'r, 'a, 'path, V, A>
    (v: &[&Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>])
     -> Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    v.into_iter()
        .filter_map(|x| x.as_ref())
        // TODO: to_vec needed because of RefCell. can be optimized
        .min_by_key(|x| x.borrow().path().to_vec())
        .map(|x| *x)
}

pub fn argmax<'r, 'a, 'path, V, A>
    (v: &[&Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>])
     -> Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    v.into_iter()
        .filter_map(|x| x.as_ref())
        // TODO: to_vec needed because of RefCell. can be optimized
        .max_by_key(|x| x.borrow().path().to_vec())
        .map(|x| *x)
}
/*
    def difference_level(self, other):
        return next((e for e, (c1, c2) in enumerate(zip(self.path(), other.path())) if c1 != c2), None)
*/
pub fn difference_level<'r, 'a, 'path, V, A>(
    a: &Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>,
    b: &Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>,
) -> usize
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    if let (Some(a), Some(b)) = (a, b) {
        fast_slice_utils::find_prefix_overlap(a.borrow().path(), b.borrow().path())
    } else {
        panic!("difference_level called with None");
    }
    
}
/*
    def next(self, i):
        current_lvl = len(self.path())
        for _ in range(current_lvl - i - 1):
            self.current = self.ascend()
        while self.current is not None and not self.has_sibling():
            self.current = self.ascend()

        if self.current is None:
            return None
        self.current = self.next_sibling()
        if self.current is None:
            return None
        print(f"next_{i} {self.current.path} from {self.name}")
        return self
*/

pub fn next<'r, 'a, 'path, V, A>(
    a: &'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>,
    level: usize,
) -> Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    let mut am = a.borrow_mut();
    assert!(level < am.path().len());
    let to_ascend = am.path().len() - level - 1;
    am.ascend(to_ascend);
    loop {
        if am.to_next_sibling_byte() {
            break;
        }
        if !am.ascend_byte() {
            return None;
        }
    }
    Some(a)
}

pub fn path<'r, 'a, 'path, V, A>(
    a: &Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
) -> Vec<u8>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    let ar = a.as_ref().unwrap();
    let ar = ar.borrow();
    eprintln!("path of {:?} is {:?}", ar.path(), ar.is_val());
    // TODO: to_vec needed because of RefCell. can be optimized
    a.as_ref().unwrap().borrow().path().to_vec()
}
pub fn is_val<'r, 'a, 'path, V, A>(
    a: &Option<&'r RefCell<ReadZipperUntracked<'a, 'path, V, A>>>
) -> bool
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    a.as_ref().unwrap().borrow().is_val()
}
