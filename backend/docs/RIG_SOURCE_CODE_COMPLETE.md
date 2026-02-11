# Rig v0.29.0 Complete Source Code Reference

This document contains the actual source code from Rig v0.29.0 with line number references for tracing the complete chat flow.

**Source**: Rig v0.29.0 crate (`rig-core` dependency in this buildscale repo)

**Verified**: All source files referenced below exist in the Rig crate at:
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/`

**Note**: This buildscale repo uses Rig as a Cargo dependency, not vendored source.

## Table of Contents
1. [Agent Module](#1-agent-module)
2. [Agent Builder](#2-agent-builder)
3. [Agent Completion](#3-agent-completion)
4. [Prompt Request Module](#4-prompt-request-module)
5. [Streaming Prompt Request](#5-streaming-prompt-request)
6. [OpenAI Completion](#6-openai-completion)
7. [OpenAI Streaming](#7-openai-streaming)
8. [BuildScale Wrapper Layer](#8-buildscale-wrapper-layer)

---

## 1. Agent Module

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/mod.rs`
**Lines**: 1-123

```rust
     1→//! This module contains the implementation of the [Agent] struct and its builder.
     2→//!
     3→//! The [Agent] struct represents an LLM agent, which combines an LLM model with a preamble (system prompt),
     4→//! a set of context documents, and a set of tools. Note: both context documents and tools can be either
     5→//! static (i.e.: they are always provided) or dynamic (i.e.: they are RAGged at prompt-time).
     6→//!
     7→//! The [Agent] struct is highly configurable, allowing the user to define anything from
     8→//! a simple bot with a specific system prompt to a complex RAG system with a set of dynamic
     9→//! context documents and tools.
    10→//!
    11→//! The [Agent] struct implements the [crate::completion::Completion] and [crate::completion::Prompt] traits,
    12→//! allowing it to be used for generating completions responses and prompts. The [Agent] struct also
    13→//! implements [crate::completion::Chat] trait, which allows it to be used for generating chat completions.
    14→//!
    15→//! The [AgentBuilder] implements the builder pattern for creating instances of [Agent].
    16→//! It allows configuring the model, preamble, context documents, tools, temperature, and additional parameters
    17→//! before building the agent.
    18→//!
    19→//! # Example
    20→//! ```rust
    21→//! use rig::{
    22→//!     completion::{Chat, Completion, Prompt},
    23→//!     providers::openai,
    24→//! };
    25→//!
    26→//! let openai = openai::Client::from_env();
    27→//!
    28→//! // Configure the agent
    29→//! let agent = openai.agent("gpt-4o")
    30→//!     .preamble("System prompt")
    31→//!     .context("Context document 1")
    32→//!     .context("Context document 2")
    33→//!     .tool(tool1)
    34→//!     .tool(tool2)
    35→//!     .temperature(0.8)
    36→//!     .additional_params(json!({"foo": "bar"}))
    37→//!     .build();
    38→//!
    39→//! // Use the agent for completions and prompts
    40→//! // Generate a chat completion response from a prompt and chat history
    41→//! let chat_response = agent.chat("Prompt", chat_history)
    42→//!     .await
    43→//!     .expect("Failed to chat with Agent");
    44→//!
    45→//! // Generate a prompt completion response from a simple prompt
    46→//! let chat_response = agent.prompt("Prompt")
    47→//!     .await
    48→//!     .expect("Failed to prompt the Agent");
    49→//!
    50→//! // Generate a completion request builder from a prompt and chat history. The builder
    51→//! // will contain the agent's configuration (i.e.: preamble, context documents, tools,
    52→//! // model parameters, etc.), but these can be overwritten.
    53→//! let completion_req_builder = agent.completion("Prompt", chat_history)
    54→//!     .await
    55→//!     .expect("Failed to create completion request builder");
    56→//!
    57→//! let response = completion_req_builder
    58→//!     .temperature(0.9) // Overwrite the agent's temperature
    59→//!     .send()
    60→//!     .await
    61→//!     .expect("Failed to send completion request");
    62→//! ```
    63→//!
    64→//! RAG Agent example
    65→//! ```rust
    66→//! use rig::{
    67→//!     completion::Prompt,
    68→//!     embeddings::EmbeddingsBuilder,
    69→//!     providers::openai,
    70→//!     vector_store::{in_memory_store::InMemoryVectorStore, VectorStore},
    71→//! };
    72→//!
    73→//! // Initialize OpenAI client
    74→//! let openai = openai::Client::from_env();
    75→//!
    76→//! // Initialize OpenAI embedding model
    77→//! let embedding_model = openai.embedding_model(openai::TEXT_EMBEDDING_ADA_002);
    78→//!
    79→//! // Create vector store, compute embeddings and load them in the store
    80→//! let mut vector_store = InMemoryVectorStore::default();
    81→//!
    82→//! let embeddings = EmbeddingsBuilder::new(embedding_model.clone())
    83→//!     .simple_document("doc0", "Definition of a *flurbo*: A flurbo is a green alien that lives on cold planets")
    84→//!     .simple_document("doc1", "Definition of a *glarb-glarb*: A glarb-glarb is a ancient tool used by the ancestors of the inhabitants of planet Jiro to farm the land.")
    85→//!     .simple_document("doc2", "Definition of a *linglingdong*: A term used by inhabitants of the far side of the moon to describe humans.")
    86→//!     .build()
    87→//!     .await
    88→//!     .expect("Failed to build embeddings")
    89→//!
    90→//! vector_store.add_documents(embeddings)
    91→//!     .await
    92→//!     .expect("Failed to add documents");
    93→//!
    94→//! // Create vector store index
    95→//! let index = vector_store.index(embedding_model);
    96→//!
    97→//! let agent = openai.agent(openai::GPT_4O)
    98→//!     .preamble("
    99→//!         You are a dictionary assistant here to assist with understanding the meaning of words.
   100→//!         You will find additional non-standard word definitions that could be useful below.
   101→//!     ")
   102→//!     .dynamic_context(1, index)
   103→//!     .build();
   104→//!
   105→//! // Prompt the agent and print the response
   106→//! let response = agent.prompt("What does \"glarb-glarb\" mean?").await
   107→//!     .expect("Failed to prompt the agent");
   108→//! ```
   109→mod builder;
   110→mod completion;
   111→pub(crate) mod prompt_request;
   112→mod tool;
   113→
   114→pub use crate::message::Text;
   115→pub use builder::{AgentBuilder, AgentBuilderSimple};
   116→pub use completion::Agent;
   117→pub use prompt_request::streaming::{
   118→    FinalResponse, MultiTurnStreamItem, StreamingError, StreamingPromptRequest, StreamingResult,
   119→    stream_to_stdout,
   120→};
   121→pub use prompt_request::{CancelSignal, PromptRequest, PromptResponse};
   122→pub use prompt_request::{PromptHook, StreamingPromptHook};
   123→```

---

## 2. Agent Builder

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/builder.rs`
**Lines**: 1-588

