use fxhash::FxHashMap;
use lazy_static::lazy_static;
use memchr::memmem::Finder;
use nom::{Err, IResult};
use nom::bytes::complete::{is_a, tag, take_while1};
use nom::character::complete::char as nchar;

use crate::common::{is_ident, is_ident_pfx, parse_lit};

lazy_static! {
	static ref ARGS_FINDER: Finder<'static> = Finder::new(b"}}(");
}

#[cfg(feature = "skip-whitespace")]
macro_rules! no_space {
    ($e:expr) => {
	    $e//nom::sequence::preceded(opt(is_a(&b" \t\r\n"[..])),$e)
    };
}

#[cfg(not(feature = "skip-whitespace"))]
macro_rules! no_space {
    ($e:expr) => {
	    nom::sequence::preceded(nom::combinator::opt(is_a(&b" \t\r\n"[..])),$e)
    };
}

pub fn parse_vars(input: &[u8]) -> Option<(FxHashMap<&[u8], &[u8]>, &[u8])> {
	let (input, head) = parse_head(input).ok()?;
	let arg_off = try_search(input, &ARGS_FINDER)? + 2;
	let (_, tail) = parse_tail(&input[arg_off..], head.capacity()).ok()?;
	if head.len() != tail.len() {
		return None;
	}
	Some((FxHashMap::from_iter(head.into_iter().zip(tail.into_iter())), &input[..arg_off]))
}

pub fn make_finder_object(object: &[u8]) -> Finder {
	Finder::new(object)
}

pub fn parse_object(input: &[u8], object: &Finder, capacity: usize) -> Option<Vec<u8>> {
	let (vars, input) = parse_vars(input)?;
	let obj_off = object.find(input)?;
	let mut input = &input[obj_off + object.needle().len()..];
	let mut bracket = Vec::with_capacity(32);
	let mut cursor = 0;
	let mut len = input.len();
	let mut json = Vec::<u8>::with_capacity(capacity);
	loop {
		if cursor == len { return None; }
		let ch = input[cursor];
		cursor += 1;
		match ch {
			n @ b'{' | n @ b',' => {
				if n == b'{' { bracket.push(b'}'); }
				if is_ident_pfx(input[cursor]) {
					json.extend(&input[..cursor]);
					let start = cursor;
					cursor += 1;
					while is_ident(input[cursor]) {
						cursor += 1;
					}
					let ident = &input[start..cursor];

					json.push(b'"');
					json.extend_from_slice(ident);
					json.push(b'"');
					input = &input[cursor..];
					len = input.len();
					cursor = 0;
				}
			}
			b'[' => { bracket.push(b']'); }
			n @ b']' | n @ b'}' => {
				if bracket.pop()? != n { return None; }
				if bracket.is_empty() { break; }
			}
			b':' => {
				if is_ident_pfx(input[cursor]) {
					json.extend(&input[..cursor]);
					let start = cursor;
					cursor += 1;
					while is_ident(input[cursor]) {
						cursor += 1;
					}
					let ident = &input[start..cursor];
					if let Some(var) = vars.get(ident) {
						json.extend_from_slice(var);
					} else {
						json.push(b'"');
						json.extend_from_slice(ident);
						json.push(b'"');
					}
					input = &input[cursor..];
					len = input.len();
					cursor = 0;
				}
			}
			//b'"' | b'\'' => {} // TODO: avoid crash because bracket in string :(
			_ => {}
		}
	}
	json.extend(&input[..cursor]);
	Some(json)
}

#[inline]
fn try_search(input: &[u8], finder: &Finder) -> Option<usize> {
	let mut input = input;
	for i in (2..=6).rev() {
		let half = (i - 1) * (input.len() / i);
		let val_buf = &input[half..];
		if let Some(res) = finder.find(val_buf) {
			return Some(res + half);
		}
		input = &input[..half];
	}
	finder.find(input)
}

fn parse_head(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
	let (input, _) = no_space!(is_a("function"))(input)?;
	let (input, _) = no_space!(nchar('('))(input)?;
	let mut vars = Vec::with_capacity(128);

	let mut input = input;

	while let Ok((buf, res)) = param_name(input) {
		vars.push(res);
		input = buf;
	}

	let (input, _) = no_space!(nchar(')'))(input)?;

	Ok((input, vars))
}

fn param_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (input, name) = no_space!(take_while1(is_ident))(input)?;
	let comma: IResult<&[u8], &[u8]> = no_space!(tag(b","))(input);
	let (input, _) = match comma {
		Ok(n) => { n }
		Err(Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	Ok((input, name))
}


fn parse_tail(input: &[u8], capacity: usize) -> IResult<&[u8], Vec<&[u8]>> {
	let (mut input, _) = no_space!(tag("("))(input)?;
	let mut res = Vec::with_capacity(capacity);
	while let Ok((_in, lit)) = parse_lit(input) {
		res.push(lit);
		input = _in;
	}
	let (input, _) = no_space!(tag(")"))(input)?;
	Ok((input, res))
}