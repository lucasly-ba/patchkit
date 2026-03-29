/// Token types and syntax node kinds for quilt series files
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_camel_case_types)]
#[repr(u16)]
pub enum SyntaxKind {
    // Tokens
    /// Hash/pound sign for comments
    HASH = 0,
    /// Space character
    SPACE,
    /// Tab character
    TAB,
    /// Newline character
    NEWLINE,
    /// Whitespace characters (spaces and tabs)
    WHITESPACE,
    /// Patch file name/path
    PATCH_NAME,
    /// Patch option (e.g., -p1, --reverse)
    OPTION,
    /// Text content (for comments)
    TEXT,
    /// Error token
    ERROR,
    /// End of file
    EOF,

    // Composite nodes
    /// Root node of the syntax tree
    ROOT,
    /// A series entry (either patch or comment)
    SERIES_ENTRY,
    /// A patch entry with name and options
    PATCH_ENTRY,
    /// A comment line
    COMMENT_LINE,
    /// Patch options section
    OPTIONS,
    /// Individual option
    OPTION_ITEM,
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(kind: SyntaxKind) -> Self {
        Self(kind as u16)
    }
}

/// Lexer for quilt series files
pub struct Lexer<'a> {
    input: &'a str,
    byte_pos: usize,  // byte position for slicing
    in_comment: bool, // track if we're inside a comment line
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input text
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            byte_pos: 0,
            in_comment: false,
        }
    }

    /// Tokenize the entire input
    pub fn tokenize(&mut self) -> Vec<(SyntaxKind, &'a str)> {
        let mut tokens = Vec::new();

        while self.byte_pos < self.input.len() {
            let token = self.next_token();
            tokens.push(token);
        }

        tokens.push((SyntaxKind::EOF, ""));
        tokens
    }

    fn next_token(&mut self) -> (SyntaxKind, &'a str) {
        let ch = self.current_char();

        match ch {
            Some('#') => {
                let start = self.byte_pos;
                self.advance();
                self.in_comment = true;
                (SyntaxKind::HASH, &self.input[start..self.byte_pos])
            }
            Some(' ') => {
                let start = self.byte_pos;
                self.advance();
                (SyntaxKind::SPACE, &self.input[start..self.byte_pos])
            }
            Some('\t') => {
                let start = self.byte_pos;
                self.advance();
                (SyntaxKind::TAB, &self.input[start..self.byte_pos])
            }
            Some('\n') => {
                let start = self.byte_pos;
                self.advance();
                self.in_comment = false;
                (SyntaxKind::NEWLINE, &self.input[start..self.byte_pos])
            }
            Some(_) => {
                if self.in_comment {
                    self.read_text()
                } else if self.at_line_start() || self.prev_is_whitespace() {
                    if self.peek_option() {
                        self.read_option()
                    } else {
                        self.read_patch_name()
                    }
                } else {
                    self.read_text()
                }
            }
            None => (SyntaxKind::ERROR, ""),
        }
    }

    fn current_char(&self) -> Option<char> {
        self.input[self.byte_pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(ch) = self.input[self.byte_pos..].chars().next() {
            self.byte_pos += ch.len_utf8();
        }
    }

    fn at_line_start(&self) -> bool {
        self.byte_pos == 0
            || (self.byte_pos > 0 && self.input.as_bytes().get(self.byte_pos - 1) == Some(&b'\n'))
    }

    fn prev_is_whitespace(&self) -> bool {
        if self.byte_pos == 0 {
            return false;
        }
        matches!(
            self.input.as_bytes().get(self.byte_pos - 1),
            Some(b' ') | Some(b'\t')
        )
    }

    fn peek_option(&self) -> bool {
        matches!(self.current_char(), Some('-'))
    }

    fn read_option(&mut self) -> (SyntaxKind, &'a str) {
        let start_byte = self.byte_pos;

        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' || ch == '\n' {
                break;
            }
            self.advance();
        }

        (SyntaxKind::OPTION, &self.input[start_byte..self.byte_pos])
    }

    fn read_patch_name(&mut self) -> (SyntaxKind, &'a str) {
        let start_byte = self.byte_pos;

        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' || ch == '\n' {
                break;
            }
            self.advance();
        }

        (
            SyntaxKind::PATCH_NAME,
            &self.input[start_byte..self.byte_pos],
        )
    }

    fn read_text(&mut self) -> (SyntaxKind, &'a str) {
        let start_byte = self.byte_pos;

        while let Some(ch) = self.current_char() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }

        (SyntaxKind::TEXT, &self.input[start_byte..self.byte_pos])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_options() {
        let text = "patch.patch -p1\n";
        let mut lexer = Lexer::new(text);
        let tokens = lexer.tokenize();

        println!("Input text: {:?}", text);
        println!("Tokens:");
        for (i, (kind, text)) in tokens.iter().enumerate() {
            println!("  {}: {:?} = {:?}", i, kind, text);
        }
    }

    #[test]
    fn test_debug_unicode() {
        let text = "# Pätch sériès with ünïcødé\npatch-ñame.patch\n# Comment with émojis 🚀\nspëcial-patch.patch -p1\n";
        let mut lexer = Lexer::new(text);
        let tokens = lexer.tokenize();

        println!("Input text: {:?}", text);
        println!("Tokens:");
        for (i, (kind, text)) in tokens.iter().enumerate() {
            println!("  {}: {:?} = {:?}", i, kind, text);
        }
    }

    #[test]
    fn test_lex_simple_patch() {
        let mut lexer = Lexer::new("patch1.patch\n");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].0, SyntaxKind::PATCH_NAME);
        assert_eq!(tokens[0].1, "patch1.patch");
        assert_eq!(tokens[1].0, SyntaxKind::NEWLINE);
        assert_eq!(tokens[2].0, SyntaxKind::EOF);
    }

    #[test]
    fn test_lex_patch_with_options() {
        let mut lexer = Lexer::new("patch1.patch -p1 --reverse\n");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].0, SyntaxKind::PATCH_NAME);
        assert_eq!(tokens[0].1, "patch1.patch");
        assert_eq!(tokens[1].0, SyntaxKind::SPACE);
        assert_eq!(tokens[2].0, SyntaxKind::OPTION);
        assert_eq!(tokens[2].1, "-p1");
        assert_eq!(tokens[3].0, SyntaxKind::SPACE);
        assert_eq!(tokens[4].0, SyntaxKind::OPTION);
        assert_eq!(tokens[4].1, "--reverse");
    }

    #[test]
    fn test_lex_comment() {
        let mut lexer = Lexer::new("# This is a comment\n");
        let tokens = lexer.tokenize();
        assert_eq!(tokens[0].0, SyntaxKind::HASH);
        assert_eq!(tokens[1].0, SyntaxKind::SPACE);
        assert_eq!(tokens[2].0, SyntaxKind::TEXT);
        assert_eq!(tokens[2].1, "This is a comment");
    }
}
