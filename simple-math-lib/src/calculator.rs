use bigdecimal::BigDecimal;
use num::bigint::Sign;
use parser::{Token, ParseError};
use std::collections::HashMap;
use std::iter::Peekable;
use std::{self, fmt, mem};

/// An error when calculating
#[derive(Debug)]
pub enum CalcError {
	DivideByZero,
	ExpectedEOF(Token),
	IncorrectArguments(usize, usize),
	InvalidSyntax,
	NotAPositive,
	NotAPrimitive(&'static str),
	NotAWhole,
	ParseError(ParseError),
	SeparatorInDef,
	TooDeep,
	UnclosedParen,
	UnknownFunction(String),
	UnknownVariable(String)
}
impl fmt::Display for CalcError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		use std::error::Error;
		match *self {
			CalcError::ExpectedEOF(ref found) => write!(f, "Expected EOF, found {}", found),
			CalcError::IncorrectArguments(expected, received) =>
				write!(f, "Incorrect amount of arguments (Expected {}, got {})", expected, received),
			CalcError::NotAPrimitive(primitive) => write!(f, "Must fit in the range of an {} primitive", primitive),
			CalcError::ParseError(ref error) => write!(f, "{}", error),
			CalcError::UnknownFunction(ref name) =>
				write!(f, "Unknown function \"{}\"\nHint: Cannot assume multiplication of variables because of ambiguity", name),
			CalcError::UnknownVariable(ref name) => write!(f, "Unknown variable \"{}\"", name),
			_ => write!(f, "{}", self.description())
		}
	}
}
impl std::error::Error for CalcError {
	fn description(&self) -> &str {
		match *self {
			CalcError::DivideByZero => "Cannot divide by zero",
			CalcError::ExpectedEOF(_) => "Expected EOF",
			CalcError::IncorrectArguments(..) => "Incorrect amount of arguments",
			CalcError::InvalidSyntax => "Invalid syntax",
			CalcError::NotAPositive => "You may only do this on positive numbers",
			CalcError::NotAPrimitive(_) => "You may only do this on a specific primitive types",
			CalcError::NotAWhole => "You may only do this on whole numbers",
			CalcError::ParseError(ref error)  => error.description(),
			CalcError::SeparatorInDef => "A function definition cannot have multiple arguments",
			CalcError::TooDeep => "Too many levels deep. This could be an issue with endless recursion.",
			CalcError::UnclosedParen => "Unclosed parenthensis",
			CalcError::UnknownFunction(_) => "Unknown function",
			CalcError::UnknownVariable(_) => "Unknown variable"
		}
	}
}

macro_rules! to_primitive {
	($expr:expr, $type:ident, $primitive:expr) => {
		match $expr.$type() {
			Some(primitive) => primitive,
			None => return Err(CalcError::NotAPrimitive($primitive))
		}
	}
}

/// A Context for `calculate` to pass around to all its sub-functions
pub struct Context<'a, I: Iterator<Item = Token>> {
	level: u8,

	/// The tokens gotten by the parser
	pub tokens: Peekable<I>,
	/// A reference to a map of variables
	pub variables: &'a mut HashMap<String, BigDecimal>,
	/// A reference to a map of functions
	pub functions: &'a mut HashMap<String, Vec<Token>>
}
impl<'a, I: Iterator<Item = Token>> Context<'a, I> {
	pub fn new(
		tokens: Peekable<I>,
		variables: &'a mut HashMap<String, BigDecimal>,
		functions: &'a mut HashMap<String, Vec<Token>>
		) -> Context<'a, I> {

		Context {
			level: 0,
			tokens: tokens,
			variables: variables,
			functions: functions
		}
	}
}