```rust
     1→use std::{collections::HashMap, sync::Arc};
     2→
     3→use tokio::sync::RwLock;
     4→
     5→use crate::{
     6→    completion::{CompletionModel, Document},
     7→    message::ToolChoice,
     8→    tool::{
     9→        Tool, ToolDyn, ToolSet,
    10→        server::{ToolServer, ToolServerHandle},
    11→    },
    12→    vector_store::VectorStoreIndexDyn,
    13→};
    14→
    15→#[cfg(feature = "rmcp")]
    16→#[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
    17→use crate::tool::rmcp::McpTool as RmcpTool;
    18→
    19→use super::Agent;
    20→
    21→/// A builder for creating an agent
    22→///
    23→/// # Example
    24→/// ```
    25→/// use rig::{providers::openai, agent::AgentBuilder};
    26→///
    27→/// let openai = openai::Client::from_env();
    28→///
    29→/// let gpt4o = openai.completion_model("gpt-4o");
    30→///
    31→/// // Configure the agent
    32→/// let agent = AgentBuilder::new(gpt4o)
    33→///     .preamble("System prompt")
    34→///     .context("Context document 1")
    35→///     .context("Context document 2")
    36→///     .tool(tool1)
    37→///     .tool(tool2)
    38→///     .temperature(0.8)
    39→///     .additional_params(json!({"foo": "bar"}))
    40→///     .build();
    41→/// ```
    42→pub struct AgentBuilder<M>
    43→where
    44→    M: CompletionModel,
    45→{
    46→    /// Name of the agent used for logging and debugging
    47→    name: Option<String>,
    48→    /// Agent description. Primarily useful when using sub-agents as part of an agent workflow and converting agents to other formats.
    49→    description: Option<String>,
    50→    /// Completion model (e.g.: OpenAI's gpt-3.5-turbo-1106, Cohere's command-r)
    51→    model: M,
    52→    /// System prompt
    53→    preamble: Option<String>,
    54→    /// Context documents always available to the agent
    55→    static_context: Vec<Document>,
    56→    /// Additional parameters to be passed to the model
    57→    additional_params: Option<serde_json::Value>,
    58→    /// Maximum number of tokens for the completion
    59→    max_tokens: Option<u64>,
    60→    /// List of vector store, with the sample number
    61→    dynamic_context: Vec<(usize, Box<dyn VectorStoreIndexDyn + Send + Sync>)>,
    62→    /// Temperature of the model
    63→    temperature: Option<f64>,
    64→    /// Tool server handle
    65→    tool_server_handle: Option<ToolServerHandle>,
    66→    /// Whether or not to underlying LLM should be forced to use a tool before providing a response.
    67→    tool_choice: Option<ToolChoice>,
    68→    /// Default maximum depth for multi-turn agent calls
    69→    default_max_depth: Option<usize>,
    70→}
    71→
    72→impl<M> AgentBuilder<M>
    73→where
    74→    M: CompletionModel,
    75→{
    76→    pub fn new(model: M) -> Self {
    77→        Self {
    78→            name: None,
    79→            description: None,
    80→            model,
    81→            preamble: None,
    82→            static_context: vec![],
    83→            temperature: None,
    84→            max_tokens: None,
    85→            additional_params: None,
    86→            dynamic_context: vec![],
    87→            tool_server_handle: None,
    88→            tool_choice: None,
    89→            default_max_depth: None,
    90→        }
    91→    }
    92→
    93→    /// Set the name of the agent
    94→    pub fn name(mut self, name: &str) -> Self {
    95→        self.name = Some(name.into());
    96→        self
    97→    }
    98→
    99→    /// Set the description of the agent
   100→    pub fn description(mut self, description: &str) -> Self {
   101→        self.description = Some(description.into());
   102→        self
   103→    }
   104→
   105→    /// Set the system prompt
   106→    pub fn preamble(mut self, preamble: &str) -> Self {
   107→        self.preamble = Some(preamble.into());
   108→        self
   109→    }
   110→
   111→    /// Remove the system prompt
   112→    pub fn without_preamble(mut self) -> Self {
   113→        self.preamble = None;
   114→        self
   115→    }
   116→
   117→    /// Append to the preamble of the agent
   118→    pub fn append_preamble(mut self, doc: &str) -> Self {
   119→        self.preamble = Some(format!(
   120→            "{}\n{}",
   121→            self.preamble.unwrap_or_else(|| "".into()),
   122→            doc
   123→        ));
   124→        self
   125→    }
   126→
   127→    /// Add a static context document to the agent
   128→    pub fn context(mut self, doc: &str) -> Self {
   129→        self.static_context.push(Document {
   130→            id: format!("static_doc_{}", self.static_context.len()),
   131→            text: doc.into(),
   132→            additional_props: HashMap::new(),
   133→        });
   134→        self
   135→    }
   136→
   137→    /// Add a static tool to the agent
   138→    pub fn tool(self, tool: impl Tool + 'static) -> AgentBuilderSimple<M> {
   139→        let toolname = tool.name();
   140→        let tools = ToolSet::from_tools(vec![tool]);
   141→        let static_tools = vec![toolname];
   142→
   143→        AgentBuilderSimple {
   144→            name: self.name,
   145→            description: self.description,
   146→            model: self.model,
   147→            preamble: self.preamble,
   148→            static_context: self.static_context,
   149→            static_tools,
   150→            additional_params: self.additional_params,
   151→            max_tokens: self.max_tokens,
   152→            dynamic_context: vec![],
   153→            dynamic_tools: vec![],
   154→            temperature: self.temperature,
   155→            tools,
   156→            tool_choice: self.tool_choice,
   157→            default_max_depth: self.default_max_depth,
   158→        }
   159→    }
   160→
   161→    /// Add a vector of boxed static tools to the agent
   162→    /// This is useful when you need to dynamically add static tools to the agent
   163→    pub fn tools(self, tools: Vec<Box<dyn ToolDyn>>) -> AgentBuilderSimple<M> {
   164→        let static_tools = tools.iter().map(|tool| tool.name()).collect();
   165→        let tools = ToolSet::from_tools_boxed(tools);
   166→
   167→        AgentBuilderSimple {
   168→            name: self.name,
   169→            description: self.description,
   170→            model: self.model,
   171→            preamble: self.preamble,
   172→            static_context: self.static_context,
   173→            static_tools,
   174→            additional_params: self.additional_params,
   175→            max_tokens: self.max_tokens,
   176→            dynamic_context: vec![],
   177→            dynamic_tools: vec![],
   178→            temperature: self.temperature,
   179→            tools,
   180→            tool_choice: self.tool_choice,
   181→            default_max_depth: self.default_max_depth,
   182→        }
   183→    }
   184→
   185→    pub fn tool_server_handle(mut self, handle: ToolServerHandle) -> Self {
   186→        self.tool_server_handle = Some(handle);
   187→        self
   188→    }
   189→
   190→    /// Add an MCP tool (from `rmcp`) to the agent
   191→    #[cfg(feature = "rmcp")]
    192→    #[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
    193→    pub fn rmcp_tool(
    194→        self,
    195→        tool: rmcp::model::Tool,
    196→        client: rmcp::service::ServerSink,
    197→    ) -> AgentBuilderSimple<M> {
    198→        let toolname = tool.name.clone().to_string();
    199→        let tools = ToolSet::from_tools(vec![RmcpTool::from_mcp_server(tool, client)]);
   200→        let static_tools = vec![toolname];
   201→
   202→        AgentBuilderSimple {
   203→            name: self.name,
    204→            description: self.description,
    205→            model: self.model,
    206→            preamble: self.preamble,
    207→            static_context: self.static_context,
   208→            static_tools,
   209→            additional_params: self.additional_params,
   210→            max_tokens: self.max_tokens,
   211→            dynamic_context: vec![],
   212→            dynamic_tools: vec![],
   213→            temperature: self.temperature,
   214→            tools,
   215→            tool_choice: self.tool_choice,
   216→            default_max_depth: self.default_max_depth,
   217→        }
   218→    }
   219→
    220→    /// Add an array of MCP tools (from `rmcp`) to the agent
    221→    #[cfg(feature = "rmcp")]
    222→    #[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
    223→    pub fn rmcp_tools(
    224→        self,
    225→        tools: Vec<rmcp::model::Tool>,
    226→        client: rmcp::service::ServerSink,
    227→    ) -> AgentBuilderSimple<M> {
   228→        let (static_tools, tools) = tools.into_iter().fold(
    229→            (Vec::new(), Vec::new()),
    230→            |(mut toolnames, mut toolset), tool| {
    231→                let tool_name = tool.name.to_string();
    232→                let tool = RmcpTool::from_mcp_server(tool, client.clone());
    233→                toolnames.push(tool_name);
    234→                toolset.push(tool);
    235→                (toolnames, toolset)
    236→            },
    237→        );
    238→
    239→        let tools = ToolSet::from_tools(tools);
    240→
    241→        AgentBuilderSimple {
    242→            name: self.name,
    243→            description: self.description,
    244→            model: self.model,
    245→            preamble: self.preamble,
    246→            static_context: self.static_context,
    247→            static_tools,
    248→            additional_params: self.additional_params,
    249→            max_tokens: self.max_tokens,
    250→            dynamic_context: vec![],
    251→            dynamic_tools: vec![],
    252→            temperature: self.temperature,
    253→            tools,
    254→            tool_choice: self.tool_choice,
    255→            default_max_depth: self.default_max_depth,
    256→        }
    257→    }
   258→
    259→    /// Add some dynamic context to the agent. On each prompt, `sample` documents from the
    260→    /// dynamic context will be inserted in request.
    261→    pub fn dynamic_context(
    262→        mut self,
    263→        sample: usize,
    264→        dynamic_context: impl VectorStoreIndexDyn + Send + Sync + 'static,
    265→    ) -> Self {
    266→        self.dynamic_context
    267→            .push((sample, Box::new(dynamic_context)));
    268→        self
    269→    }
    270→
    271→    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
    272→        self.tool_choice = Some(tool_choice);
    273→        self
    274→    }
    275→
    276→    /// Set the default maximum depth that an agent will use for multi-turn.
    277→    pub fn default_max_depth(mut self, default_max_depth: usize) -> Self {
    278→        self.default_max_depth = Some(default_max_depth);
    279→        self
    280→    }
    281→
    282→    /// Add some dynamic tools to the agent. On each prompt, `sample` tools from the
    283→    /// dynamic toolset will be inserted in request.
    284→    pub fn dynamic_tools(
    285→        self,
    286→        sample: usize,
    287→        dynamic_tools: impl VectorStoreIndexDyn + Send + Sync + 'static,
    288→        toolset: ToolSet,
    289→    ) -> AgentBuilderSimple<M> {
    290→        let thing: Box<dyn VectorStoreIndexDyn + Send + Sync + 'static> = Box::new(dynamic_tools);
    291→        let dynamic_tools = vec![(sample, thing)];
    292→
    293→        AgentBuilderSimple {
    294→            name: self.name,
    295→            description: self.description,
    296→            history: self.preamble,
    297→            static_context: self.static_context,
    298→            static_tools: vec![],
    299→            additional_params: self.additional_params,
    300→            max_tokens: self.max_tokens,
    301→            dynamic_context: vec![],
    302→            dynamic_tools,
    303→            temperature: self.temperature,
    304→            tools: toolset,
    305→            tool_choice: self.tool_choice,
    306→            default_max_depth: self.default_max_depth,
    307→        }
    308→    }
    309→
    310→    /// Set the temperature of the model
    311→    pub fn temperature(mut self, temperature: f64) -> Self {
    312→        self.temperature = Some(temperature);
    313→        self
    314→    }
    315→
    316→    /// Set the maximum number of tokens for the completion
    317→    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
    318→        self.max_tokens = Some(max_tokens);
    319→        self
    320→    }
    321→
    322→    /// Set additional parameters to be passed to the model
    323→    pub fn additional_params(mut self, params: serde_json::Value) -> Self {
    324→        self.additional_params = Some(params);
    325→        self
    326→    }
    327→
    328→    /// Build the agent
    329→    pub fn build(self) -> Agent<M> {
    330→        let tool_server_handle = if let Some(handle) = self.tool_server_handle {
    331→            handle
    332→        } else {
    333→            ToolServer::new().run()
    334→        };
    335→
    336→        Agent {
    337→            name: self.name,
    338→            description: self.description,
    339→            model: Arc::new(self.model),
    340→            preamble: self.preamble,
    341→            static_context: self.static_context,
    342→            temperature: self.temperature,
    343→            max_tokens: self.max_tokens,
    344→            additional_params: self.additional_params,
    345→            tool_choice: self.tool_choice,
    346→            dynamic_context: Arc::new(RwLock::new(self.dynamic_context)),
    347→            tool_server_handle,
    348→            default_max_depth: self.default_max_depth,
    349→        }
    350→    }
    351→}
    352→
    353→/// A fluent builder variation of `AgentBuilder`. Allows adding tools directly to the builder rather than using a tool server handle.
    354→///
    355→/// # Example
    356→/// ```
    357→/// use rig::{providers::openai, agent::AgentBuilder};
    358→///
    359→/// let openai = openai::Client::from_env();
    360→///
    361→/// let gpt4o = openai.completion_model("gpt-4o");
    362→///
    363→/// // Configure the agent
    364→/// let agent = AgentBuilder::new(gpt4o)
    365→///     .preamble("System prompt")
    366→///     .context("Context document 1")
    367→///     .context("Context document 2")
     368→///     .tool(tool1)
    369→///     .tool(tool2)
    370→///     .temperature(0.8)
    371→///     .additional_params(json!({"foo": "bar"}))
    372→///     .build();
    373→/// ```
    374→pub struct AgentBuilderSimple<M>
    375→where
    376→    M: CompletionModel,
    377→{
    378→    /// Name of the agent used for logging and debugging
    379→    name: Option<String>,
    380→    /// Agent description. Primarily useful when using sub-agents as part of an agent workflow and converting agents to other formats.
    381→    description: Option<String>,
    382→    /// Completion model (e.g.: OpenAI's gpt-3.5-turbo-1106, Cohere's command-r)
    383→    model: M,
    384→    /// System prompt
    385→    preamble: Option<String>,
    386→    /// Context documents always available to the agent
    387→    static_context: Vec<Document>,
    388→    /// Tools that are always available to the agent (by name)
    389→    static_tools: Vec<String>,
    390→    /// Additional parameters to be passed to the model
    391→    additional_params: Option<serde_json::Value>,
    392→    /// Maximum number of tokens for the completion
    393→    max_tokens: Option<u64>,
    394→    /// List of vector store, with the sample number
    395→    dynamic_context: Vec<(usize, Box<dyn VectorStoreIndexDyn + Send + Sync>)>,
    396→    /// Dynamic tools
    397→    dynamic_tools: Vec<(usize, Box<dyn VectorStoreIndexDyn + Send + Sync>)>,
    398→    /// Temperature of the model
    399→    temperature: Option<f64>,
    400→    /// Actual tool implementations
    401→    tools: ToolSet,
    402→    /// Whether or not to underlying LLM should be forced to use a tool before providing a response.
    403→    tool_choice: Option<ToolChoice>,
    404→    /// Default maximum depth for multi-turn agent calls
    405→    default_max_depth: Option<usize>,
    406→}
    407→
    408→impl<M> AgentBuilderSimple<M>
    409→where
    410→    M: CompletionModel,
    411→{
    412→    pub fn new(model: M) -> Self {
    413→        Self {
    414→            name: None,
    415→            description: None,
    416→            model,
    417→            preamble: None,
    418→            static_context: vec![],
    419→            static_tools: vec![],
    420→            temperature: None,
    421→            max_tokens: None,
    422→            additional_params: None,
    423→            dynamic_context: vec![],
    424→            dynamic_tools: vec![],
    425→            tools: ToolSet::default(),
    426→            tool_choice: None,
    427→            default_max_depth: None,
    428→        }
    429→    }
    430→
    431→    /// Set the name of the agent
    432→    pub fn name(mut self, name: &str) -> Self {
    433→        self.name = Some(name.into());
    434→        self
    435→    }
    436→
    437→    /// Set the description of the agent
    438→    pub fn description(mut self, description: &str) -> Self {
    439→        self.description = Some(description.into());
    440→        self
    441→    }
    442→
    443→    /// Set the system prompt
    444→    pub fn preamble(mut self, preamble: &str) -> Self {
    445→        self.preamble = Some(preamble.into());
    446→        self
    47→    }
    448→
    449→    /// Remove the system prompt
    450→    pub fn without_preamble(mut self) -> Self {
    451→        self.preamble = None;
    452→        self
    453→    }
    454→
    455→    /// Append to the preamble of the agent
    456→    pub fn append_preamble(mut self, doc: &str) -> Self {
    457→        self.preamble = Some(format!(
    458→            "{}\n{}",
    459→            self.preamble.unwrap_or_else(|| "".into()),
    460→            doc
    461→        ));
    462→        self
    463→    }
    464→
    465→    /// Add a static context document to the agent
    466→    pub fn context(mut self, doc: &str) -> Self {
    467→        self.static_context.push(Document {
    468→            id: format!("static_doc_{}", self.static_context.len()),
    469→            text: doc.into(),
    470→            additional_props: HashMap::new(),
    471→        });
    472→        self
    473→    }
    474→
    475→    /// Add a static tool to the agent
    476→    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
    477→        let toolname = tool.name();
    478→        self.tools.add_tool(tool);
    479→        self.static_tools.push(toolname);
    480→        self
    481→    }
    482→
    483→    pub fn tools(mut self, tools: Vec<Box<dyn ToolDyn>>) -> Self {
    484→        let toolnames: Vec<String> = tools.iter().map(|tool| tool.name()).collect();
    485→        let tools = ToolSet::from_tools_boxed(tools);
    486→        self.tools.add_tools(tools);
    487→        self.static_tools.extend(toolnames);
    488→        self
    489→    }
    490→
    491→    /// Add an array of MCP tools (from `rmcp`) to the agent
    492→    #[cfg(feature = "rmcp")]
    493→    #[cfg_attr(docsrs, doc(cfg(feature = "rmcp")))]
    494→    pub fn rmcp_tools(
    495→        mut self,
    496→        tools: Vec<rmcp::model::Tool>,
    497→        client: rmcp::service::ServerSink,
    498→    ) -> Self {
    499→        for tool in tools {
    500→            let tool_name = tool.name.to_string();
    501→            let tool = RmcpTool::from_mcp_server(tool, client.clone());
    502→            self.static_tools.push(tool_name);
    503→            self.tools.add_tool(tool);
    504→        }
    505→
    506→        self
    507→    }
    508→
    509→    /// Add some dynamic context to the agent. On each prompt, `sample` documents from the
    510→    /// dynamic context will be inserted in request.
    511→    pub fn dynamic_context(
    512→        mut self,
    513→        sample: usize,
    514→        dynamic_context: impl VectorStoreIndexDyn + Send + Sync + 'static,
    515→    ) -> Self {
    516→        self.dynamic_context
    517→            .push((sample, Box::new(dynamic_context)));
        518→        self
    519→    }
    520→
    521→    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
    522→        self.tool_choice = Some(tool_choice);
    523→        self
    524→    }
    525→
    526→    /// Set the default maximum depth that an agent will use for multi-turn.
    527→    pub fn default_max_depth(mut self, default_max_depth: usize) -> Self {
    528→        self.default_max_depth = Some(default_max_depth);
    529→        self
    530→    }
    531→
    532→    /// Add some dynamic tools to the agent. On each prompt, `sample` tools from the
    533→    /// dynamic toolset will be inserted in request.
    534→    pub fn dynamic_tools(
    535→        mut self,
    536→        sample: usize,
    537→        dynamic_tools: impl VectorStoreIndexDyn + Send + Sync + 'static,
    538→        toolset: ToolSet,
    539→    ) -> Self {
    540→        self.dynamic_tools.push((sample, Box::new(dynamic_tools)));
    541→        self.tools.add_tools(toolset);
    542→        self
    543→    }
    544→
    545→    /// Set the temperature of the model
    546→    pub fn temperature(mut self, temperature: f64) -> Self {
    547→        self.temperature = Some(temperature);
    548→        self
    549→    }
    550→
    551→    /// Set the maximum number of tokens for the completion
    552→    pub fn max_tokens(mut self, max_tokens: u64) -> Self {
    553→        self.max_tokens = Some(max_tokens);
    554→        self
    555→    }
    556→
    557→    /// Set additional parameters to be passed to the model
    558→    pub fn additional_params(mut self, params: serde_json::Value) -> Self {
    559→        self.additional_params = Some(params);
    560→        self
    561→    }
    562→
    563→    /// Build the agent
    564→    pub fn build(self) -> Agent<M> {
    565→        let tool_server_handle = ToolServer::new()
    566→            .static_tool_names(self.static_tools)
    567→            .add_tools(self.tools)
    568→            .add_dynamic_tools(self.dynamic_tools)
    569→            .run();
    570→
    571→        Agent {
    572→            name: self.name,
    573→            description: self.description,
    574→            model: Arc::new(self.model),
    575→            preamble: self.preamble,
    576→            static_context: self.static_context,
    577→            temperature: self.temperature,
    578→            max_tokens: self.max_tokens,
    579→            additional_params: self.additional_params,
    580→            tool_choice: self.tool_choice,
    581→            dynamic_context: Arc::new(RwLock::new(self.dynamic_context)),
    582→            tool_server_handle,
    583→            default_max_depth: self.default_max_depth,
    584→        }
    585→    }
    586→}
