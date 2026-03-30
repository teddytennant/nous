pub mod agent;
pub mod chunking;
pub mod conversation;
pub mod embedding;
pub mod executor;
pub mod knowledge;
pub mod pipeline;
pub mod prompt;
pub mod rag;
pub mod search;

pub use agent::{Agent, ToolDefinition, ToolExecutor, ToolRegistry};
pub use chunking::{Chunk, ChunkOptions, ChunkStrategy, chunk_text};
pub use conversation::{Conversation, Message, Role};
pub use embedding::{Embedding, EmbeddingIndex, SearchResult};
pub use executor::{ExecutionConfig, ExecutionResult, ExecutionStep, StepKind, run_agent};
pub use knowledge::{Document, DocumentChunk, KnowledgeBase, KnowledgeResult};
pub use pipeline::{
    CompletionRequest, CompletionResponse, EchoBackend, InferenceBackend, Pipeline,
};
pub use prompt::{PromptLibrary, PromptTemplate};
pub use rag::{RagConfig, RagContext, RagPipeline, RetrievedChunk, SearchMode};
pub use search::{Bm25Config, KeywordResult, SearchEngine};
