use nom::bytes::complete::{escaped, is_a, is_not, tag, take};
use nom::character::{is_alphabetic, is_alphanumeric};
use nom::error::{Error, ErrorKind};
use nom::IResult;
use nom::number::complete::recognize_float;

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
#[inline]
pub fn is_ident(ch: u8) -> bool {
	is_alphanumeric(ch) || matches!(ch, b'_' | b'$')
}

#[inline]
pub fn is_ident_pfx(ch: u8) -> bool {
	is_alphabetic(ch) || matches!(ch, b'_' | b'$')
}

pub fn string_literal(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (_in, _) = tag(b"\"")(input)?;
	let s: IResult<&[u8], &[u8]> = escaped(is_not(b"\"".as_slice()), '\\', is_a(b"\"n\\".as_slice()))(_in);
	let (_in, lit) = match s {
		Ok(lit) => { lit }
		Err(nom::Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	let (_, _) = tag(b"\"")(_in)?;
	let lit = &input[..=lit.len() + 1];
	Ok((&input[lit.len()..], lit))
}

pub fn string_literal_sq(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (_in, _) = tag(b"'")(input)?;
	let s: IResult<&[u8], &[u8]> = escaped(is_not(b"'".as_slice()), '\\', is_a(b"\'n\\".as_slice()))(_in);
	let (_in, lit) = match s {
		Ok(lit) => { lit }
		Err(nom::Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	let (_, _) = tag(b"'")(_in)?;
	let lit = &input[..=lit.len() + 1];
	Ok((&input[lit.len()..], lit))
}


#[inline]
pub fn parse_lit(input: &[u8]) -> IResult<&[u8], &[u8]> {
	let (_, pfx) = no_space!(take(1usize))(input)?;
	let (input, res) = match pfx[0] {
		b'N' => tag("NaN")(input),
		b'n' => tag("null")(input),
		b't' => tag("true")(input),
		b'u' => tag("undefined")(input).map(|(i, _)| (i, b"null".as_slice())),
		b'f' => tag("false")(input),
		b'"' => string_literal(input),
		b'\'' => string_literal_sq(input),
		ch => {
			if char::is_numeric(ch as char) || ch == b'-' || ch == b'+' {
				recognize_float(input)
			} else {
				Err(nom::Err::Failure(Error::new(input, ErrorKind::Fail)))
			}
		}
	}?;

	let comma: IResult<&[u8], &[u8]> = no_space!(tag(b","))(input);
	let (input, _) = match comma {
		Ok(n) => { n }
		Err(nom::Err::Error(err)) => {
			(err.input, b"".as_slice())
		}
		Err(e) => { return Err(e); }
	};
	Ok((input, res))
}