/// Calculates the result in a recursive descent fashion
pub fn calculate<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	if context.level == std::u8::MAX {
		return Err(CalcError::TooDeep);
	}

	let expr1 = calc_level2(context)?;

	if let Some(&Token::Xor) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calculate(context)?;

		use num::ToPrimitive;
		let primitive1 = to_primitive!(expr1, to_i64, "i64");
		let primitive2 = to_primitive!(expr2, to_i64, "i64");

		return Ok(BigDecimal::from(primitive1 ^ primitive2));
	}

	match context.tokens.peek() {
		Some(&Token::ParenClose) |
		Some(&Token::Separator)
		if context.level != 0 => Ok(expr1),

		Some(_) => Err(CalcError::ExpectedEOF(context.tokens.next().unwrap())),
		None => Ok(expr1)
	}
}
fn calc_level2<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	let expr1 = calc_level3(context)?;

	if let Some(&Token::Or) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level2(context)?;

		use num::ToPrimitive;
		let primitive1 = to_primitive!(expr1, to_i64, "i64");
		let primitive2 = to_primitive!(expr2, to_i64, "i64");

		return Ok(BigDecimal::from(primitive1 | primitive2));
	}

	Ok(expr1)
}
fn calc_level3<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	let expr1 = calc_level4(context)?;

	if let Some(&Token::And) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level3(context)?;

		use num::ToPrimitive;
		let primitive1 = to_primitive!(expr1, to_i64, "i64");
		let primitive2 = to_primitive!(expr2, to_i64, "i64");

		return Ok(BigDecimal::from(primitive1 & primitive2));
	}

	Ok(expr1)
}
fn calc_level4<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	let expr1 = calc_level5(context)?;

	use num::bigint::ToBigInt;
	if let Some(&Token::BitshiftLeft) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level4(context)?;

		use num::ToPrimitive;
		let primitive2 = to_primitive!(expr2, to_usize, "usize");

		require_whole(&expr1)?;
		return Ok(BigDecimal::new(expr1.to_bigint().unwrap() << primitive2, 0));
	} else if let Some(&Token::BitshiftRight) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level4(context)?;

		use num::ToPrimitive;
		let primitive2 = to_primitive!(expr2, to_usize, "usize");

		require_whole(&expr1)?;
		return Ok(BigDecimal::new(expr1.to_bigint().unwrap() >> primitive2, 0));
	}

	Ok(expr1)
}
fn calc_level5<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	let expr1 = calc_level6(context)?;

	if let Some(&Token::Add) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level5(context)?;

		return Ok(expr1 + expr2);
	} else if let Some(&Token::Sub) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level5(context)?;

		return Ok(expr1 - expr2);
	}

	Ok(expr1)
}
fn calc_level6<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	let expr1 = calc_level7(context)?;

	if let Some(&Token::Mul) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level6(context)?;

		return Ok(expr1 * expr2);
	} else if let Some(&Token::Div) = context.tokens.peek() {
		context.tokens.next();
		let expr2 = calc_level6(context)?;

		use num::Zero;
		if expr2.is_zero() {
			return Err(CalcError::DivideByZero);
		}

		return Ok(expr1 / expr2);
	}

	Ok(expr1)
}
fn calc_level7<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	let expr = calc_level8(context)?;
	if let Some(&Token::Factorial) = context.tokens.peek() {
		context.tokens.next();

		return factorial(expr, None);
	}
	Ok(expr)
}
fn calc_level8<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	if let Some(&Token::Not) = context.tokens.peek() {
		context.tokens.next();
		use num::ToPrimitive;
		let expr = calc_level8(context)?;
		let primitive = to_primitive!(expr, to_i64, "i64");

		return Ok(BigDecimal::from(!primitive));
	}

	Ok(calc_level9(context, None)?)
}
fn calc_level9<I: Iterator<Item = Token>>(context: &mut Context<I>, name: Option<String>) -> Result<BigDecimal, CalcError> {
	if let Some(&Token::ParenOpen) = context.tokens.peek() {
		context.tokens.next();

		let mut args = Vec::new();

		if let Some(&Token::ParenClose) = context.tokens.peek() {
		} else {
			context.level += 1;

			args.push(calculate(context)?);

			while let Some(&Token::Separator) = context.tokens.peek() {
				context.tokens.next();
				args.push(calculate(context)?);
			}

			context.level -= 1;
		}
		if Some(Token::ParenClose) != context.tokens.next() {
			return Err(CalcError::UnclosedParen);
		}

		macro_rules! usage {
			($expected:expr) => {
				if args.len() != $expected {
					return Err(CalcError::IncorrectArguments($expected, args.len()));
				}
			}
		}

		if let Some(name) = name {
			match &*name {
				"abs" => {
					usage!(1);
					use num::Signed;
					args[0] = args[0].abs();
				},
				"pow" => {
					usage!(2);
					use num::Zero;
					args[0] = pow(mem::replace(&mut args[0], BigDecimal::zero()), args.remove(1), None)?;
				},
				_ => {
					let tokens = match context.functions.get(&name) {
						Some(tokens) => tokens.clone(),
						None => return Err(CalcError::UnknownFunction(name))
					};
					let len = args.len();
					for (i, arg) in args.into_iter().enumerate() {
						let mut name = String::with_capacity(2);
						name.push('$');
						name.push_str(&(i + 1).to_string());
						context.variables.insert(name, arg);
					}
					let val = calculate(&mut Context {
						tokens: tokens.into_iter().peekable(),
						level: context.level + 1,
						variables: &mut context.variables,
						functions: &mut context.functions
					});
					for i in 1..len+1 {
						let mut name = String::with_capacity(2);
						name.push('$');
						name.push_str(&i.to_string());
						context.variables.remove(&name);
					}
					return val;
				}
			}
		} else {
			usage!(1);
		}

		if args.is_empty() {
			use num::Zero;
			return Ok(BigDecimal::zero())
		} else {
			return Ok(args.remove(0));
		}
	} else if name.is_none() {
		if let Some(&Token::BlockName(_)) = context.tokens.peek() {
			// Really ugly code, but we need to know the type *before* we walk out on it
			if let Some(Token::BlockName(name)) = context.tokens.next() {
				return calc_level9(context, Some(name));
			}
		}
	}

	Ok(get_number(context)?)
}
fn get_number<I: Iterator<Item = Token>>(context: &mut Context<I>) -> Result<BigDecimal, CalcError> {
	match context.tokens.next() {
		Some(Token::Num(num)) => Ok(num),
		Some(Token::Sub) => {
			Ok(-calc_level9(context, None)?)
		},
		Some(Token::VarAssign(name)) => {
			if let Some(&Token::ParenOpen) = context.tokens.peek() {
				context.tokens.next();
				let mut fn_tokens = Vec::new();

				let mut depth = 1;
				loop {
					let token = match context.tokens.next() {
						Some(Token::Separator) if depth == 1 => return Err(CalcError::SeparatorInDef),
						Some(token) => token,
						None => return Err(CalcError::UnclosedParen)
					};
					if token == Token::ParenOpen {
						depth += 1;
					} else if token == Token::ParenClose {
						depth -= 1;
					}
					fn_tokens.push(token);

					if depth == 0 {
						break;
					} else if depth == std::u8::MAX {
						return Err(CalcError::TooDeep);
					}
				}

				context.functions.insert(name, fn_tokens);
			} else {
				let val = calculate(context)?;
				context.variables.insert(name, val);
			}
			use num::Zero;
			Ok(BigDecimal::zero())
		},
		Some(Token::VarGet(name)) => {
			Ok(
				match context.variables.get(&name) {
					Some(val) => val.clone(),
					None => return Err(CalcError::UnknownVariable(name))
				}
			)
		},
		_ => Err(CalcError::InvalidSyntax)
	}
}
fn require_whole(num: &BigDecimal) -> Result<(), CalcError> {
	if num.with_scale(0) == *num {
		Ok(())
	} else {
		Err(CalcError::NotAWhole)
	}
}
fn require_positive(num: &BigDecimal) -> Result<(), CalcError> {
	match num.sign() {
		Sign::NoSign |
		Sign::Plus => Ok(()),
		Sign::Minus => Err(CalcError::NotAPositive)
	}
}
/// Calculates the factorial of `num`
pub fn factorial(num: BigDecimal, result: Option<BigDecimal>) -> Result<BigDecimal, CalcError> {
	require_whole(&num)?;
	require_positive(&num)?;

	use num::{Zero, One};
	if num.is_zero() {
		Ok(result.unwrap_or_else(BigDecimal::one))
	} else {
		let result = result.unwrap_or_else(BigDecimal::one);
		let result = Some(result * &num);
		// Y THIS NO TAILCALL OPTIMIZE
		factorial(num - BigDecimal::one(), result)
	}
}
/// Calculates `num` to the power of `power`
pub fn pow(num: BigDecimal, power: BigDecimal, result: Option<BigDecimal>) -> Result<BigDecimal, CalcError> {
	require_positive(&num)?;
	require_whole(&power)?;

	use num::{Zero, One};
	let one = BigDecimal::one();
	if power.is_zero() {
		Ok(result.unwrap_or(one))
	} else if power == one {
		Ok(result.unwrap_or_else(|| num.clone()))
	} else {
		match power.sign() {
			Sign::NoSign => unreachable!(),
			Sign::Plus => {
				let result = result.unwrap_or_else(|| num.clone());
				let result = Some(result * &num);
				// Y THIS NO TAILCALL OPTIMIZE
				pow(num, power - one, result)
			},
			Sign::Minus => {
				// `let power = ...` is kinda ugly, but I need it to happen BEFORE
				// the reference dies to avoid cloning.
				let power = power + &one;
				let result = result.unwrap_or(one);
				let result = Some(result / &num);
				// Y THIS NO TAILCALL OPTIMIZE
				pow(num, power, result)
			}
		}
	}
}
