mod x;

struct XOpts {
    vt: u8,
}

fn main() {
    println!("{:?}", *x::auth::Cookie::build().unwrap());
}
