#![allow(warnings)]

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Set, Say, Func, If, Else, While, For, In, Return,
    Ident(String), Str(String), Num(i64),
    Plus, Minus, Star, Slash,
    EqEq, NotEq, Lt, Gt, LtEq, GtEq,
    Assign, LBrace, RBrace, LParen, RParen, Comma,
    LBracket, RBracket, Dot,
    Eof,
}

#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub token: Token,
    pub line: usize,
}

pub fn tokenize(input: &str) -> Vec<TokenInfo> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    let mut line = 1;

    while let Some(&c) = chars.peek() {
        if c == '\n' {
            line += 1;
            chars.next();
        } else if c.is_whitespace() {
            chars.next();
        } else if c == '/' {
            chars.next();
            if chars.peek() == Some(&'/') {
                while let Some(&ch) = chars.peek() {
                    if ch == '\n' { break; }
                    chars.next();
                }
            } else {
                tokens.push(TokenInfo { token: Token::Slash, line });
            }
        } else if c.is_alphabetic() || c == '_' {
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_alphanumeric() || ch == '_' {
                    s.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            let token = match s.as_str() {
                "set" => Token::Set,
                "say" => Token::Say,
                "func" => Token::Func,
                "if" => Token::If,
                "else" => Token::Else,
                "while" => Token::While,
                "for" => Token::For,
                "in" => Token::In,
                "return" => Token::Return,
                _ => Token::Ident(s),
            };
            tokens.push(TokenInfo { token, line });
        } else if c.is_digit(10) {
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch.is_digit(10) {
                    s.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            tokens.push(TokenInfo { token: Token::Num(s.parse().unwrap()), line });
        } else if c == '"' {
            chars.next();
            let mut s = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '"' {
                    chars.next();
                    break;
                } else {
                    s.push(chars.next().unwrap());
                }
            }
            tokens.push(TokenInfo { token: Token::Str(s), line });
        } else {
            let t = match c {
                '=' => {
                    chars.next();
                    if chars.peek() == Some(&'=') { chars.next(); Token::EqEq }
                    else { Token::Assign }
                }
                '!' => {
                    chars.next();
                    if chars.peek() == Some(&'=') { chars.next(); Token::NotEq }
                    else { chars.next(); continue; }
                }
                '<' => {
                    chars.next();
                    if chars.peek() == Some(&'=') { chars.next(); Token::LtEq }
                    else { Token::Lt }
                }
                '>' => {
                    chars.next();
                    if chars.peek() == Some(&'=') { chars.next(); Token::GtEq }
                    else { Token::Gt }
                }
                '+' => { chars.next(); Token::Plus }
                '-' => { chars.next(); Token::Minus }
                '*' => { chars.next(); Token::Star }
                '{' => { chars.next(); Token::LBrace }
                '}' => { chars.next(); Token::RBrace }
                '(' => { chars.next(); Token::LParen }
                ')' => { chars.next(); Token::RParen }
                '[' => { chars.next(); Token::LBracket }
                ']' => { chars.next(); Token::RBracket }
                ',' => { chars.next(); Token::Comma }
                '.' => { chars.next(); Token::Dot }
                _ => { chars.next(); continue; }
            };
            tokens.push(TokenInfo { token: t, line });
        }
    }
    tokens.push(TokenInfo { token: Token::Eof, line });
    tokens
}