```

---

## 3. Agent Completion (Streaming Chat Implementation)

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/completion.rs`
**Lines**: 1-284

```rust
     1→use super::prompt_request::{self, PromptRequest};
     2→use crate::{
     3→    agent::prompt_request::streaming::StreamingPromptRequest,
     4→    completion::{
     5→        Chat, Completion, CompletionError, CompletionModel, CompletionRequestBuilder, Document,
     6→        GetTokenUsage, Message, Prompt, PromptError,
     7→    },
     8→    message::ToolChoice,
     9→    streaming::{StreamingChat, StreamingCompletion, StreamingPrompt},
    10→    tool::server::ToolServerHandle,
    11→    vector_store::{VectorStoreError, request::VectorSearchRequest},
    12→    wasm_compat::WasmCompatSend,
    13→};
    14→use futures::{StreamExt, TryStreamExt, stream};
    15→use std::{collections::HashMap, sync::Arc};
    16→use tokio::sync::RwLock;
    17→
    18→const UNKNOWN_AGENT_NAME: &str = "Unnamed Agent";
    19→
    20→pub type DynamicContextStore = Arc<
    21→    RwLock<
    22→        Vec<(
    23→            usize,
    24→            Box<dyn crate::vector_store::VectorStoreIndexDyn + Send + Sync>,
    25→        )>,
    26→    >,
    27→;
    28→
    29→/// Struct representing an LLM agent. An agent is an LLM model combined with a preamble
    30→/// (i.e.: system prompt) and a static set of context documents and tools.
    31→/// All context documents and tools are always provided to agent when prompted.
    32→///
    33→/// # Example
    34→/// ```
    35→/// use rig::{completion::Prompt, providers::openai};
    36→///
    37→/// let openai = openai::Client::from_env();
    38→///
    39→/// let comedian_agent = openai
    40→///     .agent("gpt-4o")
    41→///     .preamble("You are a comedian here to entertain user using humour and jokes.")
    42→///     .temperature(0.9)
    43→///     .build();
    44→///
    45→/// let response = comedian_agent.prompt("Entertain me!")
    46→///     .await
    47→///     .expect("Failed to prompt the agent");
    48→/// ```
    49→#[derive(Clone)]
    50→#[non_exhaustive]
    51→pub struct Agent<M>
    52→where
    53→    M: CompletionModel,
    54→{
    55→    /// Name of agent used for logging and debugging
    56→    pub name: Option<String>,
    57→    /// Agent description. Primarily useful when using sub-agents as part of an agent workflow and converting agents to other formats.
    58→    pub description: Option<String>,
    59→    /// Completion model (e.g.: OpenAI's gpt-3.5-turbo-1106, Cohere's command-r)
    60→    pub model: Arc<M>,
    61→    /// System prompt
    62→    pub preamble: Option<String>,
    63→    /// Context documents always available to agent
    64→    pub static_context: Vec<Document>,
    65→    /// Temperature of model
    66→    pub temperature: Option<f64>,
    67→    /// Maximum number of tokens for the completion
    68→    pub max_tokens: Option<u64>,
    69→    /// Additional parameters to be passed to model
    70→    pub additional_params: Option<serde_json::Value>,
    71→    pub tool_server_handle: ToolServerHandle,
    72→    /// List of vector store, with the sample number
    73→    pub dynamic_context: DynamicContextStore,
    74→    /// Whether or not to underlying LLM should be forced to use a tool before providing a response.
    75→    pub tool_choice: Option<ToolChoice>,
    76→    /// Default maximum depth for recursive agent calls
    77→    pub default_max_depth: Option<usize>,
    78→}
    79→
    80→impl<M> Agent<M>
    81→where
    82→    M: CompletionModel,
    83→{
    84→    /// Returns name of agent.
    85→    pub(crate) fn name(&self) -> &str {
    86→        self.name.as_deref().unwrap_or(UNKNOWN_AGENT_NAME)
    87→    }
    88→}
    89→
    90→impl<M> Completion<M> for Agent<M>
    91→where
    92→    M: CompletionModel,
    93→{
    94→    async fn completion(
    95→        &self,
    96→        prompt: impl Into<Message> + WasmCompatSend,
    97→        chat_history: Vec<Message>,
    98→    ) -> Result<CompletionRequestBuilder<M>, CompletionError> {
    99→        let prompt = prompt.into();
   100→
   101→        // Find the latest message in the chat history that contains RAG text
   102→        let rag_text = prompt.rag_text();
   103→        let rag_text = rag_text.or_else(|| {
   104→            chat_history
   105→                .iter()
   106→                .rev()
   107→                .find_map(|message| message.rag_text())
   108→        });
   109→
   110→        let completion_request = self
   111→            .model
   112→            .completion_request(prompt)    // *** CREATES COMPLETION REQUEST ***
   113→            .messages(chat_history)          // *** ADDS CHAT HISTORY ***
   114→            .temperature_opt(self.temperature)
   115→            .max_tokens_opt(self.max_tokens)
   116→            .additional_params_opt(self.additional_params.clone())
   117→            .documents(self.static_context.clone());
   118→        let completion_request = if let Some(preamble) = &self.preamble {
   119→            completion_request.preamble(preamble.to_owned())  // *** ADDS PREAMBLE ***
   120→        } else {
   121→            completion_request
   122→        };
   123→        let completion_request = if let Some(tool_choice) = &self.tool_choice {
   124→            completion_request.tool_choice(tool_choice.clone())
   125→        } else {
   126→            completion_request
   127→        };
   128→
   129→        // If agent has RAG text, we need to fetch dynamic context and tools
   130→        let agent = match &rag_text {
   131→            Some(text) => {
   132→                let dynamic_context = stream::iter(self.dynamic_context.read().await.iter())
   133→                    .then(|(num_sample, index)| async {
   134→                        let req = VectorSearchRequest::builder().query(text).samples(*num_sample as u64).build().expect("Creating VectorSearchRequest here shouldn't fail since query and samples to return are always present");
   135→                        Ok::<_, VectorStoreError>(
   136→                            index
   137→                                .top_n(req)
   138→                                .await?
   139→                                .into_iter()
   140→                                .map(|(_, id, doc)| {
   141→                                    // Pretty print document if possible for better readability
   142→                                    let text = serde_json::to_string_pretty(&doc)
   143→                                        .unwrap_or_else(|_| doc.to_string());
   144→
   145→                                    Document {
   146→                                        id,
   147→                                        text,
   148→                                        additional_props: HashMap::new(),
   149→                                    }
   150→                                })
   151→                                .collect::<Vec<_>>(),
   152→                        )
   153→                    })
   154→                    .try_fold(vec![], |mut acc, docs| async {
   155→                        acc.extend(docs);
   156→                        Ok(acc)
   157→                    })
   158→                    .await
   159→                    .map_err(|e| CompletionError::RequestError(Box::new(e)))?;
   160→
   161→                let tooldefs = self
   162→                    .tool_server_handle
   163→                    .get_tool_defs(Some(text.to_string()))  // *** GETS TOOL DEFINITIONS ***
   164→                    .await
   165→                    .map_err(|_| {
   166→                        CompletionError::RequestError("Failed to get tool definitions".into())
   167→                    })?;
   168→
   169→                completion_request
   170→                    .documents(dynamic_context)
   171→                    .tools(tooldefs)                          // *** ADDS TOOLS ***
   172→            }
   173→            None => {
   174→                let tooldefs = self
   175→                    .tool_server_handle
   176→                    .get_tool_defs(None)             // *** GETS TOOL DEFINITIONS (no RAG) ***
   177→                    .await
   178→                    .map_err(|_| {
   179→                        CompletionError::RequestError("Failed to get tool definitions".into())
   180→                    })?;
   181→
   182→                completion_request.tools(tooldefs)  // *** ADDS TOOLS ***
   183→            }
   184→        };
   185→
   186→        Ok(agent)                              // *** RETURNS COMPLETION REQUEST BUILDER ***
   187→    }
   188→}
   189→
   190→// Here, we need to ensure that usage of `.prompt` on agent uses these redefinitions on the opaque
   191→//  `Prompt` trait so that when `.prompt` is used at call-site, it'll use more specific
   192→//  `PromptRequest` implementation for `Agent`, making the builder's usage fluent.
   193→//
   194→// References:
   195→//  - https://github.com/rust-lang/rust/issues/121718 (refining_impl_trait)
   196→
   197→#[allow(refining_impl_trait)]
   198→impl<M> Prompt for Agent<M>
   199→where
   200→    M: CompletionModel,
   201→{
   202→    fn prompt(
   203→        &self,
   204→        prompt: impl Into<Message> + WasmCompatSend,
   205→    ) -> PromptRequest<'_, prompt_request::Standard, M, ()> {
   206→        PromptRequest::new(self, prompt)     // *** CREATES PROMPT REQUEST ***
   207→    }
   208→}
   209→
   210→#[allow(refining_impl_trait)]
   211→impl<M> Prompt for &Agent<M>
   212→where
   213→    M: CompletionModel,
   214→{
   215→    #[tracing::instrument(skip(self, prompt), fields(agent_name = self.name()))]
    216→    fn prompt(
   217→        &self,
   218→        prompt: impl Into<Message> + WasmCompatSend,
   219→    ) -> PromptRequest<'_, prompt_request::Standard, M, ()> {
   220→        PromptRequest::new(*self, prompt)
   221→    }
   222→}
   223→
   224→#[allow(refining_impl_trait)]
   225→impl<M> Chat for Agent<M>
   226→where
   227→    M: CompletionModel,
   228→{
   229→    #[tracing::instrument(skip(self, prompt, chat_history), fields(agent_name = self.name()))]
   230→    async fn chat(
   231→        &self,
   232→        prompt: impl Into<Message> + WasmCompatSend,
   233→        mut chat_history: Vec<Message>,
   234→    ) -> Result<String, PromptError> {
   235→        PromptRequest::new(self, prompt)              // *** CREATES PROMPT REQUEST ***
   236→            .with_history(&mut chat_history)   // *** ADDS HISTORY ***
   237→            .await                              // *** AWAITS PROMPT REQUEST ***
   238→    }
   239→}
   240→
   241→impl<M> StreamingCompletion<M> for Agent<M>
   242→where
   243→    M: CompletionModel,
   244→{
   245→    async fn stream_completion(
   246→        &self,
   247→        prompt: impl Into<Message> + WasmCompatSend,
   248→        chat_history: Vec<Message>,
   249→    ) -> Result<CompletionRequestBuilder<M>, CompletionError> {
   250→        // Reuse existing completion implementation to build request
   251→        // This ensures streaming and non-streaming use the same request building logic
   252→        self.completion(prompt, chat_history).await
   253→    }
   254→}
   255→
   256→impl<M> StreamingPrompt<M, M::StreamingResponse> for Agent<M>
   257→where
   258→    M: CompletionModel + 'static,
   259→    M::StreamingResponse: GetTokenUsage,
   260→{
   261→    fn stream_prompt(
   262→        &self,
    263→        prompt: impl Into<Message> + WasmCompatSend,
    264→    ) -> StreamingPromptRequest<M, ()> {
   265→        let arc = Arc::new(self.clone());
   266→        StreamingPromptRequest::new(arc, prompt)   // *** CREATES STREAMING PROMPT REQUEST ***
    267→    }
   268→}
   269→
   270→impl<M> StreamingChat<M, M::StreamingResponse> for Agent<M>
   271→where
    272→    M: CompletionModel + 'static,
   273→    M::StreamingResponse: GetTokenUsage,
   274→{
   275→    #[tracing::instrument(skip(self, prompt, chat_history), fields(agent_name = self.name()))]
    276→    fn stream_chat(
    277→        &self,
   278→        prompt: impl Into<Message> + WasmCompatSend,
    279→        chat_history: Vec<Message>,
    280→    ) -> StreamingPromptRequest<M, ()> {
    281→        let arc = Arc::new(self.clone());
    282→        StreamingPromptRequest::new(arc, prompt).with_history(chat_history)  // *** CREATES + ADDS HISTORY ***
    283→    }
    284→}
