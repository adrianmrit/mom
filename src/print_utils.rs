#[cfg(test)]
#[path = "print_utils_test.rs"]
mod print_utils_test;

use colored::{Color, ColoredString, Colorize};

const PREFIX: &str = "[mom]";
pub(crate) const INFO_COLOR: Color = Color::BrightBlue;
pub(crate) const WARN_COLOR: Color = Color::BrightYellow;
pub(crate) const ERROR_COLOR: Color = Color::BrightRed;

pub trait MomOutput {
    /// Just adds the `[mom]` prefix to the given string, with no color.
    fn mom_just_prefix(&self) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The prefix will also take the given color.
    fn mom_prefix<S: Into<Color> + Clone>(&self, color: S) -> String;
    /// Adds the `[mom]` prefix to the given string. The whole string will have the given color.
    fn mom_colorize<S: Into<Color> + Clone>(&self, color: S) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The whole string will be blue.
    fn mom_info(&self) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The prefix will be blue.
    fn mom_prefix_info(&self) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The whole string will be yellow.
    fn mom_warn(&self) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The prefix will be yellow.
    fn mom_prefix_warn(&self) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The whole string will be red.
    fn mom_error(&self) -> String;
    /// Returns the given string with the `[mom]` prefix in each line. The prefix will be red.
    fn mom_prefix_error(&self) -> String;
}

impl MomOutput for str {
    fn mom_just_prefix(&self) -> String {
        let lines = self.split_inclusive('\n');

        let mut result = String::new();
        for line in lines {
            result.push_str(PREFIX);
            result.push(' ');
            result.push_str(line);
        }
        result
    }
    fn mom_prefix<S: Into<Color> + Clone>(&self, color: S) -> String {
        let lines = self.split_inclusive('\n');
        let prefix = PREFIX.color(color).to_string();

        let mut result = String::new();
        for line in lines {
            result.push_str(&prefix);
            result.push(' ');
            result.push_str(line);
        }
        result
    }

    fn mom_colorize<S: Into<Color> + Clone>(&self, color: S) -> String {
        let lines = self.split_inclusive('\n');

        let mut result = String::new();
        for line in lines {
            result.push_str(PREFIX);
            result.push(' ');
            result.push_str(line);
        }
        result.color(color).to_string()
    }

    fn mom_info(&self) -> String {
        self.mom_colorize(INFO_COLOR)
    }

    fn mom_prefix_info(&self) -> String {
        self.mom_prefix(INFO_COLOR)
    }

    fn mom_warn(&self) -> String {
        self.mom_colorize(WARN_COLOR)
    }

    fn mom_prefix_warn(&self) -> String {
        self.mom_prefix(WARN_COLOR)
    }

    fn mom_error(&self) -> String {
        self.mom_colorize(ERROR_COLOR)
    }

    fn mom_prefix_error(&self) -> String {
        self.mom_prefix(ERROR_COLOR)
    }
}

// Calling the function in a ColoredString instance removes the color from it,
// so we need to transform it to a string first to keep it.
impl MomOutput for ColoredString {
    fn mom_just_prefix(&self) -> String {
        self.to_string().mom_just_prefix()
    }

    fn mom_prefix<S: Into<Color> + Clone>(&self, color: S) -> String {
        self.to_string().mom_prefix(color)
    }

    fn mom_colorize<S: Into<Color> + Clone>(&self, color: S) -> String {
        self.to_string().mom_colorize(color)
    }

    fn mom_info(&self) -> String {
        self.to_string().mom_info()
    }

    fn mom_prefix_info(&self) -> String {
        self.to_string().mom_prefix_info()
    }

    fn mom_warn(&self) -> String {
        self.to_string().mom_warn()
    }

    fn mom_prefix_warn(&self) -> String {
        self.to_string().mom_prefix_warn()
    }

    fn mom_error(&self) -> String {
        self.to_string().mom_error()
    }

    fn mom_prefix_error(&self) -> String {
        self.to_string().mom_prefix_error()
    }
}
