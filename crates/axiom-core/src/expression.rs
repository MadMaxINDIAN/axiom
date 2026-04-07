/// Sandboxed expression evaluator for ARS action value templates.
///
/// Parses and evaluates `{{ <expr> }}` template strings.
/// Security constraints (§5.5):
/// - No function calls
/// - Identifiers (field references) are resolved from context before evaluation
/// - No loops or recursion in the grammar
/// - AST depth limit: 16 nodes
/// - Checked arithmetic: overflow returns Err
use serde_json::Value;
use crate::error::EvaluationError;

const MAX_DEPTH: usize = 16;

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Number(f64),
    Bool(bool),
    Str(String),
    Ident(String),   // field path — resolved to a Value before eval
    Plus, Minus, Star, Slash, Percent,
    EqEq, BangEq, Lt, Gt, LtEq, GtEq,
    AndAnd, OrOr, Bang,
    LParen, RParen,
}

fn tokenize(src: &str) -> Result<Vec<Token>, EvaluationError> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            ' ' | '\t' | '\n' | '\r' => { i += 1; }
            '+' => { tokens.push(Token::Plus);   i += 1; }
            '-' => { tokens.push(Token::Minus);  i += 1; }
            '*' => { tokens.push(Token::Star);   i += 1; }
            '/' => { tokens.push(Token::Slash);  i += 1; }
            '%' => { tokens.push(Token::Percent);i += 1; }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            '!' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(Token::BangEq); i += 2;
                } else {
                    tokens.push(Token::Bang); i += 1;
                }
            }
            '=' if i + 1 < chars.len() && chars[i+1] == '=' => {
                tokens.push(Token::EqEq); i += 2;
            }
            '<' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(Token::LtEq); i += 2;
                } else {
                    tokens.push(Token::Lt); i += 1;
                }
            }
            '>' => {
                if i + 1 < chars.len() && chars[i+1] == '=' {
                    tokens.push(Token::GtEq); i += 2;
                } else {
                    tokens.push(Token::Gt); i += 1;
                }
            }
            '&' if i + 1 < chars.len() && chars[i+1] == '&' => {
                tokens.push(Token::AndAnd); i += 2;
            }
            '|' if i + 1 < chars.len() && chars[i+1] == '|' => {
                tokens.push(Token::OrOr); i += 2;
            }
            '"' | '\'' => {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote { i += 1; }
                let s: String = chars[start..i].iter().collect();
                tokens.push(Token::Str(s));
                i += 1; // closing quote
            }
            c if c.is_ascii_digit() || (c == '.' && i + 1 < chars.len() && chars[i+1].is_ascii_digit()) => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') { i += 1; }
                let num: f64 = chars[start..i].iter().collect::<String>()
                    .parse()
                    .map_err(|_| EvaluationError::Expression("invalid number literal".into()))?;
                tokens.push(Token::Number(num));
            }
            c if c.is_alphanumeric() || c == '_' || c == '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.as_str() {
                    "true"  => tokens.push(Token::Bool(true)),
                    "false" => tokens.push(Token::Bool(false)),
                    _       => tokens.push(Token::Ident(word)),
                }
            }
            c => return Err(EvaluationError::Expression(format!("unexpected character: '{c}'"))),
        }
    }
    Ok(tokens)
}

// ---------------------------------------------------------------------------
// AST
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Bool(bool),
    Str(String),
    BinOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    UnOp  { op: UnOp, operand: Box<Expr> },
}

#[derive(Debug, Clone, Copy)]
enum BinOp { Add, Sub, Mul, Div, Mod, EqEq, Neq, Lt, Gt, LtEq, GtEq, And, Or }

#[derive(Debug, Clone, Copy)]
enum UnOp { Not, Neg }

// ---------------------------------------------------------------------------
// Parser (recursive descent, Pratt-style precedence)
// ---------------------------------------------------------------------------