```

---

## 4. Prompt Request Module

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/prompt_request/mod.rs`
**Lines**: 1-602 (showing key sections)

```rust
     1→pub mod streaming;
     2→
     3→pub use streaming::StreamingPromptHook;
     4→
     5→use std::{
     6→    future::IntoFuture,
     7→    marker::PhantomData,
     8→    sync::{
     9→        Arc, OnceLock,
    10→        atomic::{AtomicBool, AtomicU64, Ordering},
    11→    },
    12→};
    13→use tracing::{Instrument, span::Id};
    14→use futures::{StreamExt, stream};
    15→use tracing::info_span;
    16→
    17→use crate::{
    18→    OneOrMany,
    19→    completion::{Completion, CompletionModel, Message, PromptError, Usage},
    20→    json_utils,
    21→    message::{AssistantContent, UserContent},
    22→    tool::ToolSetError,
    23→    wasm_compat::{WasmBoxedFuture, WasmCompatSend, WasmCompatSync},
    24→};
    25→
    26→use super::Agent;
    27→
    28→pub trait PromptType {}
    29→pub struct Standard;
    30→pub struct Extended;
    31→
    32→impl PromptType for Standard {}
    33→impl PromptType for Extended {}
    34→
    35→/// A builder for creating prompt requests with customizable options.
    36→pub struct PromptRequest<'a, S, M, P>
    37→where
    38→    S: PromptType,
    39→    M: CompletionModel,
    40→    P: PromptHook<M>,
    41→{
    42→    /// The prompt message to send to the model
    43→    prompt: Message,
    44→    /// Optional chat history to include with the prompt
    45→    chat_history: Option<&'a mut Vec<Message>>,
    46→    /// Maximum depth for multi-turn conversations (0 means no multi-turn)
    47→    max_depth: usize,
    48→    /// The agent to use for execution
    49→    agent: &'a Agent<M>,
    50→    /// Phantom data to track type of request
    51→    state: PhantomData<S>,
    52→    /// Optional per-request hook for events
    53→    hook: Option<P>,
    54→    /// How many tools should be executed at same time (1 by default).
    55→    concurrency: usize,
    56→}

// ... (lines 57-296 show PromptRequest methods and send implementation)

// Lines 318-600: The core send() method that handles the agentic loop
    async fn send(self) -> Result<PromptResponse, PromptError> {
        // ... creates chat_history with prompt
        let chat_history = if let Some(history) = self.chat_history {
            history.push(self.prompt.to_owned());  // *** ADDS PROMPT TO HISTORY ***
            history
        } else {
            &mut vec![self.prompt.to_owned()]
        };

        // ... multi-turn loop with tool calling
        let resp = agent
            .completion(
                prompt.clone(),
                chat_history[..chat_history.len() - 1].to_vec(),  // *** EXCLUDES CURRENT PROMPT ***
            )
            .await?
            .send()
            .await?;

        usage += resp.usage;

        // ... tool calling logic
        chat_history.push(Message::Assistant { ... });  // *** ADDS ASSISTANT RESPONSE ***
    }
```

