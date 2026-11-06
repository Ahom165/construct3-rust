//! Évaluateur d'expressions compatible Construct 3 (sous-ensemble).
//! Supporte : littéraux numériques/chaînes, YES/NO, variables, `Self.X`,
//! `Objet.Propriété`, `Objet.Comportement.Propriété`, opérateurs arithmétiques,
//! comparaisons, concaténation `&`, et les fonctions usuelles C3.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Num(f64),
    Str(String),
    Bool(bool),
}

impl Val {
    pub fn num(&self) -> f64 {
        match self {
            Val::Num(n) => *n,
            Val::Str(s) => s.trim().parse().unwrap_or(0.0),
            Val::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }
    pub fn text(&self) -> String {
        match self {
            Val::Num(n) => fmt_num(*n),
            Val::Str(s) => s.clone(),
            Val::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
        }
    }
    pub fn truthy(&self) -> bool {
        match self {
            Val::Num(n) => *n != 0.0,
            Val::Str(s) => !s.is_empty() && s != "0",
            Val::Bool(b) => *b,
        }
    }
}

pub fn fmt_num(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", (n * 1e6).round() / 1e6)
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Num(f64),
    Str(String),
    Var(String),
    Prop(Vec<String>), // chemin: ["Self", "Platform", "JumpStrength"] ou ["Enemy", "X"]
    Call(String, Vec<Expr>),
    Unary(char, Box<Expr>),
    Bin(String, Box<Expr>, Box<Expr>),
}

// ---------- Tokenizer ----------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(String),
    LParen,
    RParen,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let mut toks = Vec::new();
    let b: Vec<char> = src.chars().collect();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit()
            || (c == '.'
                && i + 1 < b.len()
                && b[i + 1].is_ascii_digit())
        {
            let start = i;
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == '.') {
                i += 1;
            }
            let s: String = b[start..i].iter().collect();
            toks.push(Tok::Num(s.parse().map_err(|_| format!("nombre invalide: {s}"))?));
            continue;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < b.len() && b[i] != quote {
                if b[i] == '\\' && i + 1 < b.len() {
                    i += 1;
                    match b[i] {
                        'n' => s.push('\n'),
                        't' => s.push('\t'),
                        other => s.push(other),
                    }
                } else {
                    s.push(b[i]);
                }
                i += 1;
            }
            if i >= b.len() {
                return Err("chaîne non terminée".into());
            }
            i += 1;
            toks.push(Tok::Str(s));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < b.len() && (b[i].is_alphanumeric() || b[i] == '_' || b[i] == '.') {
                // identifiants pouvant contenir des points (Objet.Propriété)
                if b[i] == ' ' {
                    break;
                }
                // pas de point final isolé suivi d'espace (ex: fin de phrase)
                if b[i] == '.' && (i + 1 >= b.len() || !b[i + 1].is_alphanumeric() && b[i + 1] != '_') {
                    break;
                }
                i += 1;
            }
            let s: String = b[start..i].iter().collect();
            toks.push(Tok::Ident(s));
            continue;
        }
        // opérateurs multi-caractères
        let rest: String = b[i..].iter().take(2).collect();
        if rest == "<=" || rest == ">=" || rest == "==" || rest == "!=" || rest == "<>" {
            toks.push(Tok::Op(rest));
            i += 2;
            continue;
        }
        match c {
            '(' => toks.push(Tok::LParen),
            ')' => toks.push(Tok::RParen),
            ',' => toks.push(Tok::Comma),
            '+' | '-' | '*' | '/' | '%' | '^' | '&' | '<' | '>' | '=' => {
                toks.push(Tok::Op(c.to_string()))
            }
            _ => return Err(format!("caractère inattendu: {c}")),
        }
        i += 1;
    }
    Ok(toks)
}

