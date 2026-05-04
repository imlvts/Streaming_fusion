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
pub fn descend_or_next<'a, 'path, V, A>
    (s: &mut ReadZipperUntracked<'a, 'path, V, A>)
     -> Option<ReadZipperUntracked<'a, 'path, V, A>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    let mut current = s.clone();
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
    *s = current;
    Some(s.clone())
}

pub fn prefix_of<'a, 'path, V, A>(
    a: Option<&ReadZipperUntracked<'a, 'path, V, A>>,
    b: Option<&ReadZipperUntracked<'a, 'path, V, A>>
) -> bool
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    // def prefix_of(self, other): other.path().startswith(self.path())
    match (a, b) {
        (Some(a), Some(b)) => b.path().starts_with(a.path()),
        _ => false,
    }
}

// TODO(igor): This function clones
pub fn argmin<'a, 'path, V, A>
    (v: &[&Option<ReadZipperUntracked<'a, 'path, V, A>>])
     -> Option<ReadZipperUntracked<'a, 'path, V, A>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    v.into_iter().filter_map(|x| x.as_ref()).min_by_key(|x| x.path()).cloned()
}

// TODO(igor): This function clones
pub fn argmax<'a, 'path, V, A>
    (v: &[&Option<ReadZipperUntracked<'a, 'path, V, A>>])
     -> Option<ReadZipperUntracked<'a, 'path, V, A>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    v.into_iter().filter_map(|x| x.as_ref()).max_by_key(|x| x.path()).cloned()
}
/*
    def difference_level(self, other):
        return next((e for e, (c1, c2) in enumerate(zip(self.path(), other.path())) if c1 != c2), None)
*/
pub fn difference_level<'a, 'path, V, A>(
    a: Option<&ReadZipperUntracked<'a, 'path, V, A>>,
    b: Option<&ReadZipperUntracked<'a, 'path, V, A>>,
) -> usize
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    if let (Some(a), Some(b)) = (a, b) {
        fast_slice_utils::find_prefix_overlap(a.path(), b.path())
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

pub fn next<'a, 'path, V, A>(
    a: &mut ReadZipperUntracked<'a, 'path, V, A>,
    level: usize,
) -> Option<ReadZipperUntracked<'a, 'path, V, A>>
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    assert!(level < a.path().len());
    let to_ascend = a.path().len() - level - 1;
    a.ascend(to_ascend);
    loop {
        if a.to_next_sibling_byte() {
            break;
        }
        if !a.ascend_byte() {
            return None;
        }
    }
    Some(a.clone())
}

pub fn path<'r, 'a, 'path, V, A>(a: Option<&'r ReadZipperUntracked<'a, 'path, V, A>>) -> &'r [u8]
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    a.as_ref().unwrap().path()
}
pub fn is_val<'r, 'a, 'path, V, A>(a: Option<&'r ReadZipperUntracked<'a, 'path, V, A>>) -> bool
where V: Clone + Send + Sync + Unpin, A: Allocator
{
    a.as_ref().unwrap().is_val()
}
