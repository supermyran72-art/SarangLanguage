/// A byte-offset span in source code.
///
/// Tracks the start (inclusive) and end (exclusive) byte positions
/// within a source file. Used by every compiler phase for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "span start must be <= end");
        Self { start, end }
    }

    /// A zero-width span used when no real location is available.
    pub fn dummy() -> Self {
        Self { start: 0, end: 0 }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Merge two spans into one that covers both.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

/// A named source file used for diagnostic rendering.
///
/// Holds the file name and full source text. Designed to integrate
/// with `codespan-reporting`'s `SimpleFile`.
#[derive(Debug, Clone)]
pub struct SourceFile {
    name: String,
    source: String,
}

impl SourceFile {
    pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    /// Convert to a `codespan_reporting::files::SimpleFile` for rendering.
    pub fn as_simple_file(&self) -> codespan_reporting::files::SimpleFile<&str, &str> {
        codespan_reporting::files::SimpleFile::new(&self.name, &self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_new_and_len() {
        let span = Span::new(0, 10);
        assert_eq!(span.len(), 10);
        assert!(!span.is_empty());
    }

    #[test]
    fn span_empty() {
        let span = Span::new(5, 5);
        assert!(span.is_empty());
        assert_eq!(span.len(), 0);
    }

    #[test]
    fn span_dummy() {
        let span = Span::dummy();
        assert_eq!(span.start, 0);
        assert_eq!(span.end, 0);
        assert!(span.is_empty());
    }

    #[test]
    fn span_merge() {
        let a = Span::new(2, 5);
        let b = Span::new(8, 12);
        let merged = a.merge(b);
        assert_eq!(merged.start, 2);
        assert_eq!(merged.end, 12);
    }

    #[test]
    fn span_merge_overlapping() {
        let a = Span::new(3, 10);
        let b = Span::new(5, 8);
        let merged = a.merge(b);
        assert_eq!(merged.start, 3);
        assert_eq!(merged.end, 10);
    }

    #[test]
    fn source_file_accessors() {
        let sf = SourceFile::new("test.sarang", "agent Foo {}");
        assert_eq!(sf.name(), "test.sarang");
        assert_eq!(sf.source(), "agent Foo {}");
    }

    #[test]
    fn source_file_codespan_integration() {
        let sf = SourceFile::new("test.sarang", "agent Foo {}");
        let simple = sf.as_simple_file();
        // Verify the SimpleFile exposes the correct name
        assert_eq!(*simple.name(), "test.sarang");
    }
}
