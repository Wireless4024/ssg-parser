use fxhash::FxHashMap;
use lazy_static::lazy_static;
use memchr::memmem::Finder;
use nom::{Err, IResult};
use nom::branch::alt;
use nom::bytes::complete::{escaped, is_a, is_not, tag, take, take_while1};
use nom::character::{is_alphabetic, is_alphanumeric};
use nom::character::complete::{char as nchar, digit0, digit1, one_of};
use nom::combinator::{opt, recognize};
use nom::error::{Error, ErrorKind};
use nom::sequence::tuple;

lazy_static! {
	static ref ARGS_FINDER: Finder<'static> = Finder::new(b"}}(");
}


macro_rules! no_space {
    ($e:expr) => {
	    $e//nom::sequence::preceded(opt(is_a(&b" \t\r\n"[..])),$e)
    };
}

pub fn parse_vars(input: &[u8]) -> Option<(FxHashMap<&[u8], &[u8]>, &[u8])> {
	let (input, head) = parse_head(input).ok()?;
	let arg_off = try_search(input, &ARGS_FINDER, 2)? + 2;
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
				if is_ident2(input[cursor]) {
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
				if is_ident2(input[cursor]) {
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
			_ => {}
		}
	}
	json.extend(&input[..cursor]);
	Some(json)
}

#[inline]
fn try_search(input: &[u8], finder: &Finder, min: usize) -> Option<usize> {
	for i in (min..=6).rev() {
		if let Some(res) = try_search_vals(i, input, finder) {
			return Some(res);
		}
	}
	None
}

#[inline]
fn try_search_vals(limit: usize, input: &[u8], finder: &Finder) -> Option<usize> {
	let half = (limit - 1) * (input.len() / limit);
	let val_buf = &input[half..];
	finder.find(val_buf).map(|it| it + half)
}

fn parse_head(input: &[u8]) -> IResult<&[u8], Vec<&[u8]>> {
	let (input, _) = no_space!(nchar('('))(input)?;
	let (input, _) = no_space!(is_a("function"))(input)?;
	let (input, _) = no_space!(nchar('('))(input)?;
	let mut vars = Vec::with_capacity(256);

	let mut input = input;

	while let Ok((buf, res)) = param_name(input) {
		vars.push(res);
		input = buf;
	}

	let (input, _) = no_space!(nchar(')'))(input)?;

	Ok((input, vars))
}

#[inline]
fn is_ident(ch: u8) -> bool {
	is_alphanumeric(ch) || matches!(ch, b'_' | b'$')
}

#[inline]
fn is_ident2(ch: u8) -> bool {
	is_alphabetic(ch) || matches!(ch, b'_' | b'$')
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

#[inline]
fn parse_lit(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (_, pfx) = no_space!(take(1usize))(input)?;
	let (input, res) = match pfx[0] {
		b'n' => tag("null")(input),
		b't' => tag("true")(input),
		b'u' => tag("undefined")(input),
		b'f' => tag("false")(input),
		b'"' => string_literal(input),
		b'\'' => string_literal2(input),
		ch => {
			if char::is_numeric(ch as char) || ch == b'-' || ch == b'+' {
				float(input)
			} else {
				Err(Err::Failure(Error::new(input, ErrorKind::Fail)))
			}
		}
	}?;

	let comma: IResult<&[u8], &[u8]> = no_space!(tag(b","))(input);
	let (input, _) = match comma {
		Ok(n) => { n }
		Err(Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	Ok((input, res))
}

fn string_literal(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (_in, _) = tag(b"\"")(input)?;
	let s: IResult<&[u8], &[u8]> = escaped(is_not(b"\"".as_slice()), '\\', is_a(b"\"n\\".as_slice()))(_in);
	let (_in, lit) = match s {
		Ok(lit) => { lit }
		Err(Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	let (_, _) = tag(b"\"")(_in)?;
	let lit = &input[..=lit.len() + 1];
	Ok((&input[lit.len()..], lit))
}

fn string_literal2(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (_in, _) = tag(b"'")(input)?;
	let s: IResult<&[u8], &[u8]> = escaped(is_not(b"\"".as_slice()), '\\', is_a(b"\"n\\".as_slice()))(_in);
	let (_in, lit) = match s {
		Ok(lit) => { lit }
		Err(Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	let (_, _) = tag(b"'")(_in)?;
	let lit = &input[..=lit.len() + 1];
	Ok((&input[lit.len()..], lit))
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

fn float(input: &[u8]) -> IResult<&[u8], &[u8]> {
	recognize(tuple((
		opt(one_of("+-")),
		alt((
			tag("0"),
			recognize(tuple((digit1, opt(tag(".")), opt(digit0)))),
			recognize(tuple((tag("."), digit1, opt(digit0)))),
		)),
		opt(alt((
			one_of("eE"),
			one_of("+-"),
			one_of("+-"),
		))),
		opt(digit1),
	)))(input)
}