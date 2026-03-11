/// Terminal output helpers with TTY-gated colors.
///
/// Colors are applied only when stdout/stderr is a TTY and the `NO_COLOR`
/// environment variable is not set.
use owo_colors::OwoColorize;
use owo_colors::Stream;

/// Returns `true` when the given stream supports ANSI color output.
///
/// Conditions: stream is a TTY AND `NO_COLOR` env var is not set.
pub fn supports_color(stream: Stream) -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    match stream {
        Stream::Stdout => atty::is(atty::Stream::Stdout),
        Stream::Stderr => atty::is(atty::Stream::Stderr),
    }
}

/// Print an error message to stderr in red (when TTY).
pub fn print_error(msg: &str) {
    if supports_color(Stream::Stderr) {
        eprintln!("{}", msg.if_supports_color(Stream::Stderr, |t| t.red()));
    } else {
        eprintln!("{msg}");
    }
}

/// Print a success message to stdout in green (when TTY).
pub fn print_success(msg: &str) {
    if supports_color(Stream::Stdout) {
        println!("{}", msg.if_supports_color(Stream::Stdout, |t| t.green()));
    } else {
        println!("{msg}");
    }
}

/// Print an informational message to stdout (no color).
pub fn print_info(msg: &str) {
    println!("{msg}");
}

/// Print a warning message to stderr in yellow (when TTY).
pub fn print_warning(msg: &str) {
    if supports_color(Stream::Stderr) {
        eprintln!("{}", msg.if_supports_color(Stream::Stderr, |t| t.yellow()));
    } else {
        eprintln!("{msg}");
    }
}