struct Parser<'a> {
    tokens: &'a [Token],
    pos:    usize,
    depth:  usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self { Parser { tokens, pos: 0, depth: 0 } }

    fn peek(&self) -> Option<&Token> { self.tokens.get(self.pos) }

    fn consume(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn inc_depth(&mut self) -> Result<(), EvaluationError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            Err(EvaluationError::Expression(format!("AST depth limit ({MAX_DEPTH}) exceeded")))
        } else {
            Ok(())
        }
    }

    fn dec_depth(&mut self) { if self.depth > 0 { self.depth -= 1; } }

    // expr → or_expr
    fn parse_expr(&mut self) -> Result<Expr, EvaluationError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, EvaluationError> {
        let mut left = self.parse_and()?;
        while self.peek() == Some(&Token::OrOr) {
            self.consume();
            self.inc_depth()?;
            let right = self.parse_and()?;
            self.dec_depth();
            left = Expr::BinOp { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, EvaluationError> {
        let mut left = self.parse_equality()?;
        while self.peek() == Some(&Token::AndAnd) {
            self.consume();
            self.inc_depth()?;
            let right = self.parse_equality()?;
            self.dec_depth();
            left = Expr::BinOp { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, EvaluationError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Some(Token::EqEq)  => BinOp::EqEq,
                Some(Token::BangEq)=> BinOp::Neq,
                _ => break,
            };
            self.consume();
            self.inc_depth()?;
            let right = self.parse_comparison()?;
            self.dec_depth();
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, EvaluationError> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.peek() {
                Some(Token::Lt)   => BinOp::Lt,
                Some(Token::Gt)   => BinOp::Gt,
                Some(Token::LtEq) => BinOp::LtEq,
                Some(Token::GtEq) => BinOp::GtEq,
                _ => break,
            };
            self.consume();
            self.inc_depth()?;
            let right = self.parse_add()?;
            self.dec_depth();
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, EvaluationError> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus)  => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.consume();
            self.inc_depth()?;
            let right = self.parse_mul()?;
            self.dec_depth();
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, EvaluationError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star)    => BinOp::Mul,
                Some(Token::Slash)   => BinOp::Div,
                Some(Token::Percent) => BinOp::Mod,
                _ => break,
            };
            self.consume();
            self.inc_depth()?;
            let right = self.parse_unary()?;
            self.dec_depth();
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, EvaluationError> {
        match self.peek() {
            Some(Token::Bang) => {
                self.consume();
                self.inc_depth()?;
                let e = self.parse_unary()?;
                self.dec_depth();
                Ok(Expr::UnOp { op: UnOp::Not, operand: Box::new(e) })
            }
            Some(Token::Minus) => {
                self.consume();
                self.inc_depth()?;
                let e = self.parse_unary()?;
                self.dec_depth();
                Ok(Expr::UnOp { op: UnOp::Neg, operand: Box::new(e) })
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, EvaluationError> {
        match self.consume().cloned() {
            Some(Token::Number(n)) => Ok(Expr::Num(n)),
            Some(Token::Bool(b))   => Ok(Expr::Bool(b)),
            Some(Token::Str(s))    => Ok(Expr::Str(s)),
            Some(Token::Ident(_))  => {
                // Should not reach here — caller must resolve identifiers first
                Err(EvaluationError::Expression("unresolved identifier in expression (bug: pre-resolve fields)".into()))
            }
            Some(Token::LParen) => {
                self.inc_depth()?;
                let e = self.parse_expr()?;
                self.dec_depth();
                match self.consume() {
                    Some(Token::RParen) => Ok(e),
                    _ => Err(EvaluationError::Expression("expected closing ')'".into())),
                }
            }
            Some(t) => Err(EvaluationError::Expression(format!("unexpected token: {t:?}"))),
            None    => Err(EvaluationError::Expression("unexpected end of expression".into())),
        }
    }
}

// ---------------------------------------------------------------------------
// Evaluator
// ---------------------------------------------------------------------------

fn eval_expr(expr: &Expr) -> Result<Value, EvaluationError> {
    match expr {
        Expr::Num(n)  => Ok(Value::Number(serde_json::Number::from_f64(*n)
            .ok_or(EvaluationError::Overflow)?)),
        Expr::Bool(b) => Ok(Value::Bool(*b)),
        Expr::Str(s)  => Ok(Value::String(s.clone())),

        Expr::UnOp { op, operand } => {
            let v = eval_expr(operand)?;
            match op {
                UnOp::Not => Ok(Value::Bool(!truthy(&v))),
                UnOp::Neg => {
                    let n = as_f64_val(&v)?;
                    Ok(Value::Number(serde_json::Number::from_f64(-n).ok_or(EvaluationError::Overflow)?))
                }
            }
        }

        Expr::BinOp { op, left, right } => {
            let lv = eval_expr(left)?;
            let rv = eval_expr(right)?;
            match op {
                BinOp::Add => {
                    // String concatenation if either side is a string
                    match (&lv, &rv) {
                        (Value::String(a), _) => Ok(Value::String(format!("{a}{}", value_to_str(&rv)))),
                        (_, Value::String(b)) => Ok(Value::String(format!("{}{b}", value_to_str(&lv)))),
                        _ => {
                            let r = as_f64_val(&lv)? + as_f64_val(&rv)?;
                            Ok(Value::Number(serde_json::Number::from_f64(r).ok_or(EvaluationError::Overflow)?))
                        }
                    }
                }
                BinOp::Sub => num_binop(lv, rv, |a, b| a - b),
                BinOp::Mul => num_binop(lv, rv, |a, b| a * b),
                BinOp::Div => {
                    let b = as_f64_val(&rv)?;
                    if b == 0.0 { return Err(EvaluationError::Expression("division by zero".into())); }
                    num_binop(lv, rv, |a, b| a / b)
                }
                BinOp::Mod => {
                    let b = as_f64_val(&rv)?;
                    if b == 0.0 { return Err(EvaluationError::Expression("modulo by zero".into())); }
                    num_binop(lv, rv, |a, b| a % b)
                }
                BinOp::EqEq  => Ok(Value::Bool(values_eq(&lv, &rv))),
                BinOp::Neq   => Ok(Value::Bool(!values_eq(&lv, &rv))),
                BinOp::Lt    => cmp_binop(&lv, &rv, std::cmp::Ordering::Less, false),
                BinOp::Gt    => cmp_binop(&lv, &rv, std::cmp::Ordering::Greater, false),
                BinOp::LtEq  => cmp_binop(&lv, &rv, std::cmp::Ordering::Less, true),
                BinOp::GtEq  => cmp_binop(&lv, &rv, std::cmp::Ordering::Greater, true),
                BinOp::And   => Ok(Value::Bool(truthy(&lv) && truthy(&rv))),
                BinOp::Or    => Ok(Value::Bool(truthy(&lv) || truthy(&rv))),
            }
        }
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b)   => *b,
        Value::Null      => false,
        Value::Number(n) => n.as_f64().map(|f| f != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        _                => true,
    }
}

fn as_f64_val(v: &Value) -> Result<f64, EvaluationError> {
    v.as_f64().ok_or_else(|| EvaluationError::Expression(format!("expected number, got {v}")))
}

fn value_to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) =>
            x.as_f64().zip(y.as_f64()).map(|(a,b)| (a-b).abs() < f64::EPSILON).unwrap_or(false),
        _ => a == b,
    }
}

fn num_binop(a: Value, b: Value, f: impl Fn(f64,f64)->f64) -> Result<Value, EvaluationError> {
    let r = f(as_f64_val(&a)?, as_f64_val(&b)?);
    if r.is_nan() || r.is_infinite() { return Err(EvaluationError::Overflow); }
    Ok(Value::Number(serde_json::Number::from_f64(r).ok_or(EvaluationError::Overflow)?))
}

fn cmp_binop(a: &Value, b: &Value, ord: std::cmp::Ordering, or_equal: bool) -> Result<Value, EvaluationError> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (xf, yf) = (x.as_f64().unwrap_or(0.0), y.as_f64().unwrap_or(0.0));
            let result = xf.partial_cmp(&yf) == Some(ord) || (or_equal && (xf - yf).abs() < f64::EPSILON);
            Ok(Value::Bool(result))
        }
        (Value::String(x), Value::String(y)) => {
            let result = x.cmp(y) == ord || (or_equal && x == y);
            Ok(Value::Bool(result))
        }
        _ => Err(EvaluationError::Expression(format!("cannot compare {a} and {b}"))),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Evaluate a template string like `"{{ annual_income * 3 }}"`.
