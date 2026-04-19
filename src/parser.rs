#![allow(warnings)]
use crate::lexer::{Token, TokenInfo};

#[derive(Debug, Clone)]
pub enum Expr {
    Num(i64),
    Str(String),
    Ident(String),
    Binary(Box<Expr>, String, Box<Expr>),
    Call(String, Vec<Expr>),
    Array(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set(String, Expr),
    SetIndex(Expr, Expr, Expr),
    Say(Expr),
    Expr(Expr),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Vec<Stmt>), // New: for item in array
    Func(String, Vec<String>, Vec<Stmt>),
    Return(Option<Expr>),
}

pub struct Parser {
    tokens: Vec<TokenInfo>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<TokenInfo>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek_info(&self) -> &TokenInfo {
        static EOF: TokenInfo = TokenInfo { token: Token::Eof, line: 0 };
        self.tokens.get(self.pos).unwrap_or(&EOF)
    }

    fn peek(&self) -> &Token { &self.peek_info().token }
    fn line(&self) -> usize { self.peek_info().line }

    fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        self.pos += 1;
        t
    }

    fn match_token(&mut self, expected: std::mem::Discriminant<Token>) -> bool {
        if std::mem::discriminant(self.peek()) == expected { self.advance(); true }
        else { false }
    }

    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::Eof {
            match self.parse_stmt() {
                Ok(Some(stmt)) => stmts.push(stmt),
                Ok(None) => { self.advance(); }
                Err(e) => return Err(e),
            }
        }
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Option<Stmt>, String> {
        let start_line = self.line();
        match self.peek() {
            Token::Set => {
                self.advance();
                let mut target_expr = self.parse_primary()?;
                while self.peek() == &Token::LBracket {
                    self.advance();
                    let idx = self.parse_expr()?;
                    if !self.match_token(std::mem::discriminant(&Token::RBracket)) { return Err(format!("[Řádek {}] Očekáváno ']'", self.line())); }
                    target_expr = Expr::Index(Box::new(target_expr), Box::new(idx));
                }
                if !self.match_token(std::mem::discriminant(&Token::Assign)) { return Err(format!("[Řádek {}] Očekáváno '='", self.line())); }
                let val_expr = self.parse_expr()?;
                if let Expr::Ident(name) = target_expr { Ok(Some(Stmt::Set(name, val_expr))) }
                else if let Expr::Index(arr, idx) = target_expr { Ok(Some(Stmt::SetIndex(*arr, *idx, val_expr))) }
                else { Err(format!("[Řádek {}] Neplatný cíl přiřazení", start_line)) }
            }
            Token::Say => { self.advance(); let expr = self.parse_expr()?; Ok(Some(Stmt::Say(expr))) }
            Token::If => {
                self.advance(); let cond = self.parse_expr()?;
                if !self.match_token(std::mem::discriminant(&Token::LBrace)) { return Err(format!("[Řádek {}] Očekáváno '{{'", self.line())); }
                let mut then_branch = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof { if let Some(s) = self.parse_stmt()? { then_branch.push(s); } else { self.advance(); } }
                if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončený blok 'if'", start_line)); }
                self.advance();
                let mut else_branch = None;
                if self.peek() == &Token::Else {
                    self.advance();
                    if !self.match_token(std::mem::discriminant(&Token::LBrace)) { return Err(format!("[Řádek {}] Očekáváno '{{'", self.line())); }
                    let mut b = Vec::new();
                    while self.peek() != &Token::RBrace && self.peek() != &Token::Eof { if let Some(s) = self.parse_stmt()? { b.push(s); } else { self.advance(); } }
                    if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončený blok 'else'", start_line)); }
                    self.advance();
                    else_branch = Some(b);
                }
                Ok(Some(Stmt::If(cond, then_branch, else_branch)))
            }
            Token::While => {
                self.advance(); let cond = self.parse_expr()?;
                if !self.match_token(std::mem::discriminant(&Token::LBrace)) { return Err(format!("[Řádek {}] Očekáváno '{{'", self.line())); }
                let mut body = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof { if let Some(s) = self.parse_stmt()? { body.push(s); } else { self.advance(); } }
                if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončený blok 'while'", start_line)); }
                self.advance();
                Ok(Some(Stmt::While(cond, body)))
            }
            Token::For => {
                self.advance();
                let var_name = match self.advance() { Token::Ident(n) => n, _ => return Err(format!("[Řádek {}] Očekáváno jméno proměnné za 'for'", self.line())) };
                if !self.match_token(std::mem::discriminant(&Token::In)) { return Err(format!("[Řádek {}] Očekáváno 'in' za '{}'", self.line(), var_name)); }
                let iterable = self.parse_expr()?;
                if !self.match_token(std::mem::discriminant(&Token::LBrace)) { return Err(format!("[Řádek {}] Očekáváno '{{' za cyklem 'for'", self.line())); }
                let mut body = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof { if let Some(s) = self.parse_stmt()? { body.push(s); } else { self.advance(); } }
                if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončený blok 'for'", start_line)); }
                self.advance();
                Ok(Some(Stmt::For(var_name, iterable, body)))
            }
            Token::Func => {
                self.advance();
                let name = match self.advance() { Token::Ident(n) => n, _ => return Err(format!("[Řádek {}] Očekáván název funkce", self.line())) };
                if !self.match_token(std::mem::discriminant(&Token::LParen)) { return Err(format!("[Řádek {}] Očekáváno '('", self.line())); }
                let mut params = Vec::new();
                while self.peek() != &Token::RParen && self.peek() != &Token::Eof { if let Token::Ident(p) = self.advance() { params.push(p); } if self.peek() == &Token::Comma { self.advance(); } }
                if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončené parametry funkce", self.line())); }
                self.advance();
                if !self.match_token(std::mem::discriminant(&Token::LBrace)) { return Err(format!("[Řádek {}] Očekáváno '{{'", self.line())); }
                let mut body = Vec::new();
                while self.peek() != &Token::RBrace && self.peek() != &Token::Eof { if let Some(s) = self.parse_stmt()? { body.push(s); } else { self.advance(); } }
                if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončené tělo funkce", self.line())); }
                self.advance();
                Ok(Some(Stmt::Func(name, params, body)))
            }
            Token::Return => {
                self.advance();
                let mut expr = None;
                if self.peek() != &Token::RBrace && self.peek() != &Token::Eof { expr = self.parse_expr().ok(); }
                Ok(Some(Stmt::Return(expr)))
            }
            _ => { let expr = self.parse_expr()?; Ok(Some(Stmt::Expr(expr))) }
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, String> { self.parse_equality() }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_relational()?;
        while let Token::EqEq | Token::NotEq = self.peek() {
            let op = match self.advance() { Token::EqEq => "==", Token::NotEq => "!=", _ => unreachable!() }.to_string();
            let right = self.parse_relational()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;
        while let Token::Lt | Token::Gt | Token::LtEq | Token::GtEq = self.peek() {
            let op = match self.advance() { Token::Lt => "<", Token::Gt => ">", Token::LtEq => "<=", Token::GtEq => ">=", _ => unreachable!() }.to_string();
            let right = self.parse_additive()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        while let Token::Plus | Token::Minus = self.peek() {
            let op = match self.advance() { Token::Plus => "+", Token::Minus => "-", _ => unreachable!() }.to_string();
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        while let Token::Star | Token::Slash = self.peek() {
            let op = match self.advance() { Token::Star => "*", Token::Slash => "/", _ => unreachable!() }.to_string();
            let right = self.parse_unary()?;
            left = Expr::Binary(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        if self.peek() == &Token::Minus {
            self.advance();
            let right = self.parse_unary()?;
            return Ok(Expr::Binary(Box::new(Expr::Num(0)), "-".to_string(), Box::new(right)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        while self.peek() == &Token::LBracket {
            self.advance();
            let idx = self.parse_expr()?;
            if !self.match_token(std::mem::discriminant(&Token::RBracket)) { return Err(format!("[Řádek {}] Očekáváno ']'", self.line())); }
            expr = Expr::Index(Box::new(expr), Box::new(idx));
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Num(n) => { self.advance(); Ok(Expr::Num(n)) }
            Token::Str(s) => { self.advance(); Ok(Expr::Str(s)) }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                    items.push(self.parse_expr()?);
                    if self.peek() == &Token::Comma { self.advance(); }
                }
                if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Neukončené pole", self.line())); }
                self.advance();
                Ok(Expr::Array(items))
            }
            Token::Ident(name) => {
                self.advance();
                if self.peek() == &Token::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen && self.peek() != &Token::Eof {
                        args.push(self.parse_expr()?);
                        if self.peek() == &Token::Comma { self.advance(); }
                    }
                    if self.peek() == &Token::Eof { return Err(format!("[Řádek {}] Chybí ')'", self.line())); }
                    self.advance();
                    Ok(Expr::Call(name, args))
                } else { Ok(Expr::Ident(name)) }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                if !self.match_token(std::mem::discriminant(&Token::RParen)) { return Err(format!("[Řádek {}] Chybí ')'", self.line())); }
                Ok(expr)
            }
            t => Err(format!("[Řádek {}] Neočekávaný token: {:?}", self.line(), t)),
        }
    }
}