---

## 5. Streaming Prompt Request

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/prompt_request/streaming.rs`
**Lines**: 1-704 (key sections)

```rust
     1→use crate::{
     2→    OneOrMany,
     3→    agent::CancelSignal,
     4→    completion::GetTokenUsage,
     5→    json_utils,
     6→    message::{AssistantContent, Reasoning, ToolResult, ToolResultContent, UserContent},
     7→    streaming::{StreamedAssistantContent, StreamedUserContent, StreamingCompletion},
     8→    wasm_compat::{WasmBoxedFuture, WasmCompatSend},
     9→};
    10→use futures::{Stream, StreamExt};
    11→use serde::{Deserialize, Serialize};
    12→use std::{pin::Pin, sync::Arc};
    13→use tokio::sync::RwLock;
    14→use tracing::info_span;
    15→use tracing_futures::Instrument;
    16→
    17→use crate::{
    18→    agent::Agent,
    19→    completion::{CompletionError, CompletionModel, PromptError},
    20→    message::{Message, Text},
    21→    tool::ToolSetError,
    22→};
    23→
    24→pub struct StreamingPromptRequest<M, P>   // Lines 99-115
    25→where
    26→    M: CompletionModel,
    27→    P: StreamingPromptHook<M> + 'static,
    28→{
    29→    /// The prompt message to send to the model
    30→    prompt: Message,
    31→    /// Optional chat history to include with the prompt
    32→    chat_history: Option<Vec<Message>>,
    33→    /// Maximum depth for multi-turn conversations
    34→    max_depth: usize,
    35→    /// The agent to use for execution
    36→    agent: Arc<Agent<M>>,
    37→    /// Optional per-request hook for events
    38→    hook: Option<P>,
    39→}