///
/// `context` is used to resolve any identifiers remaining in the expression.
/// Returns the evaluated `Value`.
pub fn eval_template(template: &str, context: &Value) -> Result<Value, EvaluationError> {
    let inner = extract_template(template)
        .ok_or_else(|| EvaluationError::Expression(format!("not a template: {template}")))?;

    // First pass: tokenize and substitute identifiers with their resolved values
    let raw_tokens = tokenize(inner)?;
    let resolved = resolve_tokens(raw_tokens, context)?;

    let mut parser = Parser::new(&resolved);
    let ast = parser.parse_expr()?;
    if parser.pos < parser.tokens.len() {
        return Err(EvaluationError::Expression("unexpected tokens after expression".into()));
    }
    eval_expr(&ast)
}

/// Extract the inner expression from `{{ ... }}`.
pub fn extract_template(s: &str) -> Option<&str> {
    let s = s.trim();
    if s.starts_with("{{") && s.ends_with("}}") {
        Some(s[2..s.len()-2].trim())
    } else {
        None
    }
}

/// Replace `Ident` tokens with their resolved concrete values from context.
fn resolve_tokens(tokens: Vec<Token>, context: &Value) -> Result<Vec<Token>, EvaluationError> {
    let mut out = Vec::with_capacity(tokens.len());
    for tok in tokens {
        match tok {
            Token::Ident(ref path) => {
                let val = crate::resolver::resolve_owned(context, path);
                let replacement = match &val {
                    Value::Number(n) => Token::Number(n.as_f64().unwrap_or(0.0)),
                    Value::Bool(b)   => Token::Bool(*b),
                    Value::String(s) => Token::Str(s.clone()),
                    Value::Null      => Token::Number(0.0),
                    _ => return Err(EvaluationError::Expression(
                        format!("field '{path}' resolved to non-scalar type"))),
                };
                out.push(replacement);
            }
            other => out.push(other),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_multiply() {
        let ctx = json!({ "income": 60000 });
        let result = eval_template("{{ income * 3 }}", &ctx).unwrap();
        assert_eq!(result, json!(180000.0));
    }

    #[test]
    fn string_concat() {
        let ctx = json!({ "first": "Hello", "last": "World" });
        let result = eval_template("{{ first + ' ' + last }}", &ctx).unwrap();
        assert_eq!(result, json!("Hello World"));
    }

    #[test]
    fn boolean_logic() {
        let ctx = json!({});
        let result = eval_template("{{ true && false }}", &ctx).unwrap();
        assert_eq!(result, json!(false));
    }

    #[test]
    fn depth_limit() {
        let ctx = json!({});
        // Deeply nested expression
        let expr = "{{ ((((((((((((((((1 + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) + 1) }}";
        let result = eval_template(expr, &ctx);
        assert!(result.is_err(), "should hit depth limit");
    }
}
