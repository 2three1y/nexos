//! Calculator: type an expression, get the answer in words-friendly text.
//! Supports + - * / % ^, brackets, unary minus, sqrt(), pi and ans.
//! no_std has no float maths library, so sqrt and powers are done by hand.

use super::ui;
use crate::println;
use alloc::format;
use alloc::string::String;

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    ans: f64,
}

type R = Result<f64, &'static str>;

impl<'a> Parser<'a> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i] == b' ' {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Option<u8> {
        self.ws();
        self.s.get(self.i).copied()
    }
    fn expr(&mut self) -> R {
        let mut v = self.term()?;
        while let Some(c) = self.peek() {
            match c {
                b'+' => { self.i += 1; v += self.term()?; }
                b'-' => { self.i += 1; v -= self.term()?; }
                _ => break,
            }
        }
        Ok(v)
    }
    fn term(&mut self) -> R {
        let mut v = self.power()?;
        while let Some(c) = self.peek() {
            match c {
                b'*' | b'x' => { self.i += 1; v *= self.power()?; }
                b'/' => {
                    self.i += 1;
                    let d = self.power()?;
                    if d == 0.0 { return Err("can't divide by zero"); }
                    v /= d;
                }
                b'%' => {
                    self.i += 1;
                    let d = self.power()?;
                    if d == 0.0 { return Err("can't divide by zero"); }
                    v -= trunc(v / d) * d;
                }
                _ => break,
            }
        }
        Ok(v)
    }
    fn power(&mut self) -> R {
        let base = self.unary()?;
        if self.peek() == Some(b'^') {
            self.i += 1;
            let e = self.power()?;
            if e != trunc(e) || e.abs() > 1000.0 {
                return Err("powers must be whole numbers up to 1000");
            }
            let mut r = 1.0;
            for _ in 0..(e.abs() as u32) { r *= base; }
            return Ok(if e < 0.0 { 1.0 / r } else { r });
        }
        Ok(base)
    }
    fn unary(&mut self) -> R {
        match self.peek() {
            Some(b'-') => { self.i += 1; Ok(-self.unary()?) }
            Some(b'+') => { self.i += 1; self.unary() }
            _ => self.atom(),
        }
    }
    fn atom(&mut self) -> R {
        match self.peek() {
            Some(b'(') => {
                self.i += 1;
                let v = self.expr()?;
                if self.peek() != Some(b')') { return Err("a bracket is not closed"); }
                self.i += 1;
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.i;
                while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
                    self.i += 1;
                }
                parse_num(core::str::from_utf8(&self.s[start..self.i]).unwrap_or(""))
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.i;
                while self.i < self.s.len() && self.s[self.i].is_ascii_alphabetic() {
                    self.i += 1;
                }
                match &self.s[start..self.i] {
                    b"pi" => Ok(core::f64::consts::PI),
                    b"ans" => Ok(self.ans),
                    b"sqrt" => {
                        let v = self.atom()?;
                        if v < 0.0 { return Err("no square root of a negative number"); }
                        Ok(sqrt(v))
                    }
                    _ => Err("unknown word; try sqrt, pi or ans"),
                }
            }
            Some(_) => Err("I don't understand that symbol"),
            None => Err("the sum ends too early"),
        }
    }
}

fn trunc(v: f64) -> f64 {
    if v.abs() >= 9.0e15 { v } else { (v as i64) as f64 }
}

fn sqrt(v: f64) -> f64 {
    if v == 0.0 { return 0.0; }
    let mut x = if v > 1.0 { v / 2.0 } else { 1.0 };
    for _ in 0..60 { x = 0.5 * (x + v / x); }
    x
}

fn parse_num(t: &str) -> R {
    let (int, frac) = t.split_once('.').unwrap_or((t, ""));
    if int.is_empty() && frac.is_empty() { return Err("a number is missing"); }
    let mut v = 0.0f64;
    for c in int.bytes() { v = v * 10.0 + (c - b'0') as f64; }
    let mut scale = 0.1;
    for c in frac.bytes() {
        if c == b'.' { return Err("a number has two decimal points"); }
        v += (c - b'0') as f64 * scale;
        scale /= 10.0;
    }
    Ok(v)
}

/// Format without float noise: whole numbers plainly, otherwise up to 6 decimals.
pub fn fmt_num(v: f64) -> String {
    if v != v || v.abs() > 1.0e15 { return String::from("too big to show"); }
    let r = trunc(v * 1_000_000.0 + if v < 0.0 { -0.5 } else { 0.5 }) / 1_000_000.0;
    if r == trunc(r) { return format!("{}", r as i64); }
    let s = format!("{:.6}", r);
    String::from(s.trim_end_matches('0').trim_end_matches('.'))
}

pub fn eval(src: &str, ans: f64) -> R {
    let mut p = Parser { s: src.as_bytes(), i: 0, ans };
    let v = p.expr()?;
    if p.peek().is_some() { return Err("there is something extra at the end"); }
    Ok(v)
}

fn help() {
    ui::heading("Calculator help");
    println!("Type a sum and press Enter, for example 12 * (3 + 4).");
    println!("+ add, - subtract, * or x multiply, / divide.");
    println!("% remainder, ^ power, sqrt 16, pi.");
    println!("ans is the last answer, so ans / 2 works.");
    println!("q quits.");
}

pub fn run() {
    ui::title("Calculator", "1.0", "Type a sum like 12 * (3 + 4) and press Enter.");
    let mut ans = 0.0;
    loop {
        let Some(line) = ui::read_line("calc> ") else { break };
        match line.as_str() {
            "" => {}
            l if ui::is_quit(l) => break,
            l if ui::is_help(l) => help(),
            l => match eval(l, ans) {
                Ok(v) => {
                    ans = v;
                    ui::done(format_args!("= {}", fmt_num(v)));
                }
                Err(e) => ui::error(format_args!("{}.", e)),
            },
        }
    }
    ui::closed("Calculator");
}
