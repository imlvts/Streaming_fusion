
use std::cell::RefCell;
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
	let a = RefCell::new(__src_a.read_zipper());
	let __src_b: PathMap<Option<u32>> = PathMap::from_iter([("001", None), ("010", None), ("100", None), ("101", None)]);
	let b = RefCell::new(__src_b.read_zipper());
	let __src_c: PathMap<Option<u32>> = PathMap::from_iter([("010", None), ("011", None), ("100", None), ("101", None)]);
	let c = RefCell::new(__src_c.read_zipper());
	let __src_d: PathMap<Option<u32>> = PathMap::from_iter([("000", None), ("100", None)]);
	let d = RefCell::new(__src_d.read_zipper());
	let mut r = Vec::new();

// defined vars: 1
let mut m = None;
let mut tmp_c = None;
let mut tmp_a = None;
let mut tmp_d = None;
let mut tmp_b = None;
let mut state = 0;
'dispatch: loop {
	match state {
	0 => {

		if true {
			tmp_c = descend_or_next(&c);
			tmp_a = descend_or_next(&a);
			tmp_d = descend_or_next(&d);
			tmp_b = descend_or_next(&b);
			state = 1;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	1 => {

		if tmp_a.is_some() && tmp_c.is_some() && is_val(&Some(&a)) && is_val(&Some(&c)) && path(&Some(&a)) == path(&Some(&a)) && path(&Some(&a)) == path(&Some(&c)) && (tmp_b.is_none() || path(&Some(&b)) >= path(&Some(&a))) && (tmp_d.is_none() || (path(&Some(&d)) != path(&Some(&a)) || !is_val(&Some(&d)))) {
			state = 2;
			continue 'dispatch;
		}

		if tmp_c.is_some() && tmp_b.is_some() && is_val(&Some(&c)) && is_val(&Some(&b)) && path(&Some(&c)) == path(&Some(&c)) && path(&Some(&c)) == path(&Some(&b)) && (tmp_a.is_none() || path(&Some(&a)) >= path(&Some(&c))) && (tmp_d.is_none() || (path(&Some(&d)) != path(&Some(&c)) || !is_val(&Some(&d)))) {
			state = 3;
			continue 'dispatch;
		}

		if true {
			state = 4;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	4 => {

		if tmp_a.is_some() && (tmp_c.is_none() || path(&Some(&c)) >= path(&Some(&a))) && (tmp_b.is_none() || path(&Some(&b)) >= path(&Some(&a))) {
			state = 5;
			continue 'dispatch;
		}

		if tmp_c.is_some() && (tmp_a.is_none() || path(&Some(&a)) >= path(&Some(&c))) && (tmp_b.is_none() || path(&Some(&b)) >= path(&Some(&c))) {
			state = 6;
			continue 'dispatch;
		}

		if tmp_b.is_some() && (tmp_a.is_none() || path(&Some(&a)) >= path(&Some(&b))) && (tmp_c.is_none() || path(&Some(&c)) >= path(&Some(&b))) {
			state = 7;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	6 => {

		if tmp_a.is_some() && path(&Some(&c)) == path(&Some(&a)) {
			state = 8;
			continue 'dispatch;
		}

		if tmp_b.is_some() && path(&Some(&c)) == path(&Some(&b)) {
			state = 9;
			continue 'dispatch;
		}

		if true {
			m = argmin(&[&tmp_a, &tmp_b]);
			state = 10;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	5 => {

		if tmp_c.is_some() && path(&Some(&a)) == path(&Some(&c)) {
			m = argmin(&[&tmp_a, &tmp_b]);
			state = 11;
			continue 'dispatch;
		}

		if tmp_b.is_some() && path(&Some(&a)) == path(&Some(&b)) {
			state = 12;
			continue 'dispatch;
		}

		if true {
			m = tmp_c.clone();
			state = 13;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	14 => {
		break 'dispatch;
	},
	7 => {

		if tmp_a.is_some() && path(&Some(&b)) == path(&Some(&a)) {
			state = 15;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(&Some(&b)) == path(&Some(&c)) {
			m = argmin(&[&tmp_a, &tmp_b]);
			state = 16;
			continue 'dispatch;
		}

		if true {
			m = tmp_c.clone();
			state = 17;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	2 => {

		if (tmp_d.is_none() || path(&Some(&d)) > path(&Some(&a)) || (path(&Some(&d)) == path(&Some(&a)) && !is_val(&Some(&d)))) {
			r.push(path(&tmp_a).to_vec());
			state = 5;
			continue 'dispatch;
		}

		if tmp_d.is_some() && path(&Some(&d)) < path(&Some(&a)) {
			state = 18;
			continue 'dispatch;
		}

		if tmp_d.is_some() && is_val(&Some(&d)) && path(&Some(&d)) == path(&Some(&a)) {
			state = 1;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	3 => {

		if (tmp_d.is_none() || path(&Some(&d)) > path(&Some(&c)) || (path(&Some(&d)) == path(&Some(&c)) && !is_val(&Some(&d)))) {
			r.push(path(&tmp_c).to_vec());
			state = 6;
			continue 'dispatch;
		}

		if tmp_d.is_some() && path(&Some(&d)) < path(&Some(&c)) {
			state = 19;
			continue 'dispatch;
		}

		if tmp_d.is_some() && is_val(&Some(&d)) && path(&Some(&d)) == path(&Some(&c)) {
			state = 1;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	18 => {

		if tmp_d.is_some() && prefix_of(&tmp_d, &tmp_a) {
			tmp_d = descend_or_next(&d);
			state = 2;
			continue 'dispatch;
		}

		if tmp_d.is_some() && !prefix_of(&tmp_d, &tmp_a) {
			let diff_level = difference_level(&Some(&d), &Some(&a));
			tmp_d = next(&d, diff_level);
			state = 2;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	19 => {

		if tmp_d.is_some() && prefix_of(&tmp_d, &tmp_c) {
			tmp_d = descend_or_next(&d);
			state = 3;
			continue 'dispatch;
		}

		if tmp_d.is_some() && !prefix_of(&tmp_d, &tmp_c) {
			let diff_level = difference_level(&Some(&d), &Some(&c));
			tmp_d = next(&d, diff_level);
			state = 3;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	11 => {

		if m.is_none() {
			tmp_c = descend_or_next(&c);
			state = 5;
			continue 'dispatch;
		}

		if prefix_of(&tmp_c, &m) {
			tmp_c = descend_or_next(&c);
			state = 5;
			continue 'dispatch;
		}

		if !prefix_of(&tmp_c, &m) {
			let diff_level = difference_level(&Some(&c), &m);
			tmp_c = next(&c, diff_level);
			state = 5;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	12 => {

		if tmp_c.is_some() && path(&Some(&a)) == path(&Some(&b)) && prefix_of(&tmp_b, &tmp_c) {
			tmp_b = descend_or_next(&b);
			state = 5;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(&Some(&a)) == path(&Some(&b)) && !prefix_of(&tmp_b, &tmp_c) {
			let diff_level = difference_level(&Some(&b), &Some(&c));
			tmp_b = next(&b, diff_level);
			state = 5;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_b = None;
			state = 5;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	13 => {

		if m.is_none() {
			tmp_a = descend_or_next(&a);
			state = 1;
			continue 'dispatch;
		}

		if prefix_of(&tmp_a, &m) {
			tmp_a = descend_or_next(&a);
			state = 1;
			continue 'dispatch;
		}

		if !prefix_of(&tmp_a, &m) {
			let diff_level = difference_level(&Some(&a), &m);
			tmp_a = next(&a, diff_level);
			state = 1;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	8 => {

		if tmp_c.is_some() && path(&Some(&c)) == path(&Some(&a)) && prefix_of(&tmp_a, &tmp_c) {
			tmp_a = descend_or_next(&a);
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(&Some(&c)) == path(&Some(&a)) && !prefix_of(&tmp_a, &tmp_c) {
			let diff_level = difference_level(&Some(&a), &Some(&c));
			tmp_a = next(&a, diff_level);
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_a = None;
			state = 6;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	9 => {

		if tmp_c.is_some() && path(&Some(&c)) == path(&Some(&b)) && prefix_of(&tmp_b, &tmp_c) {
			tmp_b = descend_or_next(&b);
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(&Some(&c)) == path(&Some(&b)) && !prefix_of(&tmp_b, &tmp_c) {
			let diff_level = difference_level(&Some(&b), &Some(&c));
			tmp_b = next(&b, diff_level);
			state = 6;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_b = None;
			state = 6;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	10 => {

		if m.is_none() {
			tmp_c = descend_or_next(&c);
			state = 1;
			continue 'dispatch;
		}

		if prefix_of(&tmp_c, &m) {
			tmp_c = descend_or_next(&c);
			state = 1;
			continue 'dispatch;
		}

		if !prefix_of(&tmp_c, &m) {
			let diff_level = difference_level(&Some(&c), &m);
			tmp_c = next(&c, diff_level);
			state = 1;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	15 => {

		if tmp_c.is_some() && path(&Some(&b)) == path(&Some(&a)) && prefix_of(&tmp_a, &tmp_c) {
			tmp_a = descend_or_next(&a);
			state = 7;
			continue 'dispatch;
		}

		if tmp_c.is_some() && path(&Some(&b)) == path(&Some(&a)) && !prefix_of(&tmp_a, &tmp_c) {
			let diff_level = difference_level(&Some(&a), &Some(&c));
			tmp_a = next(&a, diff_level);
			state = 7;
			continue 'dispatch;
		}

		if tmp_c.is_none() {
			tmp_a = None;
			state = 7;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	16 => {

		if m.is_none() {
			tmp_c = descend_or_next(&c);
			state = 7;
			continue 'dispatch;
		}

		if prefix_of(&tmp_c, &m) {
			tmp_c = descend_or_next(&c);
			state = 7;
			continue 'dispatch;
		}

		if !prefix_of(&tmp_c, &m) {
			let diff_level = difference_level(&Some(&c), &m);
			tmp_c = next(&c, diff_level);
			state = 7;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	17 => {

		if m.is_none() {
			tmp_b = descend_or_next(&b);
			state = 1;
			continue 'dispatch;
		}

		if prefix_of(&tmp_b, &m) {
			tmp_b = descend_or_next(&b);
			state = 1;
			continue 'dispatch;
		}

		if !prefix_of(&tmp_b, &m) {
			let diff_level = difference_level(&Some(&b), &m);
			tmp_b = next(&b, diff_level);
			state = 1;
			continue 'dispatch;
		}
		break 'dispatch;
	},
	unk_state => unreachable!("invalid state {}", unk_state),
	} // match state
} // 'dispatch: loop
// state id mapping: { 0: s0, 1: s1, 2: sc0, 3: sc1, 4: s2, 5: sa, 6: sc, 7: sb, 8: n5, 9: n6, 10: n7, 11: n2, 12: n3, 13: n4, 14: sd, 15: n8, 16: n9, 17: n10, 18: n0, 19: n1 }

// eprintln!("wanted: {:?}", {('010', None), ('101', None)});
println!("result:");
for v in r {
    shim::print_path(&v);
}
}
fn main() { test(); }
        