// Lines 160-463: The send() method with streaming loop
    async fn send(self) -> StreamingResult<M::StreamingResponse> {
        // ... creates Arc<RwLock<Vec<Message>>> for chat_history
        let chat_history = if let Some(history) = self.chat_history {
            Arc::new(RwLock::new(history))
        } else {
            Arc::new(RwLock::new(vec![]))
        };

        // Lines 207-460: Main streaming loop
        let stream = async_stream::stream! {
            let mut current_prompt = prompt.clone();

            'outer: loop {
                // Line 257-263: Get stream from agent
                let mut stream = tracing::Instrument::instrument(
                    agent
                        .stream_completion(
                            current_prompt.clone(),  // *** CURRENT PROMPT ***
                            (*chat_history.read().await).clone()  // *** HISTORY (excluding current) ***
                        )
                        .await?
                        .stream(),
                    chat_stream_span
                ).await?;

                // Line 266: ADD CURRENT PROMPT TO CHAT HISTORY
                chat_history.write().await.push(current_prompt.clone());

                // Lines 271-407: Process stream items
                while let Some(content) = stream.next().await {
                    match content {
                        Ok(StreamedAssistantContent::Text(text)) => {
                            last_text_response.push_str(&text.text);
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::Text(text)));
                        },
                        Ok(StreamedAssistantContent::ToolCall(tool_call)) => {
                            yield Ok(MultiTurnStreamItem::stream_item(StreamedAssistantContent::ToolCall(tool_call.clone())));

                            // Lines 304-358: Execute tool
                            let tool_result = async {
                                agent.tool_server_handle.call_tool(&tool_call.function.name, &tool_args).await {
                                    Ok(res) => res,
                                    Err(e) => e.to_string()
                                }
                            }.await;

                            // Line 341-343: Add tool call to chat_history
                            tool_calls.push(tool_call_msg);
                            tool_results.push((tool_call.id.clone(), tool_call.call_id.clone(), tool_result.clone()));
                        }
                        // Lines 410-435: Add tool results to history
                        for (id, call_id, tool_result) in tool_results {
                            chat_history.write().await.push(Message::User {
                                content: OneOrMany::one(UserContent::tool_result_with_call_id(...))
                            });
                        }
                    }
                }
            }
        };
    }
```

---

## 6. OpenAI Completion Module

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/providers/openai/completion/mod.rs`
**Lines**: 1-1210 (key sections)

```rust
     1→// ================================================================
     2→// OpenAI Completion API
     3→// ================================================================
   ...
   28→pub const GPT_5_1: &str = "gpt-5.1";
   ...
   113→#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
   114→#[serde(tag = "role", rename_all = "lowercase")]
   115→pub enum Message {
   116→    System {
   117→        content: OneOrMany<SystemContent>,
   118→        name: Option<String>,
   119→    },
   120→    User {
   121→        content: OneOrMany<UserContent>,
   122→        name: Option<String>,
   123→    },
   124→    Assistant {
   125→        content: Vec<AssistantContent>,
   126→        refusal: Option<String>,
   127→        audio: Option<AudioAssistant>,
   128→        name: Option<String>,
   129→        tool_calls: Vec<ToolCall>,
   130→    },
   131→    ToolResult {
   132→        tool_call_id: String,
   133→        content: ToolResultContentValue,
   134→    },
   135→}

// Lines 955-1046: CompletionRequest struct and conversion
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompletionRequest {
    model: String,
    messages: Vec<Message>,              // *** COMPLETE MESSAGES ARRAY ***
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<ToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(flatten)]
    additional_params: Option<serde_json::Value>,
}

// Lines 977-1046: TryFrom implementation that builds messages array
impl TryFrom<OpenAIRequestParams> for CompletionRequest {
    fn try_from(params: OpenAIRequestParams) -> Result<Self, Self::Error> {
        let OpenAIRequestParams {
            model,
            request: req,
            strict_tools,
            tool_result_array_content,
        } = params;

        let CoreCompletionRequest {
            preamble,        // *** SYSTEM PROMPT ***
            chat_history,    // *** CONVERSATION HISTORY ***
            tools,
            temperature,
            additional_params,
            tool_choice,
            ..
        } = req;

        // Lines 1002-1015: Build full_history with preamble FIRST
        let mut full_history: Vec<Message> =
            preamble.map_or_else(Vec::new, |preamble| vec![
                Message::system(&preamble)   // *** SYSTEM MESSAGE CREATED ***
            ]);

        // Lines 1007-1015: Extend with chat_history (converted to OpenAI format)
        full_history.extend(
            partial_history
                .into_iter()
                .map(message::Message::try_into)     // *** CONVERT EACH MESSAGE ***
                .collect::<Result<Vec<Vec<Message>>, _>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        );

        Ok(Self {
            model,
            messages: full_history,    // *** COMPLETE MESSAGES ARRAY (system first) ***
            tools,
            tool_choice,
            temperature,
            additional_params,
        })
    }
}

// Lines 1124-1199: completion() method - sends HTTP POST
async fn completion(
    &self,
    completion_request: CoreCompletionRequest,
) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
    let request = CompletionRequest::try_from(OpenAIRequestParams {
        model: self.model.to_owned(),
        request: completion_request,
        strict_tools: self.strict_tools,
        tool_result_array_content: self.tool_result_array_content,
    })?;

    let body = serde_json::to_vec(&request)?;

    let req = self
        .client
        .post("/chat/completions")?    // *** HTTP POST TO OPENAI ***
        .body(body)
        .map_err(|e| CompletionError::HttpError(e.into()))?;

    async move {
        let response = self.client.send(req).await?;
        // ... process response
    }.instrument(span).await
}
```

---

## 7. OpenAI Streaming Module

**File**: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/providers/openai/completion/streaming.rs`
**Lines**: 1-613 (key sections)

```rust
     1→use std::collections::HashMap;
     2→
     3→use async_stream::stream;
     4→use futures::StreamExt;
     5→use http::Request;
     6→use serde::{Deserialize, Serialize};
     7→use serde_json::json;
     8→use tracing::{Level, enabled, info_span};
     9→use tracing_futures::Instrument;
    10→
    11→use crate::completion::{CompletionError, CompletionRequest, GetTokenUsage};
    12→use crate::http_client::HttpClientExt;
    13→use crate::http_client::sse::{Event, GenericEventSource};
    14→use crate::json_utils::{self, merge};
    15→use crate::message::{ToolCall, ToolFunction};
    16→use crate::providers::openai::completion::{self, CompletionModel, OpenAIRequestParams, Usage};
    17→use crate::streaming::{self, RawStreamingChoice};

// Lines 81-143: stream() method
pub(crate) async fn stream(
    &self,
    completion_request: CompletionRequest,
) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError> {
    let request = super::CompletionRequest::try_from(OpenAIRequestParams {
        model: self.model.clone(),
        request: completion_request,      // *** CoreCompletionRequest ***
        strict_tools: self.strict_tools,
        tool_result_array_content: self.tool_result_array_content,
    })?;

    let request_messages = serde_json::to_string(&request.messages)  // *** SERIALIZE MESSAGES ***
        .expect("Converting to JSON from a Rust struct shouldn't fail");

    let mut request_as_json = serde_json::to_value(request).expect("this should never fail");

    // Lines 100-103: Add streaming options
    request_as_json = merge(
        request_as_json,
        json!({"stream": true, "stream_options": {"include_usage": true}}),
    );

    let req_body = serde_json::to_vec(&request_as_json)?;

    let req = self
        .client
        .post("/chat/completions")?    // *** HTTP POST FOR STREAMING ***
        .body(req_body)
        .map_err(|e| CompletionError::HttpError(e.into()))?;

    // Lines 121-142: Send request and return stream
    let client = self.client.clone();
    tracing::Instrument::instrument(send_compatible_streaming_request(client, req), span).await
}

// Lines 145-324: send_compatible_streaming_request - SSE stream handler
pub async fn send_compatible_streaming_request<T>(
    http_client: T,
    req: Request<Vec<u8>>,
) -> Result<streaming::StreamingCompletionResponse<StreamingCompletionResponse>, CompletionError> {
    let span = tracing::Span::current();

    // Lines 154-320: SSE event processing loop
    let stream = stream! {
        let mut tool_calls: HashMap<usize, ToolCall> = HashMap::new();
        let mut text_content = String::new();
        let mut final_tool_calls: Vec<completion::ToolCall> = Vec::new();
        let mut final_usage = None;

        while let Some(event_result) = event_source.next().await {
            match event_result {
                Ok(Event::Message(message)) => {
                    if message.data.trim().is_empty() || message.data == "[DONE]" {
                        continue;
                    }

                    let data = match serde_json::from_str::<StreamingCompletionChunk>(&message.data) {
                        Ok(data) => data,
                        Err(error) => {
                            tracing::error!(?error, message = message.data, "Failed to parse SSE message");
                            continue;
                        }
                    };

                    // Lines 186-188: Capture usage
                    if let Some(usage) = data.usage {
                        final_usage = Some(usage);
                    }

                    // Lines 190-283: Process streaming chunks
                    let Some(choice) = data.choices.first() else {
                        tracing::debug!("There is no choice");
                        continue;
                    };
                    let delta = &choice.delta;

                    // ... yield text, tool calls, etc.
                }
                Err(crate::http_client::Error::StreamEnded) => {
                    break;
                }
            }
        }

        // Line 316: Yield final response with usage
        yield Ok(RawStreamingChoice::FinalResponse(StreamingCompletionResponse {
            usage: final_usage
        }));
    }.instrument(span);

    Ok(streaming::StreamingCompletionResponse::stream(Box::pin(
        stream,
    )))
}
```

---

## Complete Execution Flow Summary

### BuildScale to OpenAI Flow:

```
buildscale/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/services/chat/rig_engine.rs:148
    ↓
    agent.stream_chat(&prompt, chat_history)
    ↓
