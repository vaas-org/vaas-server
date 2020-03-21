use slog::o;
extern crate slog_term;

use slog::Drain;

pub fn logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    return slog::Logger::root(drain, o!())
}
