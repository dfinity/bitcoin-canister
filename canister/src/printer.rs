use std::panic;

/// Sets a custom panic hook, uses debug.trace
fn set_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let file = info.location().unwrap().file();
        let line = info.location().unwrap().line();
        let col = info.location().unwrap().column();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let err_info = format!("Panicked at '{}', {}:{}:{}", msg, file, line, col);
        ic_cdk::api::print(&err_info);
        ic_cdk::api::trap(&err_info);
    }));
}

/// Sets a custom panic hook.
pub fn hook() {
    set_panic_hook();
}