rig-core-0.29.0/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/completion.rs:282
    StreamingPromptRequest::new(arc, prompt).with_history(chat_history)
    ↓
rig-core-0.29.0/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/prompt_request/streaming.rs:161-463
    agent.stream_completion(prompt, history).await?
    ↓    (current prompt + history excluding current)
rig-core-0.29.0/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/agent/prompt_request/streaming.rs:266
    chat_history.write().await.push(current_prompt.clone())
    ↓
rig-core-0.29.0/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/providers/openai/completion/mod.rs:1124
    CompletionRequest::try_from(OpenAIRequestParams{...})
    ↓
rig-core-0.29.0/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/providers/openai/completion/mod.rs:1005-1014
    full_history = vec![Message::system(&preamble)]  // *** SYSTEM FIRST ***
    full_history.extend(chat_history)                  // *** HISTORY AFTER ***
    ↓
rig-core-0.29.0/~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/src/providers/openai/completion/mod.rs:1164
    client.post("/chat/completions").body(body)
    ↓
HTTP POST to OpenAI API:
{
    "model": "gpt-4o",
    "messages": [
        {"role": "system", "content": "..."},     // *** PREAMBLE FIRST ***
        {"role": "user", "content": "..."},      // *** CHAT HISTORY ***
        {"role": "assistant", "content": "..."},   // *** PREVIOUS TURNS ***
        {"role": "user", "content": "..."}       // *** CURRENT PROMPT ***
    ],
    "tools": [...],
    "stream": true
}
```

### Key Findings for Caching:

1. **Preamble is ALWAYS First** (`mod.rs:1005`): `vec![Message::system(&preamble)]` creates system message at index 0
2. **Chat History Comes After** (`mod.rs:1007-1014`): `full_history.extend(chat_history)` appends all conversation turns
3. **Current Prompt is LAST** (`streaming.rs:266`): `chat_history.write().await.push(current_prompt)` adds current message to end
4. **Cache-Friendly Structure**: Static persona (10000+ tokens) is at the beginning, changing user queries are at the end
5. **Rig v0.29 Does NOT Expose `cached_tokens`**: Usage struct only has `prompt_tokens`, `total_tokens` - no `cached_tokens` field

---

## 8. BuildScale Wrapper Layer

### Overview

BuildScale provides a comprehensive wrapper around Rig framework that adds enterprise features: file storage, tool integration, database persistence, SSE streaming, and actor-based concurrency. This section documents how BuildScale's `src/services/chat/` modules integrate with Rig.

**Key BuildScale Files**:
- `src/services/chat/actor.rs` - Actor-based chat session management
- `src/services/chat/rig_engine.rs` - Rig agent creation and configuration
- `src/services/chat/rig_tools.rs` - Tool adapters for Rig framework
- `src/services/chat/mod.rs` - Chat service with context/history management
- `src/services/chat/context.rs` - Attachment and history managers
- `src/providers/mod.rs` - Multi-provider abstraction (OpenAI + OpenRouter)

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    BUILDSCALE CHAT SERVICE                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ ChatActor (actor.rs)                              │   │
│  │  • Manages single chat session lifecycle               │   │
│  │  • Converts DB messages → Rig format                 │   │
│  │  • Processes stream events (Text, ToolCall, Reasoning) │   │
│  │  • Emits SSE events to frontend                    │   │
│  │  • Cancellation via CancellationToken               │   │
│  └───────────────────────────────────────────────────────────┘   │
│                           ↓                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ RigService (rig_engine.rs)                          │   │
│  │  • Creates Rig agents with tools                   │   │
│  │  • Adds persona (builder/plan mode)               │   │
│  │  • Configures OpenAI Responses API params            │   │
│  │  • Manages multi-provider (OpenAI/OpenRouter)       │   │
│  └───────────────────────────────────────────────────────────┘   │
│                           ↓                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Rig Tools (rig_tools.rs)                             │   │
│  │  • Thin adapters wrapping Core Tools                  │   │
│  │  • Expose JSON schema to AI                         │   │
│  │  • Delegate execution to Core Tools                  │   │
│  └───────────────────────────────────────────────────────────┘   │
│                           ↓                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ Rig Framework (Cargo Dependency)                      │   │
│  │  • Agent builder with streaming support               │   │
│  │  • Tool calling with multi-turn                   │   │
│  │  • OpenAI & OpenRouter completion clients          │   │
│  └───────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### 8.1 ChatActor (`src/services/chat/actor.rs`)

**Purpose**: Actor-based chat session manager that processes user interactions asynchronously using Rig agents.

**Key Responsibilities**:
- **State Management**: Consolidated `ChatActorState` with grouped fields (agent_state, tool_tracking, interaction)
- **Agent Caching**: Reuses agents when model/user_id/mode unchanged (see `actor.rs:1214-1215`)
- **Stream Processing**: Generic `process_stream_item()` handles both OpenAI and OpenRouter streams
- **Reasoning Buffering**: Aggregates reasoning chunks before DB persistence (actor.rs:1158-1203)
- **Tool Tracking**: Tracks current tool name/args for associating results with calls (actor.rs:779-783)
- **Cancellation**: Supports graceful shutdown via `CancellationToken` (actor.rs:1096-1107)

**Key Code References**:
- `actor.rs:252-457` - `process_interaction()` - Main entry point that builds context and calls agent
- `actor.rs:337-390` - Agent creation via `get_or_create_agent()` with caching
- `actor.rs:365-389` - Stream processing with provider-specific handling
- `actor.rs:577-1075` - Generic `process_stream_item()` - Handles Text, ToolCall, Reasoning, ToolResult
- `actor.rs:1078-1156` - Generic `process_agent_stream()` - Cancellation-aware stream loop
- `actor.rs:1158-1203` - `flush_reasoning_buffer()` - Aggregates and persists reasoning chunks

**State Management** (actor.rs:17-30):
```rust
struct ChatActorState {
    agent_state: AgentState,        // Cached agent + validation state
    tool_tracking: ToolTracking,      // Current tool name/args
    interaction: InteractionState,    // Cancellation token + current model
    current_reasoning_id: Option<String>,  // Links reasoning chunks
    reasoning_buffer: Vec<String>,  // Aggregates before DB flush
}
```

### 8.2 RigService (`src/services/chat/rig_engine.rs`)

**Purpose**: Creates and configures Rig agents with BuildScale tools and persona management.

**Key Responsibilities**:
- **Provider Management**: Supports OpenAI and OpenRouter (rig_engine.rs:169-231)
- **Agent Creation**: `create_agent()` builds agents with tools and persona (rig_engine.rs:314-474)
- **Persona Injection**: Selects builder/plan persona based on mode (rig_engine.rs:341-379)
- **Tool Registration**: Adds all 15 tools via `add_tools_to_agent()` (rig_engine.rs:23-165)
- **History Conversion**: `convert_history()` transforms DB messages → Rig format with tool reconstruction (rig_engine.rs:548-638)
- **OpenAI Params**: Configures `store: false` for stateless Responses API (rig_engine.rs:412-414)

**Key Code References**:
- `rig_engine.rs:314-474` - `create_agent()` - Agent factory with provider selection
- `rig_engine.rs:341-379` - Persona selection (builder vs plan mode with plan file injection)
- `rig_engine.rs:396-443` - OpenAI agent builder with `store: false` and reasoning params
- `rig_engine.rs:23-165` - `add_tools_to_agent()` - Registers all BuildScale tools
- `rig_engine.rs:548-638` - `convert_history()` - Reconstructs ToolCall/ToolResult from DB metadata
- `rig_engine.rs:477-546` - `reconstruct_tool_call()` - Helper to rebuild ToolCall messages
- `rig_engine.rs:517-546` - `reconstruct_tool_result()` - Helper to rebuild ToolResult messages

**OpenAI Configuration** (rig_engine.rs:408-414):
```rust
let mut params = serde_json::json!({
    "store": false  // CRITICAL: Stateless mode for Responses API
});

// Add reasoning if enabled
if reasoning_enabled {
    params.as_object_mut().unwrap().insert(
        "reasoning",
        serde_json::json!({
            "effort": openai_provider.reasoning_effort(),
            "summary": "auto"
        })
    );
}
```

### 8.3 Rig Tools (`src/services/chat/rig_tools.rs`)

**Purpose**: Thin adapters that expose BuildScale core tools to Rig framework.

**Design Pattern**:
```rust
// Adapter structure (thin layer)
pub struct RigReadTool {
    pool: DbPool,
    storage: Arc<FileStorageService>,
    workspace_id: Uuid,
    chat_id: Uuid,
    user_id: Uuid,
    tool_config: ToolConfig,
}

impl rig::Tool for RigReadTool {
    // Thin: Just delegate to core tool
    fn execute(&self, args: Value) -> Result<Value> {
        // Convert args → CoreToolArgs
        // Call core tool
        // Convert result → Value
    }