// ---------- Parser ----------

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }
    fn eat_op(&mut self, ops: &[&str]) -> Option<String> {
        if let Some(Tok::Op(o)) = self.peek() {
            if ops.contains(&o.as_str()) {
                return self.next().map(|t| match t {
                    Tok::Op(s) => s,
                    _ => String::new(),
                });
            }
        }
        None
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_concat()
    }

    fn parse_concat(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_cmp()?;
        while let Some(op) = self.eat_op(&["&"]) {
            let rhs = self.parse_cmp()?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_cmp(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_add()?;
        while let Some(op) = self.eat_op(&["=", "==", "!=", "<>", "<", "<=", ">", ">="]) {
            let norm = match op.as_str() {
                "==" => "=",
                "<>" => "!=",
                o => o,
            };
            let rhs = self.parse_add()?;
            lhs = Expr::Bin(norm.to_string(), Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_mul()?;
        while let Some(op) = self.eat_op(&["+", "-"]) {
            let rhs = self.parse_mul()?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, String> {
        let mut lhs = self.parse_unary()?;
        while let Some(op) = self.eat_op(&["*", "/", "%"]) {
            let rhs = self.parse_unary()?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if let Some(op) = self.eat_op(&["-", "+"]) {
            let e = self.parse_unary()?;
            return Ok(Expr::Unary(if op == "-" { '-' } else { '+' }, Box::new(e)));
        }
        self.parse_pow()
    }

    fn parse_pow(&mut self) -> Result<Expr, String> {
        let base = self.parse_atom()?;
        if let Some(op) = self.eat_op(&["^"]) {
            let exp = self.parse_unary()?;
            return Ok(Expr::Bin(op, Box::new(base), Box::new(exp)));
        }
        Ok(base)
    }

    fn parse_atom(&mut self) -> Result<Expr, String> {
        match self.next() {
            Some(Tok::Num(n)) => Ok(Expr::Num(n)),
            Some(Tok::Str(s)) => Ok(Expr::Str(s)),
            Some(Tok::LParen) => {
                let e = self.parse_expr()?;
                match self.next() {
                    Some(Tok::RParen) => Ok(e),
                    _ => Err("parenthèse fermante attendue".into()),
                }
            }
            Some(Tok::Ident(name)) => {
                let lname = name.to_lowercase();
                if lname == "yes" {
                    return Ok(Expr::Num(1.0));
                }
                if lname == "no" {
                    return Ok(Expr::Num(0.0));
                }
                // appel de fonction ?
                if let Some(Tok::LParen) = self.peek() {
                    self.next();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Some(Tok::RParen)) {
                        loop {
                            args.push(self.parse_expr()?);
                            match self.next() {
                                Some(Tok::Comma) => continue,
                                Some(Tok::RParen) => break,
                                _ => return Err("virgule ou ) attendue".into()),
                            }
                        }
                    } else {
                        self.next(); // RParen
                    }
                    return Ok(Expr::Call(name, args));
                }
                Ok(Expr::Var(name))
            }
            Some(Tok::Op(o)) if o == "-" => {
                let e = self.parse_unary()?;
                Ok(Expr::Unary('-', Box::new(e)))
            }
            other => Err(format!("expression inattendue: {other:?}")),
        }
    }
}

/// Aplatit les chaînes de `.` en chemin de propriétés.
fn flatten(e: &Expr, out: &mut Vec<String>) -> bool {
    match e {
        Expr::Var(v) => {
            out.push(v.clone());
            true
        }
        Expr::Prop(p) => {
            for part in p {
                out.push(part.clone());
            }
            true
        }
        _ => false,
    }
}

/// Post-traitement: transforme Var.Var.Var... en Prop([...])
pub fn postprocess(e: Expr) -> Expr {
    match e {
        Expr::Var(v) if v.contains('.') => {
            Expr::Prop(v.split('.').map(|s| s.to_string()).collect())
        }
        Expr::Bin(op, a, b) => Expr::Bin(op, Box::new(postprocess(*a)), Box::new(postprocess(*b))),
        Expr::Unary(op, a) => Expr::Unary(op, Box::new(postprocess(*a))),
        Expr::Call(name, args) => Expr::Call(
            name,
            args.into_iter().map(postprocess).collect(),
        ),
        other => {
            // tente de réduire une chaîne de Prop/Var
            let mut path = Vec::new();
            if flatten_chain(&other, &mut path) && path.len() > 1 {
                Expr::Prop(path)
            } else {
                other
            }
        }
    }
}

fn flatten_chain(e: &Expr, out: &mut Vec<String>) -> bool {
    match e {
        Expr::Var(v) => {
            out.push(v.clone());
            true
        }
        _ => false,
    }
}

pub fn parse(src: &str) -> Result<Expr, String> {
    let toks = tokenize(src)?;
    if toks.is_empty() {
        return Ok(Expr::Num(0.0));
    }
    let mut p = Parser { toks, pos: 0 };
    let e = p.parse_expr()?;
    if p.pos != p.toks.len() {
        return Err(format!("tokens restants à la position {}", p.pos));
    }
    // rebuild chain: gérer Self.Platform.JumpStrength via un parsing spécial
    Ok(postprocess(rebuild_chains(e)))
}

/// Fusionne les chaînes Ident(.Ident)+ en Expr::Prop
fn rebuild_chains(e: Expr) -> Expr {
    match e {
        Expr::Bin(op, a, b) => Expr::Bin(op, Box::new(rebuild_chains(*a)), Box::new(rebuild_chains(*b))),
        Expr::Unary(op, a) => Expr::Unary(op, Box::new(rebuild_chains(*a))),
        Expr::Call(name, args) => Expr::Call(name, args.into_iter().map(rebuild_chains).collect()),
        Expr::Var(_) => e,
        Expr::Num(_) | Expr::Str(_) => e,
        Expr::Prop(_) => e,
    }
}

// ---------- Environnement d'évaluation ----------

pub trait Env {
    fn var(&mut self, _name: &str) -> Option<Val> {
        None
    }
    fn prop(&mut self, _obj: Option<&str>, _path: &[String]) -> Option<Val> {
        None
    }
    fn call(&mut self, _name: &str, _args: &[Val]) -> Option<Val> {
        None
    }
    fn warn(&mut self, _msg: &str) {}
}

pub fn eval(e: &Expr, env: &mut dyn Env) -> Val {
    match e {
        Expr::Num(n) => Val::Num(*n),
        Expr::Str(s) => Val::Str(s.clone()),
        Expr::Var(name) => {
            if name.eq_ignore_ascii_case("true") {
                return Val::Bool(true);
            }
            if name.eq_ignore_ascii_case("false") {
                return Val::Bool(false);
            }
            if let Some(v) = env.var(name) {
                return v;
            }
            if let Some(v) = env.prop(None, std::slice::from_ref(name)) {
                return v;
            }
            env.warn(&format!("identifiant inconnu: {name} (0 supposé)"));
            Val::Num(0.0)
        }
        Expr::Prop(path) => {
            let (head, rest) = path.split_first().unwrap();
            if head.eq_ignore_ascii_case("self") {
                if let Some(v) = env.prop(None, rest) {
                    return v;
                }
            } else if let Some(v) = env.prop(Some(head), rest) {
                return v;
            }
            // variable composée ? tente le nom complet
            let full = path.join(".");
            if let Some(v) = env.var(&full) {
                return v;
            }
            env.warn(&format!("propriété inconnue: {full} (0 supposé)"));
            Val::Num(0.0)
        }
        Expr::Call(name, args) => {
            let vals: Vec<Val> = args.iter().map(|a| eval(a, env)).collect();
            if let Some(v) = env.call(name, &vals) {
                return v;
            }
            eval_builtin(name, &vals, env)
        }
        Expr::Unary(op, a) => {
            let v = eval(a, env).num();
            match op {
                '-' => Val::Num(-v),
                _ => Val::Num(v),
            }
        }
        Expr::Bin(op, a, b) => {
            let l = eval(a, env);
            let r = eval(b, env);
            match op.as_str() {
                "&" => Val::Str(format!("{}{}", l.text(), r.text())),
                "+" => Val::Num(l.num() + r.num()),
                "-" => Val::Num(l.num() - r.num()),
                "*" => Val::Num(l.num() * r.num()),
                "/" => {
                    let d = r.num();
                    if d == 0.0 {
                        Val::Num(0.0)
                    } else {
                        Val::Num(l.num() / d)
                    }
                }
                "%" => {
                    let d = r.num();
                    if d == 0.0 {
                        Val::Num(0.0)
                    } else {
                        Val::Num(l.num() % d)
                    }
                }
                "^" => Val::Num(l.num().powf(r.num())),
                "=" => Val::Bool(vals_eq(&l, &r)),
                "!=" => Val::Bool(!vals_eq(&l, &r)),
                "<" => Val::Bool(l.num() < r.num()),
                "<=" => Val::Bool(l.num() <= r.num()),
                ">" => Val::Bool(l.num() > r.num()),
                ">=" => Val::Bool(l.num() >= r.num()),
                other => {
                    env.warn(&format!("opérateur non géré: {other}"));
                    Val::Num(0.0)
                }
            }
        }
    }
}

fn vals_eq(a: &Val, b: &Val) -> bool {
    match (a, b) {
        (Val::Str(x), Val::Str(y)) => x == y,
        _ => (a.num() - b.num()).abs() < 1e-9,
    }
}

fn eval_builtin(name: &str, args: &[Val], env: &mut dyn Env) -> Val {
    let n = |i: usize| args.get(i).map(|v| v.num()).unwrap_or(0.0);
    let s = |i: usize| args.get(i).map(|v| v.text()).unwrap_or_default();
    match name.to_lowercase().as_str() {
        "floor" => Val::Num(n(0).floor()),
        "ceil" => Val::Num(n(0).ceil()),
        "round" => Val::Num(n(0).round()),
        "abs" => Val::Num(n(0).abs()),
        "sqrt" => Val::Num(n(0).sqrt()),
        "sin" => Val::Num(n(0).to_radians().sin()),
        "cos" => Val::Num(n(0).to_radians().cos()),
        "tan" => Val::Num(n(0).to_radians().tan()),
        "min" => Val::Num(args.iter().map(|v| v.num()).fold(f64::INFINITY, f64::min)),
        "max" => Val::Num(args.iter().map(|v| v.num()).fold(f64::NEG_INFINITY, f64::max)),
        "clamp" => Val::Num(n(0).clamp(n(1), n(2))),
        "lerp" => Val::Num(n(0) + (n(1) - n(0)) * n(2)),
        "int" => Val::Num(n(0).trunc()),
        "float" => Val::Num(n(0)),
        "number" => Val::Num(args.get(0).map(|v| v.num()).unwrap_or(0.0)),
        "str" => Val::Str(s(0)),
        "len" => Val::Num(s(0).chars().count() as f64),
        "left" => Val::Str(s(0).chars().take(n(1) as usize).collect()),
        "right" => {
            let t = s(0);
            let k = (n(1) as usize).min(t.len());
            Val::Str(t[t.len() - k..].to_string())
        }
        "mid" => {
            let t: Vec<char> = s(0).chars().collect();
            let st = (n(1) as usize).min(t.len());
            let cnt = (n(2) as usize).min(t.len().saturating_sub(st));
            Val::Str(t[st..st + cnt].iter().collect())
        }
        "find" => Val::Num(
            s(0)
                .find(&s(1))
                .map(|i| i as f64)
                .unwrap_or(-1.0),
        ),
        "choose" => {
            if args.is_empty() {
                Val::Num(0.0)
            } else {
                let i = (env_rand(env) * args.len() as f64) as usize % args.len();
                args[i].clone()
            }
        }
        "rgbex" => {
            let r = (n(0).clamp(0.0, 100.0) / 100.0 * 255.0) as u32;
            let g = (n(1).clamp(0.0, 100.0) / 100.0 * 255.0) as u32;
            let b = (n(2).clamp(0.0, 100.0) / 100.0 * 255.0) as u32;
            Val::Num(((r << 16) | (g << 8) | b) as f64)
        }
        "rgb" => {
            let r = n(0).clamp(0.0, 255.0) as u32;
            let g = n(1).clamp(0.0, 255.0) as u32;
            let b = n(2).clamp(0.0, 255.0) as u32;
            Val::Num(((r << 16) | (g << 8) | b) as f64)
        }
        _ => {
            env.warn(&format!("fonction inconnue: {name} (0 supposé)"));
            Val::Num(0.0)
        }
    }
}

fn env_rand(env: &mut dyn Env) -> f64 {
    env.call("rand", &[]).map(|v| v.num()).unwrap_or(0.5)
}

/// Évalue directement une source texte.
pub fn eval_str(src: &str, env: &mut dyn Env) -> Val {
    match parse(src) {
        Ok(e) => eval(&e, env),
        Err(err) => {
            env.warn(&format!("expression « {src} » invalide: {err}"));
            Val::Num(0.0)
        }
    }
}

/// Convertit une valeur JSON brute (paramètre) en Val.
pub fn json_to_val(v: &serde_json::Value) -> Val {
    match v {
        serde_json::Value::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Val::Str(s.clone()),
        serde_json::Value::Bool(b) => Val::Bool(*b),
        serde_json::Value::Null => Val::Num(0.0),
        other => Val::Str(other.to_string()),
    }
}

pub type Vars = HashMap<String, Val>;
