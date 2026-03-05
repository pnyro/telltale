mod linux;
mod windows;

use crate::rule::Rule;

pub fn linux_rules() -> Vec<Rule> {
    linux::rules()
}

pub fn windows_rules() -> Vec<Rule> {
    windows::rules()
}
