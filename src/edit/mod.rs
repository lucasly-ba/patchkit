use rowan::{ast::AstNode, GreenNode, SyntaxNode};
use std::fmt;
use std::marker::PhantomData;

/// Lexer for patch files
pub mod lex;
/// Lossless AST structures for patch files
pub mod lossless;
mod parse;
/// Lossless parser and editor for quilt series files
pub mod series;

pub use lossless::{
    // Common types
    AddLine,
    ContextChangeLine,

    // Context diff types
    ContextDiffFile,
    ContextHunk,
    ContextHunkHeader,
    ContextLine,
    ContextNewFile,
    ContextNewSection,
    ContextOldFile,
    ContextOldSection,
    DeleteLine,
    DiffFormat,

    EdAddCommand,
    EdChangeCommand,
    // Ed diff types
    EdCommand,
    EdContentLine,

    EdDeleteCommand,
    // Unified diff types
    FileHeader,
    Hunk,
    HunkHeader,
    HunkLine,
    HunkRange,
    Lang,
    NewFile,
    NormalChangeCommand,
    // Normal diff types
    NormalHunk,
    NormalNewLines,
    NormalOldLines,
    NormalSeparator,
    OldFile,
    Patch,
    PatchFile,
};
pub use rowan::TextRange;

/// Parse error containing a list of error messages
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParseError(pub Vec<String>);

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, err) in self.0.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", err)?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// Parse error with position information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionedParseError {
    /// The error message
    pub message: String,
    /// The position in the source text where the error occurred
    pub position: rowan::TextRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parse result containing a syntax tree and any parse errors
pub struct Parse<T> {
    green: GreenNode,
    errors: Vec<String>,
    positioned_errors: Vec<PositionedParseError>,
    _ty: PhantomData<T>,
}

impl<T> Parse<T> {
    /// Create a new parse result
    pub fn new(green: GreenNode, errors: Vec<String>) -> Self {
        Parse {
            green,
            errors,
            positioned_errors: Vec::new(),
            _ty: PhantomData,
        }
    }

    /// Create a new parse result with positioned errors
    pub fn new_with_positioned_errors(
        green: GreenNode,
        errors: Vec<String>,
        positioned_errors: Vec<PositionedParseError>,
    ) -> Self {
        Parse {
            green,
            errors,
            positioned_errors,
            _ty: PhantomData,
        }
    }

    /// Get the green node (thread-safe representation)
    pub fn green(&self) -> &GreenNode {
        &self.green
    }

    /// Get the syntax errors
    pub fn errors(&self) -> &[String] {
        &self.errors
    }

    /// Get parse errors with position information
    pub fn positioned_errors(&self) -> &[PositionedParseError] {
        &self.positioned_errors
    }

    /// Get parse errors as strings
    pub fn error_messages(&self) -> Vec<String> {
        self.positioned_errors
            .iter()
            .map(|e| e.message.clone())
            .collect()
    }

    /// Check if parsing succeeded without errors
    pub fn ok(&self) -> bool {
        self.errors.is_empty() && self.positioned_errors.is_empty()
    }

    /// Convert to a Result, returning the tree if there are no errors
    pub fn to_result(self) -> Result<T, ParseError>
    where
        T: AstNode,
    {
        if self.errors.is_empty() && self.positioned_errors.is_empty() {
            let node = SyntaxNode::<T::Language>::new_root(self.green);
            Ok(T::cast(node).expect("root node has wrong type"))
        } else {
            let mut all_errors = self.errors.clone();
            all_errors.extend(self.error_messages());
            Err(ParseError(all_errors))
        }
    }

    /// Get the parsed syntax tree, panicking if there are errors
    pub fn tree(&self) -> T
    where
        T: AstNode,
    {
        assert!(
            self.errors.is_empty() && self.positioned_errors.is_empty(),
            "tried to get tree with errors: {:?}",
            self.errors
        );
        let node = SyntaxNode::<T::Language>::new_root(self.green.clone());
        T::cast(node).expect("root node has wrong type")
    }

    /// Get the parsed syntax tree even if there are parse errors
    pub fn tree_lossy(&self) -> T
    where
        T: AstNode,
    {
        let node = SyntaxNode::<T::Language>::new_root_mut(self.green.clone());
        T::cast(node).expect("root node has wrong type")
    }

    /// Get the syntax node
    pub fn syntax_node(&self) -> SyntaxNode<T::Language>
    where
        T: AstNode,
    {
        SyntaxNode::<T::Language>::new_root(self.green.clone())
    }

    /// Cast this parse result to a different AST node type
    pub fn cast<U>(self) -> Option<Parse<U>>
    where
        T: AstNode,
        U: AstNode<Language = T::Language>,
    {
        let node = SyntaxNode::<T::Language>::new_root(self.green.clone());
        U::cast(node)?;
        Some(Parse {
            green: self.green,
            errors: self.errors,
            positioned_errors: self.positioned_errors,
            _ty: PhantomData,
        })
    }
}

/// Parse a patch file into a lossless AST
pub fn parse(text: &str) -> Parse<Patch> {
    lossless::parse(text)
}
