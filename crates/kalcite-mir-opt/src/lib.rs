#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    C(u32),
    Add(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
}
pub fn fold(e: Expr) -> Expr {
    match e {
        Expr::Add(a, b) => match (fold(*a), fold(*b)) {
            (Expr::C(a), Expr::C(b)) => Expr::C(a.wrapping_add(b)),
            (a, b) => Expr::Add(Box::new(a), Box::new(b)),
        },
        Expr::Mul(a, b) => match (fold(*a), fold(*b)) {
            (Expr::C(a), Expr::C(b)) => Expr::C(a.wrapping_mul(b)),
            (a, b) => Expr::Mul(Box::new(a), Box::new(b)),
        },
        x => x,
    }
}
pub fn narrow(n: u32) -> u8 {
    if n <= 255 {
        8
    } else if n <= 65535 {
        16
    } else {
        32
    }
}
