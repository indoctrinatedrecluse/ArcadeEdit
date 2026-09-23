//! Core LanguageService and background worker thread for asynchronous syntax parsing.

use crate::highlight::{map_capture_name, HighlightSpan};
use crate::markdown::highlight_markdown;
use arcade_core::Revision;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use streaming_iterator::StreamingIterator;

/// Supported languages for syntax parsing and highlighting in ArcadeEdit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LanguageId {
    /// Rust source code (`.rs`).
    Rust,
    /// Markdown documentation (`.md`, `.markdown`).
    Markdown,
    /// Plain text without specialized grammar parsing.
    PlainText,
}

impl LanguageId {
    /// Infers the language identifier from a file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "md" | "markdown" => Self::Markdown,
            _ => Self::PlainText,
        }
    }

    /// Infers the language identifier from a file path.
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|s| s.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::PlainText)
    }
}

/// Language service responsible for AST parsing and semantic syntax tokenization.
pub struct LanguageService {
    parser: tree_sitter::Parser,
    rust_language: tree_sitter::Language,
    rust_query: Option<tree_sitter::Query>,
}

impl LanguageService {
    /// Creates a new `LanguageService` instance with pre-compiled grammar queries.
    pub fn new() -> Result<Self, String> {
        let mut parser = tree_sitter::Parser::new();
        let rust_language = tree_sitter::Language::from(tree_sitter_rust::LANGUAGE);

        parser
            .set_language(&rust_language)
            .map_err(|e| format!("Failed to set Rust language in parser: {e:?}"))?;

        let rust_query = match tree_sitter::Query::new(&rust_language, tree_sitter_rust::HIGHLIGHTS_QUERY) {
            Ok(q) => Some(q),
            Err(e) => {
                eprintln!("Warning: failed to compile tree-sitter Rust query: {e:?}");
                None
            }
        };

        Ok(Self {
            parser,
            rust_language,
            rust_query,
        })
    }

    /// Generates syntax highlight spans for the given text and language.
    pub fn highlight(&mut self, language: LanguageId, text: &str) -> Vec<HighlightSpan> {
        match language {
            LanguageId::Rust => self.highlight_rust(text),
            LanguageId::Markdown => highlight_markdown(text),
            LanguageId::PlainText => Vec::new(),
        }
    }

    fn highlight_rust(&mut self, text: &str) -> Vec<HighlightSpan> {
        let query = match &self.rust_query {
            Some(q) => q,
            None => return Vec::new(),
        };

        if self.parser.set_language(&self.rust_language).is_err() {
            return Vec::new();
        }

        let tree = match self.parser.parse(text, None) {
            Some(t) => t,
            None => return Vec::new(),
        };

        let mut cursor = tree_sitter::QueryCursor::new();
        let mut captures = cursor.captures(query, tree.root_node(), text.as_bytes());

        let mut spans = Vec::new();
        let capture_names = query.capture_names();

        while let Some((m, idx)) = captures.next() {
            let cap = m.captures[*idx];
            let name = &capture_names[cap.index as usize];
            let kind = map_capture_name(name);
            let start = cap.node.start_byte();
            let end = cap.node.end_byte();

            if end > start {
                spans.push(HighlightSpan::new(start, end, kind));
            }
        }

        spans
    }
}

/// Request sent to the background syntax highlighting worker.
#[derive(Debug)]
pub struct HighlightRequest {
    /// Document revision identifier.
    pub revision: Revision,
    /// Target language grammar.
    pub language: LanguageId,
    /// Full buffer source code to parse.
    pub text: String,
}

/// Response returned from the background syntax worker.
#[derive(Debug)]
pub struct HighlightResponse {
    /// Document revision identifier matching the request.
    pub revision: Revision,
    /// Resulting syntax highlight spans.
    pub spans: Vec<HighlightSpan>,
}

/// Asynchronous background worker running language parsing on a separate OS thread.
pub struct HighlightWorker {
    request_tx: Sender<HighlightRequest>,
    response_rx: Receiver<HighlightResponse>,
}

impl HighlightWorker {
    /// Spawns a dedicated background thread with an initialized `LanguageService`.
    pub fn spawn() -> Result<Self, String> {
        let (request_tx, request_rx) = channel::<HighlightRequest>();
        let (response_tx, response_rx) = channel::<HighlightResponse>();

        thread::Builder::new()
            .name("arcade-highlight-worker".into())
            .spawn(move || {
                let mut service = match LanguageService::new() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("LanguageWorker failed to initialize LanguageService: {e}");
                        return;
                    }
                };

                while let Ok(req) = request_rx.recv() {
                    // Drain any newer requests in the queue to avoid redundant backlogs on rapid typing
                    let mut latest_req = req;
                    while let Ok(newer) = request_rx.try_recv() {
                        latest_req = newer;
                    }

                    let spans = service.highlight(latest_req.language, &latest_req.text);
                    let _ = response_tx.send(HighlightResponse {
                        revision: latest_req.revision,
                        spans,
                    });
                }
            })
            .map_err(|e| format!("Failed to spawn highlight worker thread: {e}"))?;

        Ok(Self {
            request_tx,
            response_rx,
        })
    }

    /// Dispatches a new highlighting request to the background worker.
    pub fn request_highlight(&self, revision: Revision, language: LanguageId, text: String) {
        let _ = self.request_tx.send(HighlightRequest { revision, language, text });
    }

    /// Attempts to poll a completed response from the background worker without blocking.
    pub fn try_recv_response(&self) -> Option<HighlightResponse> {
        match self.response_rx.try_recv() {
            Ok(res) => Some(res),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rust_source_into_highlight_spans() {
        let mut service = LanguageService::new().expect("service initialization");
        let code = "pub fn add(a: u32, b: u32) -> u32 { a + b }";
        let spans = service.highlight(LanguageId::Rust, code);

        assert!(!spans.is_empty());
        assert!(spans.iter().any(|s| s.kind == crate::highlight::HighlightKind::Keyword));
        assert!(spans.iter().any(|s| s.kind == crate::highlight::HighlightKind::Function));
        assert!(spans.iter().any(|s| s.kind == crate::highlight::HighlightKind::Type));
    }

    #[test]
    fn infers_language_from_paths_and_extensions() {
        assert_eq!(LanguageId::from_extension("rs"), LanguageId::Rust);
        assert_eq!(LanguageId::from_extension("md"), LanguageId::Markdown);
        assert_eq!(LanguageId::from_extension("txt"), LanguageId::PlainText);

        assert_eq!(LanguageId::from_path(Path::new("src/main.rs")), LanguageId::Rust);
        assert_eq!(LanguageId::from_path(Path::new("README.md")), LanguageId::Markdown);
        assert_eq!(LanguageId::from_path(Path::new("LICENSE")), LanguageId::PlainText);
    }

    #[test]
    fn worker_processes_highlight_requests_asynchronously() {
        let worker = HighlightWorker::spawn().expect("worker spawn");
        worker.request_highlight(Revision(1), LanguageId::Rust, "fn hello() {}".into());

        // Wait briefly for the worker to respond
        let start = std::time::Instant::now();
        let mut response = None;
        while start.elapsed() < std::time::Duration::from_millis(3000) {
            if let Some(res) = worker.try_recv_response() {
                response = Some(res);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert!(response.is_some());
        let res = response.unwrap();
        assert_eq!(res.revision, Revision(1));
        assert!(!res.spans.is_empty());
    }
}

