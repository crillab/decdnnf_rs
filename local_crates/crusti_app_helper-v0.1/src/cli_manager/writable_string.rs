// crusti_app_helper
// Copyright (C) 2020  Univ. Artois & CNRS
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::io::Write;

/// An implementation of `std::io::Write` based on a `String`.
#[derive(Default)]
pub(crate) struct WritableString(String);

impl WritableString {
    fn format(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Write for WritableString {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let to_add = String::from_utf8(buf.to_vec()).unwrap();
        self.0.push_str(&to_add);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl std::fmt::Display for WritableString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f)
    }
}

impl std::fmt::Debug for WritableString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let mut s = WritableString::default();
        write!(s, "abc").unwrap();
        write!(s, "def").unwrap();
        s.flush().unwrap();
        assert_eq!("abcdef", s.to_string())
    }

    #[test]
    fn test_debug() {
        let mut s = WritableString::default();
        write!(s, "abc").unwrap();
        write!(s, "def").unwrap();
        s.flush().unwrap();
        assert_eq!("abcdef", format!("{:?}", s))
    }
}
