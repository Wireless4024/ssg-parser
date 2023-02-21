#![feature(test)]
extern crate test;

use std::fmt::Write;
use test::Bencher;

use rand::{Rng, RngCore, SeedableRng};
use rand::prelude::SliceRandom;
use rand::rngs::StdRng;

use ssg_parser::ssg_ifn::{make_finder_object, parse_object};

#[inline]
fn rng() -> StdRng {
	StdRng::seed_from_u64(4024)
}

#[test]
fn test_find() {
	let finder = make_finder_object(b"hello:");
	// input will have to start with function
	let data = b"function(a,b){return {hello:{bgga:b,dsa:a},other:\"...\"}}(1,\"qnlcpb\")";
	let json = parse_object(data, &finder, 8);
	assert_eq!(json, Some(b"{\"bgga\":\"qnlcpb\",\"dsa\":1}".to_vec()));
}

fn build_js(args: usize, obj_deep: usize, insert: &str, insert_at: usize) -> String {
	let mut js = String::new();
	js.push_str("function(");
	let mut arg_names = Vec::new();
	if args > 0 {
		let mut gen = ArgsGenerator::new();
		let mut arg_name = String::with_capacity(16);
		gen.next(&mut arg_name);
		js.push_str(&arg_name);
		arg_names.push(arg_name);
		for _ in 1..args {
			js.push(',');
			let mut arg_name = String::with_capacity(16);
			gen.next(&mut arg_name);
			js.push_str(&arg_name);
			arg_names.push(arg_name);
		}
	}
	js.push_str("){");
	let mut js_gen = JsValueRandom::new();
	if obj_deep > 0 {
		arg_names.shuffle(&mut js_gen.rng);
		js.push_str("return ");
		let obj_deep = obj_deep - 1;
		js_gen.build_js_object(obj_deep, &mut Some(insert), obj_deep - insert_at, &mut arg_names, &mut js);
	}
	js.push_str("}(");
	if args > 0 {
		js_gen.next_value(&mut js);
		for _ in 1..args {
			js.push(',');
			js_gen.next_value(&mut js);
		}
	}
	js.push(')');
	js
}

struct ArgsGenerator {
	inner: Vec<char>,
}

impl ArgsGenerator {
	fn new() -> Self {
		Self {
			inner: vec!['a']
		}
	}
	fn next(&mut self, output: &mut String) {
		output.reserve(self.inner.len());
		for &x in self.inner.iter() {
			output.push(x);
		}
		self.inc_char();
	}
	fn inc_char(&mut self) {
		if let Some('z') = self.inner.last() {
			self.inner.pop();
			self.inc_char();
			self.inner.push('a');
		} else if let Some(last) = self.inner.last_mut() {
			*last = ((*last as u8) + 1) as char;
		} else {
			self.inner.push('a');
		}
	}
}

struct JsValueRandom {
	rng: StdRng,
}

impl JsValueRandom {
	fn new() -> Self {
		Self {
			rng: rng()
		}
	}

	fn next_value(&mut self, out: &mut String) {
		match self.rng.gen_range(0..8) {
			0 => self.next_str(out),
			1 => self.next_int(out),
			2 => self.next_fp(out),
			3 => self.next_bool(out),
			4 => self.next_null(out),
			5 => self.next_str(out),
			6 => self.next_int(out),
			7 => self.next_undefined(out),
			_ => {}
		}
	}

	fn next_str(&mut self, out: &mut String) {
		out.push('"');
		self.next_ident(out);
		out.push('"');
	}

	fn build_js_object(&mut self, level: usize, insert: &mut Option<&str>, insert_at: usize, args: &mut Vec<String>, out: &mut String) {
		out.push('{');
		let mut pairs = self.rng.gen_range(2..5);
		if insert_at == level && insert.is_some() {
			pairs -= 1;
			out.push_str(insert.take().unwrap());
			out.push(':');
			if level == 0 {
				if !args.is_empty() {
					out.push_str(&args.pop().unwrap());
				} else {
					self.next_value(out);
				}
			} else {
				self.build_js_object(level - 1, insert, insert_at, args, out);
			}
			if pairs >= 1 {
				out.push(',');
			}
		}
		if pairs > 0 {
			self.next_ident(out);
			out.push(':');
			if level == 0 {
				if !args.is_empty() {
					out.push_str(&args.pop().unwrap());
				} else {
					self.next_value(out);
				}
			} else {
				self.build_js_object(level - 1, insert, insert_at, args, out);
			}
			for _ in 1..pairs {
				out.push(',');
				self.next_ident(out);
				out.push(':');
				if level == 0 {
					if !args.is_empty() {
						out.push_str(&args.pop().unwrap());
					} else {
						self.next_value(out);
					}
				} else {
					self.build_js_object(level - 1, insert, insert_at, args, out);
				}
			}
		}
		out.push('}');
	}

	fn next_ident(&mut self, out: &mut String) {
		let len = self.rng.gen_range(4..12);
		out.reserve(len + 1);
		for _ in 0..len {
			out.push(self.rng.gen_range('a'..='z'));
		}
	}

	fn next_int(&mut self, out: &mut String) {
		out.write_fmt(format_args!("{}", self.rng.next_u64())).unwrap();
	}

	fn next_fp(&mut self, out: &mut String) {
		let mul = self.rng.gen_range(1..5) * 10;
		let fp: f64 = self.rng.gen();
		let fp = fp * (mul as f64);
		out.write_fmt(format_args!("{fp}")).unwrap();
	}

	fn next_bool(&mut self, out: &mut String) {
		if self.rng.gen_bool(0.5) {
			out.push_str("true");
		} else {
			out.push_str("false");
		}
	}

	fn next_null(&mut self, out: &mut String) {
		out.push_str("null");
	}

	fn next_undefined(&mut self, out: &mut String) {
		out.push_str("undefined");
	}
}

#[bench]
fn _1_parse_1k(b: &mut Bencher) {
	let js = build_js(5, 4, "hello", 2);
	// js.len() is around 1k
	let finder = make_finder_object(b"hello:");
	b.iter(|| {
		parse_object(js.as_bytes(), &finder, 256).unwrap();
	})
}

#[bench]
fn _2_parse_10k(b: &mut Bencher) {
	let js = build_js(15, 6, "hello", 2);
	// js.len() is around 10k
	let finder = make_finder_object(b"hello:");
	b.iter(|| {
		parse_object(js.as_bytes(), &finder, 256).unwrap();
	})
}

#[bench]
fn _3_parse_200k(b: &mut Bencher) {
	let js = build_js(30, 8, "hello", 3);
	// js.len() is around 200k
	let finder = make_finder_object(b"hello:");
	b.iter(|| {
		parse_object(js.as_bytes(), &finder, 256).unwrap();
	})
}

#[bench]
fn _4_parse_2m(b: &mut Bencher) {
	let js = build_js(60, 10, "hello", 3);
	// js.len() is around 2m
	let finder = make_finder_object(b"hello:");
	b.iter(|| {
		parse_object(js.as_bytes(), &finder, 256).unwrap();
	})
}

#[bench]
fn _5_parse_100m(b: &mut Bencher) {
	let js = build_js(120, 14, "hello", 4);
	// js.len() is around 100m
	let finder = make_finder_object(b"hello:");
	b.iter(|| {
		parse_object(js.as_bytes(), &finder, 256).unwrap();
	})
}