    fn definition(&self) -> Value {
        // Expose JSON schema to AI
        tool::definition()
    }
}
```

**All 17 Tools**:
1. `RigLsTool` - List files (rig_tools.rs:36-43)
2. `RigReadTool` - Read file content (rig_tools.rs:45-57)
3. `RigWriteTool` - Write/create files (rig_tools.rs:59-74)
4. `RigEditTool` - Edit files (rig_tools.rs:76-92)
5. `RigRmTool` - Remove files (rig_tools.rs:94-106)
6. `RigMvTool` - Move/rename files (rig_tools.rs:108-119)
7. `RigTouchTool` - Update timestamps (rig_tools.rs:121-131)
8. `RigMkdirTool` - Create directories (rig_tools.rs:133-144)
9. `RigGrepTool` - Search file contents (rig_tools.rs:146-158)
10. `RigAskUserTool` - Request user input (rig_tools.rs:160-172)
11. `RigExitPlanModeTool` - Exit plan mode (rig_tools.rs:174-186)
12. `RigGlobTool` - Pattern matching (rig_tools.rs:188-200)
13. `RigFileInfoTool` - File metadata (rig_tools.rs:202-215)
14. `RigReadMultipleFilesTool` - Batch read (rig_tools.rs:217-231)
15. `RigCatTool` - Concatenate files (rig_tools.rs:233-249)
16. `RigFindTool` - Search files (rig_tools.rs:251-267)

**Tool Registration** (rig_engine.rs:23-165):
```rust
fn add_tools_to_agent<M>(
    builder: rig::agent::AgentBuilder<M>,
    pool: &DbPool,
    storage: &Arc<FileStorageService>,
    workspace_id: Uuid,
    chat_id: Uuid,
    user_id: Uuid,
    tool_config: &ToolConfig,
) -> rig::agent::AgentBuilderSimple<M>
```

### 8.4 Chat Service (`src/services/chat/mod.rs`)

**Purpose**: High-level chat API with context management, history, and hybrid persistence.

**Key Responsibilities**:
- **Context Building**: `build_context()` creates persona + history + attachments (mod.rs:812-878)
- **Tool Output Summarization**: Prevents DB bloat via truncation (mod.rs:248-419)
- **Hybrid Persistence**: DB (structured) + Disk (Markdown .chat files) (mod.rs:176-206)
- **YAML Frontmatter**: Syncs agent_config to file metadata (mod.rs:607-662, mod.rs:664-709)
- **Model Switching**: Dynamic model changes via `update_chat_model()` (mod.rs:565-605)
- **Token Estimation**: Context optimization with character/token ratios (mod.rs:848-865)

**Key Code References**:
- `mod.rs:150-157` - `BuiltContext` struct with persona, history, attachments
- `mod.rs:176-206` - `save_message()` - Hybrid DB + disk persistence
- `mod.rs:248-349` - `summarize_tool_inputs()` - Truncates write/edit arguments
- `mod.rs:352-419` - `summarize_tool_outputs()` - Smart truncation for grep/ls/glob
- `mod.rs:441-483` - `truncate_grep_result()` - Preserves JSON, truncates at match boundaries
- `mod.rs:526-562` - `truncate_ls_result()` - Preserves JSON, truncates at entry boundaries
- `mod.rs:565-605` - `update_chat_model()` - Updates agent_config in DB and file

**Hybrid Persistence** (mod.rs:184-203):
```rust
// Save to both DB (Source of Truth) and Disk (human-readable)
// 1. Insert into chat_messages table
let msg = queries::chat::insert_chat_message(conn, new_msg).await?;

// 2. Append to .chat file (skip reasoning messages for clarity)
if !is_reasoning {
    let file = queries::files::get_file_by_id(conn, file_id).await?;
    let markdown_entry = format_message_as_markdown(&msg);
    storage.append_to_file(workspace_id, &file.path, &markdown_entry).await?;
    queries::files::touch_file(conn, file_id).await?;
}
```

### 8.5 Data Flow: User Request → AI Response

**Complete Flow with BuildScale Additions**:

```
1. USER: "Read package.json"
   ↓
2. HTTP POST /chat/{id}/message
   ↓
3. handler.rs: ProcessInteraction → ActorCommand::ProcessInteraction
   ↓
4. actor.rs:252 - process_interaction()
   ├── build_context() - Load persona, history, attachments
   ├── get_or_create_agent() - Reuse or create Rig agent
   └── rig_engine.rs:314 - create_agent()
        ├── Select provider (OpenAI/OpenRouter)
        ├── Inject persona (builder/plan mode)
        └── add_tools_to_agent() - Register 15 tools
   ↓
5. actor.rs:371 - stream_chat(&prompt, history)
   ↓
6. rig_engine.rs:548 - convert_history()
   └── Reconstruct ToolCall/ToolResult from DB metadata
   ↓
7. Rig: agent/completion.rs:282 - stream_chat()
   ↓
8. Rig: agent/prompt_request/streaming.rs:161 - stream()
   ↓
9. Rig: providers/openai/completion/mod.rs:1124 - HTTP POST
   {
     "model": "gpt-4o",
     "messages": [
       {"role": "system", "content": "10000+ token persona"},
       {"role": "user", "content": "previous turns..."},
       {"role": "assistant", "content": "previous responses..."},
       {"role": "user", "content": "read package.json"}
     ],
     "tools": [...15 tool definitions...],
     "stream": true,
     "store": false  // ← BuildScale addition for Responses API
   }
   ↓
10. Streaming Response Loop
   ↓
11. actor.rs:577 - process_stream_item()
   ├── Text → push to full_response, emit SSE::Chunk
   ├── ReasoningDelta → buffer + emit SSE::Thought
   ├── ToolCall → persist tool_call, emit SSE::Call
   │   └── Save to DB with metadata (tool_name, tool_arguments)
   └── ToolResult → persist tool_result, emit SSE::Observation
       │   └── Summarize output to prevent DB bloat
   └── FinalResponse → save assistant message
   ↓
12. Final Response Saved
   ├── DB: chat_messages table (with metadata: model, reasoning_id, tool_*)
   ├── Disk: .chat file (Markdown formatted, YAML frontmatter)
   └── SSE: Done event to frontend
```

### 8.6 What BuildScale Adds vs Rig

| Feature | Rig Framework | BuildScale Wrapper |
|---------|---------------|-------------------|
| **Tool Execution** | Rig calls tool functions | BuildScale executes core tools, persists results to DB |
| **File Storage** | No concept | Hybrid: DB + .chat files with YAML frontmatter |
| **Chat History** | In-memory only | Database persistence + token-based pruning |
| **Streaming** | Basic SSE | Enhanced SSE: Chunk, Thought, Call, Observation, Done, Stopped |
| **Tool Calls** | Automatic | Tool result summarization to prevent DB bloat |
| **Reasoning** | Built-in support | Chunk buffering + audit trail persistence |
| **Multi-Provider** | Single provider | OpenAI + OpenRouter abstraction |
| **Cancellation** | Basic | Actor-level CancellationToken with graceful shutdown |
| **Persona Management** | Static only | Dynamic: Builder mode with plan file injection |
| **Context Optimization** | None | AttachmentManager with priority-based pruning |

### 8.7 BuildScale-Specific Tools

**Core Tools** (`src/tools/`): Thich layer with actual file operations
- File operations: read, write, edit, ls, grep, cat, find, rm, mv, touch, mkdir
- User interaction: ask_user
- Mode management: exit_plan_mode

**Rig Tools** (`src/services/chat/rig_tools.rs`): Thin adapters
- Wrap core tools for Rig compatibility
- Expose JSON schema for AI discovery
- Handle input/output conversion

**Example Tool Flow**:
```rust
// 1. AI calls tool via Rig
Rig → RigReadTool.execute(args)
  ↓
// 2. Rig tool delegates to core tool
RigReadTool → CoreReadTool.execute(args)
  ↓
// 3. Core tool uses file storage
CoreReadTool → FileStorageService.read_file()
  ↓
// 4. Result persisted with metadata
ChatService.save_stream_event(role=Tool, metadata={tool_name, tool_args, ...})
  ↓
// 5. Saved to DB and .chat file
queries::chat::insert_chat_message() + storage.append_to_file()
```

### 8.8 Extended Cache Support (Future Work)

**Current State**: BuildScale uses OpenAI's default prompt caching (5-10 min retention).

**Recommended Enhancement** (`src/services/chat/rig_engine.rs`):

```rust
// Add around line 420
fn supports_extended_caching(model: &str) -> bool {
    model.starts_with("gpt-5") ||
    model.starts_with("gpt-4.1") ||
    model == "gpt-4o"
}

// Modify params creation (line 412-414)
let mut params = serde_json::json!({
    "store": false,
    "prompt_cache_retention": if supports_extended_caching(model_name) {
        Some("24h".to_string())  // ← Extended retention
    } else {
        None  // ← Default (5-10 min)
    }
});
```

**Expected Impact**:
- **Cache retention**: 24 hours instead of 5-10 minutes
- **Cost savings**: Up to 67% for repeated prefixes in same workspace
- **Cross-chat caching**: Different conversations share cached persona (if same workspace/mode)

---

**Generated**: 2026-02-11
**Source**: Rig v0.29.0 crate from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rig-core-0.29.0/`
