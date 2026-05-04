
mod shim;
#[allow(unused_imports)]
use shim::{
    PathMap, argmin, argmax, descend_or_next,
    prefix_of, difference_level, next, path, is_val,
    Zipper, ZipperMoving, ReadZipperUntracked,
};
#[allow(unused_parens)]
fn test() {
	let __src_a: PathMap<Option<u32>> = PathMap::from_iter([("001", None), ("100", None), ("101", None), ("110", None)]);
	let mut a = __src_a.read_zipper();
	let __src_b: PathMap<Option<u32>> = PathMap::from_iter([("001", None), ("010", None), ("100", None), ("101", None)]);
	let mut b = __src_b.read_zipper();
	let __src_c: PathMap<Option<u32>> = PathMap::from_iter([("010", None), ("011", None), ("100", None), ("101", None)]);
	let mut c = __src_c.read_zipper();
	let __src_d: PathMap<Option<u32>> = PathMap::from_iter([("000", None), ("100", None)]);
	let mut d = __src_d.read_zipper();
	let mut r = Vec::new();

// defined vars: 1
let mut m = None;
let mut tmp_a = None;
let mut tmp_b = None;
let mut tmp_d = None;
let mut tmp_c = None;
let mut state = 0;
'dispatch: loop {
	match state {
	0 => {

		if true {
			tmp_a = descend_or_next(&mut a);
			tmp_b = descend_or_next(&mut b);
			tmp_d = descend_or_next(&mut d);
			tmp_c = descend_or_next(&mut c);
			state = 1;
			continue 'dispatch;
		}
	},
	1 => {

		if tmp_b.is_some() && tmp_c.is_some() && is_val(Some(&b)) && is_val(Some(&c)) && path(Some(&b)) == path(Some(&b)) && path(Some(&b)) == path(Some(&c)) && (tmp_a.is_none() || path(Some(&a)) >= path(Some(&b))) && (tmp_d.is_none() || (path(Some(&d)) != path(Some(&b)) || !is_val(Some(&d)))) {
			state = 2;
			continue 'dispatch;
		}

		if tmp_a.is_some() && tmp_c.is_some() && is_val(Some(&a)) && is_val(Some(&c)) && path(Some(&a)) == path(Some(&a)) && path(Some(&a)) == path(Some(&c)) && (tmp_b.is_none() || path(Some(&b)) >= path(Some(&a))) && (tmp_d.is_none() || (path(Some(&d)) != path(Some(&a)) || !is_val(Some(&d)))) {
			state = 3;
			continue 'dispatch;
		}

		if true {
			state = 4;
			continue 'dispatch;
		}
	},
	4 => {

		if tmp_a.is_some() && (tmp_b.is_none() || path(Some(&b)) >= path(Some(&a))) && (tmp_c.is_none() || path(Some(&c)) >= path(Some(&a))) {
			state = 5;
			continue 'dispatch;
		}

		if tmp_b.is_some() && (tmp_a.is_none() || path(Some(&a)) >= path(Some(&b))) && (tmp_c.is_none() || path(Some(&c)) >= path(Some(&b))) {
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_some() && (tmp_a.is_none() || path(Some(&a)) >= path(Some(&c))) && (tmp_b.is_none() || path(Some(&b)) >= path(Some(&c))) {
			state = 7;
			continue 'dispatch;
		}
	},
	5 => {

		if tmp_b.is_some() && path(Some(&a)) == path(Some(&b)) {
			state = 8;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(Some(&a)) == path(Some(&c)) {
			m = argmin(&[&tmp_a, &tmp_b]);
			state = 9;
			continue 'dispatch;
		}

		if true {
			m = tmp_c.clone();
			state = 10;
			continue 'dispatch;
		}
	},
	6 => {

		if tmp_a.is_some() && path(Some(&b)) == path(Some(&a)) {
			state = 11;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(Some(&b)) == path(Some(&c)) {
			m = argmin(&[&tmp_a, &tmp_b]);
			state = 12;
			continue 'dispatch;
		}

		if true {
			m = tmp_c.clone();
			state = 13;
			continue 'dispatch;
		}
	},
	14 => {
	},
	7 => {

		if tmp_a.is_some() && path(Some(&c)) == path(Some(&a)) {
			state = 15;
			continue 'dispatch;
		}

		if tmp_b.is_some() && path(Some(&c)) == path(Some(&b)) {
			state = 16;
			continue 'dispatch;
		}

		if true {
			m = argmin(&[&tmp_a, &tmp_b]);
			state = 17;
			continue 'dispatch;
		}
	},
	2 => {

		if (tmp_d.is_none() || path(Some(&d)) > path(Some(&b)) || (path(Some(&d)) == path(Some(&b)) && !is_val(Some(&d)))) {
			r.push(path(tmp_b.as_ref()).to_vec());
			state = 6;
			continue 'dispatch;
		}

		if tmp_d.is_some() && path(Some(&d)) < path(Some(&b)) {
			state = 18;
			continue 'dispatch;
		}

		if tmp_d.is_some() && is_val(Some(&d)) && path(Some(&d)) == path(Some(&b)) {
			state = 1;
			continue 'dispatch;
		}
	},
	3 => {

		if (tmp_d.is_none() || path(Some(&d)) > path(Some(&a)) || (path(Some(&d)) == path(Some(&a)) && !is_val(Some(&d)))) {
			r.push(path(tmp_a.as_ref()).to_vec());
			state = 5;
			continue 'dispatch;
		}

		if tmp_d.is_some() && path(Some(&d)) < path(Some(&a)) {
			state = 19;
			continue 'dispatch;
		}

		if tmp_d.is_some() && is_val(Some(&d)) && path(Some(&d)) == path(Some(&a)) {
			state = 1;
			continue 'dispatch;
		}
	},
	18 => {

		if tmp_d.is_some() && prefix_of(tmp_d.as_ref(), tmp_b.as_ref()) {
			tmp_d = descend_or_next(&mut d);
			state = 2;
			continue 'dispatch;
		}

		if tmp_d.is_some() && !prefix_of(tmp_d.as_ref(), tmp_b.as_ref()) {
			let diff_level = difference_level(Some(&d), Some(&b));
			tmp_d = next(&mut d, diff_level);
			state = 2;
			continue 'dispatch;
		}
	},
	19 => {

		if tmp_d.is_some() && prefix_of(tmp_d.as_ref(), tmp_a.as_ref()) {
			tmp_d = descend_or_next(&mut d);
			state = 3;
			continue 'dispatch;
		}

		if tmp_d.is_some() && !prefix_of(tmp_d.as_ref(), tmp_a.as_ref()) {
			let diff_level = difference_level(Some(&d), Some(&a));
			tmp_d = next(&mut d, diff_level);
			state = 3;
			continue 'dispatch;
		}
	},
	8 => {

		if tmp_c.is_some() && path(Some(&a)) == path(Some(&b)) && prefix_of(tmp_b.as_ref(), tmp_c.as_ref()) {
			tmp_b = descend_or_next(&mut b);
			state = 5;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(Some(&a)) == path(Some(&b)) && !prefix_of(tmp_b.as_ref(), tmp_c.as_ref()) {
			let diff_level = difference_level(Some(&b), Some(&c));
			tmp_b = next(&mut b, diff_level);
			state = 5;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_b = None;
			state = 5;
			continue 'dispatch;
		}
	},
	9 => {

		if m.is_none() {
			tmp_c = descend_or_next(&mut c);
			state = 5;
			continue 'dispatch;
		}

		if prefix_of(tmp_c.as_ref(), m.as_ref()) {
			tmp_c = descend_or_next(&mut c);
			state = 5;
			continue 'dispatch;
		}

		if !prefix_of(tmp_c.as_ref(), m.as_ref()) {
			let diff_level = difference_level(Some(&c), m.as_ref());
			tmp_c = next(&mut c, diff_level);
			state = 5;
			continue 'dispatch;
		}
	},
	10 => {

		if m.is_none() {
			tmp_a = descend_or_next(&mut a);
			state = 1;
			continue 'dispatch;
		}

		if prefix_of(tmp_a.as_ref(), m.as_ref()) {
			tmp_a = descend_or_next(&mut a);
			state = 1;
			continue 'dispatch;
		}

		if !prefix_of(tmp_a.as_ref(), m.as_ref()) {
			let diff_level = difference_level(Some(&a), m.as_ref());
			tmp_a = next(&mut a, diff_level);
			state = 1;
			continue 'dispatch;
		}
	},
	11 => {

		if tmp_c.is_some() && path(Some(&b)) == path(Some(&a)) && prefix_of(tmp_a.as_ref(), tmp_c.as_ref()) {
			tmp_a = descend_or_next(&mut a);
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(Some(&b)) == path(Some(&a)) && !prefix_of(tmp_a.as_ref(), tmp_c.as_ref()) {
			let diff_level = difference_level(Some(&a), Some(&c));
			tmp_a = next(&mut a, diff_level);
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_a = None;
			state = 6;
			continue 'dispatch;
		}
	},
	12 => {

		if m.is_none() {
			tmp_c = descend_or_next(&mut c);
			state = 6;
			continue 'dispatch;
		}

		if prefix_of(tmp_c.as_ref(), m.as_ref()) {
			tmp_c = descend_or_next(&mut c);
			state = 6;
			continue 'dispatch;
		}

		if !prefix_of(tmp_c.as_ref(), m.as_ref()) {
			let diff_level = difference_level(Some(&c), m.as_ref());
			tmp_c = next(&mut c, diff_level);
			state = 6;
			continue 'dispatch;
		}
	},
	13 => {

		if m.is_none() {
			tmp_b = descend_or_next(&mut b);
			state = 1;
			continue 'dispatch;
		}

		if prefix_of(tmp_b.as_ref(), m.as_ref()) {
			tmp_b = descend_or_next(&mut b);
			state = 1;
			continue 'dispatch;
		}

		if !prefix_of(tmp_b.as_ref(), m.as_ref()) {
			let diff_level = difference_level(Some(&b), m.as_ref());
			tmp_b = next(&mut b, diff_level);
			state = 1;
			continue 'dispatch;
		}
	},
	15 => {

		if tmp_c.is_some() && path(Some(&c)) == path(Some(&a)) && prefix_of(tmp_a.as_ref(), tmp_c.as_ref()) {
			tmp_a = descend_or_next(&mut a);
			state = 7;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(Some(&c)) == path(Some(&a)) && !prefix_of(tmp_a.as_ref(), tmp_c.as_ref()) {
			let diff_level = difference_level(Some(&a), Some(&c));
			tmp_a = next(&mut a, diff_level);
			state = 7;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_a = None;
			state = 7;
			continue 'dispatch;
		}
	},
	16 => {

		if tmp_c.is_some() && path(Some(&c)) == path(Some(&b)) && prefix_of(tmp_b.as_ref(), tmp_c.as_ref()) {
			tmp_b = descend_or_next(&mut b);
			state = 7;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(Some(&c)) == path(Some(&b)) && !prefix_of(tmp_b.as_ref(), tmp_c.as_ref()) {
			let diff_level = difference_level(Some(&b), Some(&c));
			tmp_b = next(&mut b, diff_level);
			state = 7;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_b = None;
			state = 7;
			continue 'dispatch;
		}
	},
	17 => {

		if m.is_none() {
			tmp_c = descend_or_next(&mut c);
			state = 1;
			continue 'dispatch;
		}

		if prefix_of(tmp_c.as_ref(), m.as_ref()) {
			tmp_c = descend_or_next(&mut c);
			state = 1;
			continue 'dispatch;
		}

		if !prefix_of(tmp_c.as_ref(), m.as_ref()) {
			let diff_level = difference_level(Some(&c), m.as_ref());
			tmp_c = next(&mut c, diff_level);
			state = 1;
			continue 'dispatch;
		}
	},
	unk_state => unreachable!("invalid state {}", unk_state),
	} // match state
} // 'dispatch: loop
// state id mapping: { 0: s0, 1: s1, 2: sc0, 3: sc1, 4: s2, 5: sa, 6: sb, 7: sc, 8: n2, 9: n3, 10: n4, 11: n5, 12: n6, 13: n7, 14: sd, 15: n8, 16: n9, 17: n10, 18: n0, 19: n1 }

}
fn main() { test(); }
        