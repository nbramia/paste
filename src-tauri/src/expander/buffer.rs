/// Rolling character buffer for tracking recently typed characters.
/// Used to match typed text against snippet abbreviations.
pub struct CharBuffer {
    chars: Vec<char>,
    max_size: usize,
}

impl CharBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            chars: Vec::with_capacity(max_size),
            max_size,
        }
    }

    /// Add a character to the buffer.
    pub fn push(&mut self, ch: char) {
        if self.chars.len() >= self.max_size {
            self.chars.remove(0);
        }
        self.chars.push(ch);
    }

    /// Remove the last character (for backspace handling).
    pub fn pop(&mut self) -> Option<char> {
        self.chars.pop()
    }

    /// Get the current buffer content as a string.
    pub fn as_str(&self) -> String {
        self.chars.iter().collect()
    }

    /// Check if the buffer ends with the given string.
    pub fn ends_with(&self, suffix: &str) -> bool {
        let suffix_chars: Vec<char> = suffix.chars().collect();
        if suffix_chars.len() > self.chars.len() {
            return false;
        }
        let start = self.chars.len() - suffix_chars.len();
        self.chars[start..] == suffix_chars[..]
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.chars.clear();
    }

    /// Get the number of characters in the buffer.
    pub fn len(&self) -> usize {
        self.chars.len()
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_buffer_is_empty() {
        let buf = CharBuffer::new(100);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.as_str(), "");
    }

    #[test]
    fn test_push_and_read() {
        let mut buf = CharBuffer::new(100);
        buf.push('h');
        buf.push('e');
        buf.push('l');
        buf.push('l');
        buf.push('o');
        assert_eq!(buf.as_str(), "hello");
        assert_eq!(buf.len(), 5);
    }

    #[test]
    fn test_max_size_eviction() {
        let mut buf = CharBuffer::new(5);
        for ch in "abcdefgh".chars() {
            buf.push(ch);
        }
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.as_str(), "defgh");
    }

    #[test]
    fn test_pop() {
        let mut buf = CharBuffer::new(100);
        buf.push('a');
        buf.push('b');
        buf.push('c');
        assert_eq!(buf.pop(), Some('c'));
        assert_eq!(buf.as_str(), "ab");
        assert_eq!(buf.pop(), Some('b'));
        assert_eq!(buf.pop(), Some('a'));
        assert_eq!(buf.pop(), None);
    }

    #[test]
    fn test_ends_with() {
        let mut buf = CharBuffer::new(100);
        for ch in ";sig".chars() {
            buf.push(ch);
        }
        assert!(buf.ends_with(";sig"));
        assert!(buf.ends_with("sig"));
        assert!(buf.ends_with("ig"));
        assert!(buf.ends_with("g"));
        assert!(!buf.ends_with(";signature"));
        assert!(!buf.ends_with("xyz"));
    }

    #[test]
    fn test_ends_with_longer_than_buffer() {
        let mut buf = CharBuffer::new(100);
        buf.push('a');
        assert!(!buf.ends_with("abc"));
    }

    #[test]
    fn test_clear() {
        let mut buf = CharBuffer::new(100);
        buf.push('a');
        buf.push('b');
        buf.clear();
        assert!(buf.is_empty());
        assert_eq!(buf.as_str(), "");
    }
}
