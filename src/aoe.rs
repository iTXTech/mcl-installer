use std::{fmt, process};

pub trait AbortOnError<T>: Sized {
    fn aoe(self) -> T;

    fn aoe_msg(self, msg: impl fmt::Display) -> T;
}

impl<T, E> AbortOnError<T> for Result<T, E>
where
    E: fmt::Display,
{
    #[track_caller]
    fn aoe(self) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("Error: {}", err),
        }
    }

    #[track_caller]
    fn aoe_msg(self, msg: impl fmt::Display) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("{}: {}", msg, err),
        }
    }
}

pub fn register() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.payload().downcast_ref::<String>();
        let loc = info.location();
        if let (Some(msg), Some(loc)) = (msg, loc) {
            eprintln!("[{}:{}:{}] {}", loc.file(), loc.line(), loc.column(), msg);
        } else {
            eprintln!("{}", info)
        }
        process::exit(1);
    }));
}
