#[cfg(windows)]
extern crate winres;

#[cfg(windows)]
fn main() {
    let res = winres::WindowsResource::new();
    res.compile().unwrap();
}

#[cfg(unix)]
fn main() {}
