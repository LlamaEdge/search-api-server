#[macro_use]
extern crate log;

mod backend;
mod error;
mod search;
mod utils;

use anyhow::Result;
use chat_prompts::{MergeRagContextPolicy, PromptTemplateType};
use clap::Parser;
use error::ServerError;
use hyper::{
    body::HttpBody,
    header,
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server, StatusCode,
};
use llama_core::{
    error::SearchError,
    search::{ContentType, SearchConfig, SearchOutput, SearchResult},
    MetadataBuilder,
};
use once_cell::sync::OnceCell;
use search::google_parser;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};
use utils::LogLevel;

// global system prompt
pub(crate) static GLOBAL_RAG_PROMPT: OnceCell<String> = OnceCell::new();
// server info
pub(crate) static SERVER_INFO: OnceCell<ServerInfo> = OnceCell::new();

// default socket address
const DEFAULT_SOCKET_ADDRESS: &str = "0.0.0.0:8080";

#[derive(Debug, Parser)]
#[command(name = "LlamaEdge-Search API Server", version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"), about = "LlamaEdge-RAG API Server")]
struct Cli {
    /// Sets names for chat and embedding models. The names are separated by comma without space, for example, '--model-name Llama-2-7b,all-minilm'.
    #[arg(short, long, value_delimiter = ',', required = true)]
    model_name: Vec<String>,
    /// Model aliases for chat and embedding models
    #[arg(
        short = 'a',
        long,
        value_delimiter = ',',
        default_value = "default,embedding"
    )]
    model_alias: Vec<String>,
    /// Sets context sizes for chat and embedding models, respectively. The sizes are separated by comma without space, for example, '--ctx-size 4096,384'. The first value is for the chat model, and the second is for the embedding model.
    #[arg(
        short = 'c',
        long,
        value_delimiter = ',',
        default_value = "4096,384",
        value_parser = clap::value_parser!(u64)
    )]
    ctx_size: Vec<u64>,
    /// Sets prompt templates for chat and embedding models, respectively. The prompt templates are separated by comma without space, for example, '--prompt-template llama-2-chat,embedding'. The first value is for the chat model, and the second is for the embedding model.
    #[arg(short, long, value_delimiter = ',', value_parser = clap::value_parser!(PromptTemplateType), required = true)]
    prompt_template: Vec<PromptTemplateType>,
    /// Halt generation at PROMPT, return control.
    #[arg(short, long)]
    reverse_prompt: Option<String>,
    /// Sets batch sizes for chat and embedding models, respectively. The sizes are separated by comma without space, for example, '--batch-size 128,64'. The first value is for the chat model, and the second is for the embedding model.
    #[arg(short, long, value_delimiter = ',', default_value = "512,512", value_parser = clap::value_parser!(u64))]
    batch_size: Vec<u64>,
    /// Custom rag prompt.
    #[arg(long)]
    rag_prompt: Option<String>,
    /// Strategy for merging RAG context into chat messages.
    #[arg(long = "rag-policy", default_value_t, value_enum)]
    policy: MergeRagContextPolicy,
    /// URL of Qdrant REST Service
    #[arg(long, default_value = "http://127.0.0.1:6333")]
    qdrant_url: String,
    /// Name of Qdrant collection
    #[arg(long, default_value = "default")]
    qdrant_collection_name: String,
    /// Max number of retrieved result (no less than 1)
    #[arg(long, default_value = "5", value_parser = clap::value_parser!(u64))]
    qdrant_limit: u64,
    /// Minimal score threshold for the search result
    #[arg(long, default_value = "0.4", value_parser = clap::value_parser!(f32))]
    qdrant_score_threshold: f32,
    /// Maximum number of tokens each chunk contains
    #[arg(long, default_value = "100", value_parser = clap::value_parser!(usize))]
    chunk_capacity: usize,
    /// Socket address of LlamaEdge API Server instance
    #[arg(long, default_value = DEFAULT_SOCKET_ADDRESS)]
    socket_addr: String,
    /// Root path for the Web UI files
    #[arg(long, default_value = "chatbot-ui")]
    web_ui: PathBuf,
    /// Whether to enable RAG functionality.
    #[arg(long)]
    enable_rag: bool,
    /// Deprecated. Print prompt strings to stdout
    #[arg(long)]
    log_prompts: bool,
    /// Deprecated. Print statistics to stdout
    #[arg(long)]
    log_stat: bool,
    /// Deprecated. Print all log information to stdout
    #[arg(long)]
    log_all: bool,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), error::ServerError> {
    let mut plugin_debug = false;

    // get the environment variable `LLAMA_LOG`
    let log_level: LogLevel = std::env::var("LLAMA_LOG")
        .unwrap_or("info".to_string())
        .parse()
        .unwrap_or(LogLevel::Info);

    if log_level == LogLevel::Debug || log_level == LogLevel::Trace {
        plugin_debug = true;
    }

    // set global logger
    wasi_logger::Logger::install().expect("failed to install wasi_logger::Logger");
    log::set_max_level(log_level.into());

    // log the version of the server
    info!(target: "server_config", "server_version: {}", env!("CARGO_PKG_VERSION"));

    // parse the command line arguments
    let cli = Cli::parse();

    if cli.enable_rag {
        #[cfg(not(feature = "rag"))]
        return Err(ServerError::ArgumentError(
            "'--enable_rag' argument provided without the feature. Please enable the feature."
                .to_owned(),
        ));

        if cli.model_name.len() != 2 {
            return Err(ServerError::ArgumentError(
                "Enabling RAG functiionality with the LlamaEdge Search API server requires a chat model and an embedding model.".to_owned(),
            ));
        }
    } else {
        if cli.model_name.is_empty() || cli.model_name.len() > 2 {
            return Err(ServerError::ArgumentError(
                "Invalid setting for model name. For running chat or embedding model, please specify a single model name. For running both chat and embedding models, please specify two model names: the first one for chat model, the other for embedding model.".to_owned(),
            ));
        }
    }

    info!(target: "server_config", "model_name: {}", cli.model_name.join(","));

    // log model alias
    if cli.model_alias.len() != 2 {
        return Err(ServerError::ArgumentError(
            "LlamaEdge RAG API server requires two model aliases: one for chat model, one for embedding model.".to_owned(),
        ));
    }

    info!(target: "server_config", "model_alias: {}", cli.model_alias.join(","));

    // log context size
    if cli.ctx_size.len() != 2 {
        return Err(ServerError::ArgumentError(
            "LlamaEdge RAG API server requires two context sizes: one for chat model, one for embedding model.".to_owned(),
        ));
    }
    let ctx_sizes_str: String = cli
        .ctx_size
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<String>>()
        .join(",");
    info!(target: "server_config", "ctx_size: {}", ctx_sizes_str);

    // log batch size
    if cli.batch_size.len() != 2 {
        return Err(ServerError::ArgumentError(
            "LlamaEdge RAG API server requires two batch sizes: one for chat model, one for embedding model.".to_owned(),
        ));
    }
    let batch_sizes_str: String = cli
        .batch_size
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<String>>()
        .join(",");
    info!(target: "server_config", "batch_size: {}", batch_sizes_str);

    // log prompt template
    if cli.prompt_template.len() != 2 {
        return Err(ServerError::ArgumentError(
            "LlamaEdge RAG API server requires two prompt templates: one for chat model, one for embedding model.".to_owned(),
        ));
    }
    let prompt_template_str: String = cli
        .prompt_template
        .iter()
        .map(|n| n.to_string())
        .collect::<Vec<String>>()
        .join(",");
    info!(target: "server_config", "prompt_template: {}", prompt_template_str);

    // log reverse prompt
    if let Some(reverse_prompt) = &cli.reverse_prompt {
        info!(target: "server_config", "reverse_prompt: {}", reverse_prompt);
    }

    // log rag prompt
    if let Some(rag_prompt) = &cli.rag_prompt {
        info!(target: "server_config", "rag_prompt: {}", rag_prompt);

        GLOBAL_RAG_PROMPT.set(rag_prompt.clone()).map_err(|_| {
            ServerError::Operation("Failed to set `GLOBAL_RAG_PROMPT`.".to_string())
        })?;
    }

    // log qdrant url
    //if !is_valid_url(&cli.qdrant_url) {
    //    let err_msg = format!(
    //        "The URL of Qdrant REST API is invalid: {}.",
    //        &cli.qdrant_url
    //    );

    //    // log
    //    {
    //        error!(target: "server_config", "qdrant_url: {}", err_msg);
    //    }

    //    return Err(ServerError::ArgumentError(err_msg));
    //}
    //if !qdrant_up(&cli.qdrant_url).await {
    //    let err_msg = format!("[INFO] Qdrant not found at: {}", &cli.qdrant_url);
    //    error!(target: "server_config", "qdrant_url: {}", err_msg);

    //    return Err(ServerError::DatabaseError(err_msg));
    //}

    //// log qdrant url
    //info!(target: "server_config", "qdrant_url: {}", &cli.qdrant_url);

    //// log qdrant collection name
    //info!(target: "server_config", "qdrant_collection_name: {}", &cli.qdrant_collection_name);

    //// log qdrant limit
    //info!(target: "server_config", "qdrant_limit: {}", &cli.qdrant_limit);

    //// log qdrant score threshold
    //info!(target: "server_config", "qdrant_score_threshold: {}", &cli.qdrant_score_threshold);

    //// create qdrant config
    //let qdrant_config = QdrantConfig {
    //    url: cli.qdrant_url,
    //    collection_name: cli.qdrant_collection_name,
    //    limit: cli.qdrant_limit,
    //    score_threshold: cli.qdrant_score_threshold,
    //};

    //// log chunk capacity
    //info!(target: "server_config", "chunk_capacity: {}", &cli.chunk_capacity);

    //// RAG policy
    //info!(target: "server_config", "rag_policy: {}", &cli.policy);

    //let mut policy = cli.policy;
    //if policy == MergeRagContextPolicy::SystemMessage && !cli.prompt_template[0].has_system_prompt()
    //{
    //    warn!(target: "server_config", "{}", format!("The chat model does not support system message, while the '--policy' option sets to \"{}\". Update the RAG policy to {}.", cli.policy, MergeRagContextPolicy::LastUserMessage));

    //    policy = MergeRagContextPolicy::LastUserMessage;
    //}

    let google_config = SearchConfig::new(
        "google".to_owned(),
        5,
        1000,
        "http://localhost:3000/search".to_owned(),
        ContentType::JSON,
        ContentType::JSON,
        "POST".to_owned(),
        None,
        google_parser,
    );

    #[derive(Serialize)]
    struct GoogleSearchInput {
        term: String,
        engine: String,
        maxSearchResults: u8,
    }

    google_config
        .perform_search(&GoogleSearchInput {
            term: "Megami Tensei".to_owned(),
            engine: "google".to_owned(),
            maxSearchResults: 5,
        })
        .await;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ServerInfo {
    version: String,
    plugin_version: String,
    port: String,
    //#[serde(flatten)]
    //rag_config: RagConfig,
    //#[serde(flatten)]
    //qdrant_config: QdrantConfig,
}
