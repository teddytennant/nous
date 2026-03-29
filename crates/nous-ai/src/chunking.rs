use serde::{Deserialize, Serialize};

/// Strategy for splitting text into chunks suitable for embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChunkStrategy {
    /// Fixed character count with overlap.
    FixedSize,
    /// Split on paragraph boundaries (double newline).
    Paragraph,
    /// Split on sentence boundaries.
    Sentence,
}

/// Options controlling how text is chunked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkOptions {
    /// Maximum characters per chunk.
    pub max_chars: usize,
    /// Characters of overlap between consecutive chunks (FixedSize only).
    pub overlap: usize,
    /// Strategy to use.
    pub strategy: ChunkStrategy,
}

impl Default for ChunkOptions {
    fn default() -> Self {
        Self {
            max_chars: 512,
            overlap: 64,
            strategy: ChunkStrategy::Paragraph,
        }
    }
}

/// A single chunk of text extracted from a larger document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// The chunk text.
    pub text: String,
    /// Byte offset of this chunk in the original document.
    pub offset: usize,
    /// Zero-based index of this chunk.
    pub index: usize,
}

/// Split `text` into chunks according to `options`.
///
/// Returns an empty vec for empty input. Guarantees every returned chunk
/// has at least one non-whitespace character.
pub fn chunk_text(text: &str, options: &ChunkOptions) -> Vec<Chunk> {
    if text.trim().is_empty() {
        return Vec::new();
    }

    let max = options.max_chars.max(1);

    match options.strategy {
        ChunkStrategy::FixedSize => chunk_fixed(text, max, options.overlap),
        ChunkStrategy::Paragraph => chunk_paragraph(text, max),
        ChunkStrategy::Sentence => chunk_sentence(text, max),
    }
}

fn chunk_fixed(text: &str, max_chars: usize, overlap: usize) -> Vec<Chunk> {
    let overlap = overlap.min(max_chars.saturating_sub(1));
    let step = max_chars - overlap;
    let mut chunks = Vec::new();
    let mut pos = 0;

    while pos < text.len() {
        let end = (pos + max_chars).min(text.len());
        // Snap to char boundary.
        let end = snap_char_boundary(text, end);
        let slice = &text[pos..end];
        if !slice.trim().is_empty() {
            chunks.push(Chunk {
                text: slice.to_string(),
                offset: pos,
                index: chunks.len(),
            });
        }
        let next = pos + step;
        if next <= pos || next >= text.len() {
            break;
        }
        pos = snap_char_boundary(text, next);
    }

    chunks
}

fn chunk_paragraph(text: &str, max_chars: usize) -> Vec<Chunk> {
    let paragraphs: Vec<&str> = text.split("\n\n").collect();
    merge_segments(&paragraphs, max_chars, "\n\n")
}

fn chunk_sentence(text: &str, max_chars: usize) -> Vec<Chunk> {
    let sentences = split_sentences(text);
    let refs: Vec<&str> = sentences.iter().map(|s| s.as_str()).collect();
    merge_segments(&refs, max_chars, " ")
}

/// Merge small segments until they fill a chunk, then start a new one.
fn merge_segments(segments: &[&str], max_chars: usize, joiner: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_offset = 0;
    let mut byte_pos = 0;

    for &seg in segments {
        let seg_trimmed = seg.trim();
        if seg_trimmed.is_empty() {
            byte_pos += seg.len() + joiner.len();
            continue;
        }

        let would_be = if current.is_empty() {
            seg_trimmed.len()
        } else {
            current.len() + joiner.len() + seg_trimmed.len()
        };

        if would_be > max_chars && !current.is_empty() {
            chunks.push(Chunk {
                text: current.clone(),
                offset: current_offset,
                index: chunks.len(),
            });
            current.clear();
        }

        if current.is_empty() {
            current_offset = byte_pos;
            // If a single segment exceeds max_chars, take it whole.
            current.push_str(seg_trimmed);
        } else {
            current.push_str(joiner);
            current.push_str(seg_trimmed);
        }

        byte_pos += seg.len() + joiner.len();
    }

    if !current.trim().is_empty() {
        chunks.push(Chunk {
            text: current,
            offset: current_offset,
            index: chunks.len(),
        });
    }

    chunks
}

/// Naive sentence splitter: split on `. `, `? `, `! ` while preserving the
/// punctuation on the preceding sentence.
fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        current.push(chars[i]);
        let is_terminal = matches!(chars[i], '.' | '?' | '!');
        let next_is_space = i + 1 < chars.len() && chars[i + 1] == ' ';

        if is_terminal && next_is_space {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current.clear();
            i += 2; // skip the space
            continue;
        }

        i += 1;
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    sentences
}

/// Snap a byte position to the nearest valid char boundary (backwards).
fn snap_char_boundary(text: &str, pos: usize) -> usize {
    if pos >= text.len() {
        return text.len();
    }
    let mut p = pos;
    while p > 0 && !text.is_char_boundary(p) {
        p -= 1;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_empty() {
        let opts = ChunkOptions::default();
        assert!(chunk_text("", &opts).is_empty());
        assert!(chunk_text("   ", &opts).is_empty());
        assert!(chunk_text("\n\n", &opts).is_empty());
    }

    #[test]
    fn single_short_paragraph() {
        let opts = ChunkOptions {
            max_chars: 1000,
            strategy: ChunkStrategy::Paragraph,
            ..Default::default()
        };
        let chunks = chunk_text("Hello world.", &opts);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "Hello world.");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn paragraph_splitting() {
        let text = "First paragraph.\n\nSecond paragraph.\n\nThird paragraph.";
        let opts = ChunkOptions {
            max_chars: 18,
            strategy: ChunkStrategy::Paragraph,
            ..Default::default()
        };
        let chunks = chunk_text(text, &opts);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].text, "First paragraph.");
        assert_eq!(chunks[1].text, "Second paragraph.");
        assert_eq!(chunks[2].text, "Third paragraph.");
    }

    #[test]
    fn paragraphs_merge_when_small() {
        let text = "Short.\n\nAlso short.";
        let opts = ChunkOptions {
            max_chars: 100,
            strategy: ChunkStrategy::Paragraph,
            ..Default::default()
        };
        let chunks = chunk_text(text, &opts);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Short."));
        assert!(chunks[0].text.contains("Also short."));
    }

    #[test]
    fn sentence_splitting() {
        let text = "First sentence. Second sentence. Third sentence.";
        let opts = ChunkOptions {
            max_chars: 20,
            strategy: ChunkStrategy::Sentence,
            ..Default::default()
        };
        let chunks = chunk_text(text, &opts);
        assert!(chunks.len() >= 2);
        assert!(chunks[0].text.contains("First sentence."));
    }

    #[test]
    fn sentences_merge_when_small() {
        let text = "Short. Also short.";
        let opts = ChunkOptions {
            max_chars: 100,
            strategy: ChunkStrategy::Sentence,
            ..Default::default()
        };
        let chunks = chunk_text(text, &opts);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn fixed_size_basic() {
        let text = "a".repeat(100);
        let opts = ChunkOptions {
            max_chars: 30,
            overlap: 0,
            strategy: ChunkStrategy::FixedSize,
        };
        let chunks = chunk_text(&text, &opts);
        assert!(chunks.len() >= 3);
        for chunk in &chunks {
            assert!(chunk.text.len() <= 30);
        }
    }

    #[test]
    fn fixed_size_with_overlap() {
        let text = "abcdefghijklmnopqrstuvwxyz";
        let opts = ChunkOptions {
            max_chars: 10,
            overlap: 3,
            strategy: ChunkStrategy::FixedSize,
        };
        let chunks = chunk_text(text, &opts);
        assert!(chunks.len() >= 2);
        // Overlapping: chunk 0 ends overlap chars into chunk 1's start.
        if chunks.len() >= 2 {
            let c0_end = &chunks[0].text[chunks[0].text.len() - 3..];
            let c1_start = &chunks[1].text[..3];
            assert_eq!(c0_end, c1_start);
        }
    }

    #[test]
    fn chunk_indices_sequential() {
        let text = "One.\n\nTwo.\n\nThree.\n\nFour.";
        let opts = ChunkOptions {
            max_chars: 10,
            strategy: ChunkStrategy::Paragraph,
            ..Default::default()
        };
        let chunks = chunk_text(text, &opts);
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.index, i);
        }
    }

    #[test]
    fn unicode_safe() {
        let text = "\u{1F600}".repeat(50); // 50 emoji
        let opts = ChunkOptions {
            max_chars: 20,
            overlap: 0,
            strategy: ChunkStrategy::FixedSize,
        };
        // Should not panic on char boundaries.
        let chunks = chunk_text(&text, &opts);
        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // All chunks must be valid UTF-8 (they are Strings, so this is guaranteed).
            assert!(!chunk.text.is_empty());
        }
    }

    #[test]
    fn serializes() {
        let opts = ChunkOptions::default();
        let json = serde_json::to_string(&opts).unwrap();
        let restored: ChunkOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.max_chars, 512);
    }

    #[test]
    fn question_marks_split_sentences() {
        let text = "What is Nous? It is everything. Why? Because.";
        let opts = ChunkOptions {
            max_chars: 20,
            strategy: ChunkStrategy::Sentence,
            ..Default::default()
        };
        let chunks = chunk_text(text, &opts);
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn oversized_single_segment() {
        let text = "a".repeat(200);
        let opts = ChunkOptions {
            max_chars: 50,
            strategy: ChunkStrategy::Paragraph,
            ..Default::default()
        };
        let chunks = chunk_text(&text, &opts);
        // Single paragraph exceeds limit — should still be returned.
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text.len(), 200);
    }
}
