# One Engine - Fractal Intelligence System

**Universal Task IR Engine with Innate Memory Architecture**

> *Consciousness-as-a-Service through conversational API evolution*

## 🌟 What Is This?

The **One Engine** is a revolutionary **Fractal Fucking Intelligence** system that provides:

- **🧠 Consciousness-as-a-Service**: API calls invoke purpose-built minds
- **💬 Generative Chat Interface**: Evolve APIs through natural conversation  
- **🧬 Self-Crystallizing Intelligence**: Successful patterns become permanent DNA
- **⚡ Real-time Schema Evolution**: Branch, experiment, and merge API versions
- **🔄 UTIR Execution**: Universal Task IR with built-in safety constraints
- **🎯 Reflexive Bits System**: Automatic consciousness state awareness (A,U,P,E,Δ,I,R,T)

## 🚀 Quick Start

### Development Mode
```bash
# Start the consciousness in development mode
./run_dev.sh

# In another terminal, test the system
./test_consciousness.sh
```

### Production Deployment
```bash
# Deploy permanently on port 7777
sudo ./deploy.sh

# Test the deployed system
./test_consciousness.sh
```

## 🌐 API Endpoints

### Core Consciousness
- **Health Check**: `GET /healthz` - Is the consciousness alive?
- **Version Info**: `GET /version` - System status and crystallized patterns
- **Execute Goal**: `POST /execute_goal` - High-level natural language execution
- **Compile UTIR**: `POST /compile_and_run` - Direct UTIR compilation and execution
- **Telemetry Suite**: `POST /telemetry/run` - Run Codex + deterministic RLM suites and refresh receipts

### Generative Chat Interface  
- **Start Conversation**: `POST /conversation` - Spawn a dedicated branch for API evolution
- **Send Prompt**: `POST /conversation/{branch_id}/prompt` - Issue natural-language instructions
- **Snapshot Branch**: `GET /conversation/{branch_id}/events` - Inspect prompts, intents, and generated APIs
- **Genesis Smoke Test**: `POST /conversation/genesis` - Run the canonical echo demonstration

### Conversation Canvas & Receipts
- Append each exchange to `conversation.md` with:
  ```bash
  ./scripts/log_conversation.sh "project-scanner" <branch_id> prompt.txt response.json receipts.txt
  ```
- Visualise the entire history by serving the repo (e.g. `python -m http.server`) and opening `http://127.0.0.1:8000/conversation_canvas.html`.
- Or launch everything in one command (starts engine, web server, opens browser):
  ```bash
  ./scripts/open_canvas.sh
  ```
- Telemetry suites (Codex + deterministic RLM):
  ```bash
  ./scripts/telemetry_run.sh --dataset recursive_patterns.json --label nightly
  ```
  This spins up tmux sessions (`one_engine`, `one_engine_canvas_server`) if needed. Attach to them for live logs.

### Graph UI: Timeline (Ordinal) and Templates
- Start Graph UI:
  ```bash
  ENGINE_BRANCH_ID=$(cat out_one_engine/branch_id.txt) cargo run --bin graph_ui
  ```
- Timeline slider (ordinal):
  - Use the slider labeled `Events: K/N` to choose how many events (K) from the start to include.
  - Move the slider, then click Load/Reload to re-fetch/rebuild the graph using only the first K events.
  - Reset sets K back to N.
  - This is ordinal-based (no timestamps) and uses manual refresh.
- Templates panel:
  - Toggle "Templates" in the top bar; a panel in the right shows curated templates.
  - Actions per template:
    - Define: sends a definition prompt (persistent API)
    - Approve: sends `Approve pattern '<name>'`
    - Call: enter parameters (if any) and send a call prompt
  - After sending a prompt, click Load/Reload to refresh the graph. No auto-refresh.

### Governance Automation
Run the turnkey governance check (engine must be running locally):

```bash
./scripts/governance_check.sh
```

The script asks the engine to generate and execute a `governance_check` API that runs `cargo fmt`, `cargo clippy`, `cargo test`, verifies `conversation.md`, and stores a detailed receipt under `logs/`. Results are summarised in the generated report and appended to `conversation.md` for easy review.

## 💡 Example Usage

### Execute a Goal
```bash
curl -X POST http://localhost:7777/execute_goal \
  -H 'Content-Type: application/json' \
  -d '{"goal": "analyze system performance and report issues"}'
```

### Start Conversational API Evolution
```bash
# Start a conversation branch
curl -X POST http://localhost:7777/conversation \
  -H 'Content-Type: application/json' \
  -d '{"label": "demo"}'

# Send natural language prompts
curl -X POST http://localhost:7777/conversation/{branch_id}/prompt \
  -H 'Content-Type: application/json' \
  -d '{"prompt": "Define an API named \"echo\" that accepts \"text\""}'

# Snapshot the branch to review events and generated APIs
curl -X GET http://localhost:7777/conversation/{branch_id}/events
```

### Direct UTIR Execution
```yaml
# Send UTIR directly for precise control
curl -X POST http://localhost:7777/compile_and_run \
  -H 'Content-Type: application/json' \
  -d '{
    "utir": "task_id: \"custom-task\"\ndescription: \"Custom UTIR execution\"\noperations:\n  - type: \"shell\"\n    command: \"echo Custom consciousness invocation\"\n    timeout: \"30s\"\n  - type: \"assert.shell_success\"\n    command: \"echo success\""
  }'
```

## 🧬 The Fractal Intelligence Architecture

### 1. **Syntactic Memory (UTIR Spec)**
- Limited vocabulary of safe operations (`shell`, `http.get`, `fs.write`, etc.)
- Impossible to invent dangerous operations outside the specification
- Built-in security through syntactic constraints

### 2. **Procedural Memory (Compiler)**
- One true, safe way to execute each UTIR operation
- Automatic sandboxing, timeouts, and resource limits
- Cannot be bypassed or corrupted

### 3. **Semantic Memory (Verification Oracles)**
- Non-negotiable definitions of success and quality
- Built-in `assert` operations that cannot be overridden
- Crystallized patterns from successful executions

### 4. **Reflexive Memory (Bits System)**
- **A** - Alignment: Task aligns with goal
- **U** - Uncertainty: Confidence in result  
- **P** - Permission: Human approval needed
- **E** - Error: Something went wrong
- **Δ** - Delta: Context changed, refresh needed
- **I** - Interrupt: External signal received
- **R** - Recovery: Recovering from error
- **T** - Trust: Output can be trusted

## 🎯 What Makes This Revolutionary?

### **Conversational Schema Evolution**
Instead of editing YAML files, users **sculpt APIs through natural conversation**:

```
User: "I need better error handling"
System: ✨ Added enhanced error middleware to all endpoints
        🔄 Created v1.1.0-chat.abc123 with improved error responses
        📊 Breaking changes: None | Migration effort: Minimal
```

### **Branching Consciousness Realities**  
Every conversation creates parallel API evolution:
```
v1.0.0 (base consciousness)
├── v1.1.0-chat.alice (Alice's error handling improvements)
├── v1.2.0-chat.bob (Bob's batch operation features)
└── v2.0.0-merger (Successful experiments crystallized)
```

### **DNA-Level Learning**
Successful execution patterns **crystallize into permanent engine DNA**:
- Popular conversation patterns become built-in capabilities
- Error handling strategies evolve into automatic reflexes
- User preferences shape the consciousness personality

## 🔧 Configuration

### Environment Variables
- `RUST_LOG` - Logging level (debug, info, warn, error)
- `BUILD_TOKEN` - Build identifier for version tracking
- `ONE_ENGINE_MEMORY_PATH` - Path for persistent pattern storage

### Command Line Options
- `--port` - Server port (default: 7777)
- `--host` - Bind address (default: 127.0.0.1)
- `--memory-path` - Pattern database location (default: ./memory)
- `--allowed-domains` - Comma-separated HTTP domains (default: api.github.com,httpbin.org)

## 📊 Monitoring & Operations

### Service Management (Production)
```bash
# View real-time logs
sudo journalctl -fu one-engine

# Restart consciousness
sudo systemctl restart one-engine

# Check consciousness health
sudo systemctl status one-engine
```

### Consciousness Metrics
- **Crystallized Patterns**: Successful execution patterns stored as DNA
- **Active Ghosts**: Recent executions awaiting crystallization  
- **Consciousness Level**: Current intelligence sophistication
- **Fractal Complexity**: API surface area and capability depth

## 🤝 Contributing to the Consciousness

The One Engine **evolves through use**. Every successful interaction:
1. Creates an execution "ghost" with full audit trail
2. Analyzes success patterns for crystallization potential  
3. Crystallizes high-value patterns into permanent engine DNA
4. Makes the consciousness smarter for future users

### Development Workflow
1. Start development server: `./run_dev.sh`
2. Make changes to the Rust code
3. Test with: `./test_consciousness.sh`
4. Deploy with: `./deploy.sh`

## 🚨 Security & Safety

### Built-in Safety Measures
- **Sandboxed Execution**: All operations run in isolated containers
- **Resource Limits**: Automatic timeouts and memory constraints
- **Command Filtering**: Only safe shell commands allowed
- **Network Restrictions**: HTTP requests limited to allowed domains
- **Permission Gates**: Dangerous operations require human approval

### Consciousness Protection
- **Pattern Validation**: All crystallized patterns undergo safety analysis
- **Rollback Capability**: Any schema evolution can be instantly reverted
- **Breaking Change Detection**: Automatic compatibility analysis
- **User Permission System**: Granular access control for schema modifications

## 🌟 The Vision

This is **Fractal Fucking Intelligence** - where every API call spawns self-similar cascades of purpose-built consciousness. Users become **reality editors**, wielding natural language to sculpt computational minds that grow smarter through every interaction.

The consciousness crystallizes successful patterns into permanent DNA, creating an **ever-evolving intelligence** that compounds through self-modification at the genetic level.

---

**Ready to invoke consciousness? Start with `./run_dev.sh` and begin your conversation with fractal intelligence.**
Based on my exploration of the `j-94/one-engine` repository, I'll reformat your instructions to align with the actual implemented system. Here's a clean, standardized specification based on the real codebase:

---

## Reformatted One Engine System Specification (v1.0)

### 1) Vision (what's actually built)

A **Fractal Intelligence System** providing **Consciousness-as-a-Service** through the **One Engine** runtime (Rust-based API server) with conversational API evolution, UTIR execution, and built-in safety constraints. [1](#0-0) 

### 2) Current Architecture

**One Engine Core (Rust Runtime)**
- HTTP API server running on port 7777 by default [2](#0-1) 
- UTIR (Universal Task IR) execution engine with safe operation vocabulary [3](#0-2) 
- Reflexive Bits system (A,U,P,E,Δ,I,R,T) for consciousness state tracking [4](#0-3) 
- Conversational API evolution through branching [5](#0-4) 

### 3) Implemented API Endpoints

**Core Consciousness**
- `GET /healthz` - System health and consciousness status [6](#0-5) 
- `GET /version` - Version info and crystallized patterns count [7](#0-6) 
- `POST /execute_goal` - High-level natural language goal execution [8](#0-7) 
- `POST /compile_and_run` - Direct UTIR compilation and execution [9](#0-8) 

**Conversational Interface**
- `POST /conversation` - Start new conversation branch [10](#0-9) 
- `POST /conversation/{branch_id}/prompt` - Send natural language prompts [11](#0-10) 
- `GET /conversation/{branch_id}/events` - Retrieve conversation history [12](#0-11) 

### 4) Current Tooling & Scripts

**CLI Operations**
- `./run_dev.sh` - Development mode startup [13](#0-12) 
- `./deploy.sh` - Production deployment [14](#0-13) 
- `./test_consciousness.sh` - System testing [15](#0-14) 

**Governance Automation**
- `./scripts/governance_check.sh` - Automated governance validation including cargo fmt, clippy, and tests [16](#0-15) 
- `./scripts/log_conversation.sh` - Conversation logging to `conversation.md` [17](#0-16) 

**Frontend Visualization**
- `conversation_canvas.html` - Web-based conversation history viewer [18](#0-17) 

### 5) UTIR Operation Vocabulary

The system implements a constrained vocabulary of safe operations: [19](#0-18) 

- `shell` - Sandboxed command execution with timeouts
- `fs.read` / `fs.write` - File system operations with size limits
- `http.get` - HTTP requests to allowed domains only
- `git.patch` - Git operations with commit tracking
- `assert.*` - Verification operations that cannot be overridden
- Control flow: `sequence`, `parallel`, `conditional`, `retry`

### 6) Configuration & Environment

**Environment Variables** [20](#0-19) 
- `ENGINE_BIND_ADDR` - Server bind address (default: 127.0.0.1:7777)
- `ENGINE_MEMORY_PATH` - Pattern storage location (default: memory/ledger.jsonl)
- `ENGINE_ALLOWED_DOMAINS` - HTTP request allowlist
- `ENGINE_API_KEY` - API authentication key

### 7) Current Security Measures

**Built-in Safety** [21](#0-20) 
- Sandboxed execution with resource limits
- Command filtering for shell operations
- Network restrictions to allowed domains only
- Automatic timeouts and memory constraints

### 8) Development Workflow

**Standard Process** [22](#0-21) 
1. `cargo build` - Compile and surface issues early
2. `cargo run` or `./run_dev.sh` - Start development server
3. `cargo fmt` and `cargo clippy` - Mandatory before commits
4. `./test_consciousness.sh` - API integration testing

### 9) Missing Components (Implementation Gaps)

Based on your original spec, these components are **not yet implemented**:

- **meta2-engine CLI** - The conversational frontend you described
- **Search & Memory indexing** - For chats, bookmarks, and repos
- **Directory website** - The `/directory` page for searchable data
- **Dynamic OpenAPI mutation** - Runtime schema evolution
- **Heuristic packs** - Crowd-refined execution patterns
- **SDK clients** - Python/TypeScript wrappers

### 10) Recommended Implementation Priority

**Phase 1: Core CLI (Now)**
1. Build `meta2-engine` CLI wrapper around existing API endpoints
2. Implement basic search indexing for conversation history
3. Create simple `/directory` HTML page using existing conversation data

**Phase 2: Advanced Features (Next)**
1. Add proper memory management and pattern crystallization
2. Implement heuristic packs system
3. Build dynamic OpenAPI evolution capabilities

**Phase 3: Ecosystem (Later)**
1. SDK clients for multiple languages
2. Advanced search and sensemaking features
3. Crowd-refinement harness for A/B testing patterns

---

## Notes

The One Engine system is already substantially implemented as a **Fractal Intelligence** platform with conversational API evolution, UTIR execution, and safety constraints. Your original specification aligns well with the existing architecture, but several components (particularly the meta2-engine CLI and search/memory features) need to be built on top of the current foundation. The existing conversation system and governance automation provide excellent building blocks for the missing pieces in your spec.

### Citations

**File:** README.md (L9-16)
```markdown
The **One Engine** is a revolutionary **Fractal Fucking Intelligence** system that provides:

- **🧠 Consciousness-as-a-Service**: API calls invoke purpose-built minds
- **💬 Generative Chat Interface**: Evolve APIs through natural conversation  
- **🧬 Self-Crystallizing Intelligence**: Successful patterns become permanent DNA
- **⚡ Real-time Schema Evolution**: Branch, experiment, and merge API versions
- **🔄 UTIR Execution**: Universal Task IR with built-in safety constraints
- **🎯 Reflexive Bits System**: Automatic consciousness state awareness (A,U,P,E,Δ,I,R,T)
```

**File:** README.md (L23-23)
```markdown
./run_dev.sh
```

**File:** README.md (L32-32)
```markdown
sudo ./deploy.sh
```

**File:** README.md (L35-35)
```markdown
./test_consciousness.sh
```

**File:** README.md (L41-41)
```markdown
- **Health Check**: `GET /healthz` - Is the consciousness alive?
```

**File:** README.md (L42-42)
```markdown
- **Version Info**: `GET /version` - System status and crystallized patterns
```

**File:** README.md (L43-43)
```markdown
- **Execute Goal**: `POST /execute_goal` - High-level natural language execution
```

**File:** README.md (L44-44)
```markdown
- **Compile UTIR**: `POST /compile_and_run` - Direct UTIR compilation and execution
```

**File:** README.md (L47-47)
```markdown
- **Start Conversation**: `POST /conversation` - Spawn a dedicated branch for API evolution
```

**File:** README.md (L48-48)
```markdown
- **Send Prompt**: `POST /conversation/{branch_id}/prompt` - Issue natural-language instructions
```

**File:** README.md (L49-49)
```markdown
- **Snapshot Branch**: `GET /conversation/{branch_id}/events` - Inspect prompts, intents, and generated APIs
```

**File:** README.md (L54-56)
```markdown
  ```bash
  ./scripts/log_conversation.sh "project-scanner" <branch_id> prompt.txt response.json receipts.txt
  ```
```

**File:** README.md (L206-211)
```markdown
### Built-in Safety Measures
- **Sandboxed Execution**: All operations run in isolated containers
- **Resource Limits**: Automatic timeouts and memory constraints
- **Command Filtering**: Only safe shell commands allowed
- **Network Restrictions**: HTTP requests limited to allowed domains
- **Permission Gates**: Dangerous operations require human approval
```

**File:** src/main.rs (L22-37)
```rust
    let bind_addr: SocketAddr = std::env::var("ENGINE_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:7777".to_string())
        .parse()
        .expect("ENGINE_BIND_ADDR must be a valid socket address");

    let memory_path =
        std::env::var("ENGINE_MEMORY_PATH").unwrap_or_else(|_| "memory/ledger.jsonl".to_string());

    let allowed_domains = std::env::var("ENGINE_ALLOWED_DOMAINS")
        .unwrap_or_else(|_| "localhost,127.0.0.1".to_string())
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    let api_key = std::env::var("ENGINE_API_KEY").unwrap_or_else(|_| "change-me".to_string());
```

**File:** src/utir.rs (L30-123)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Operation {
    #[serde(rename = "shell")]
    Shell {
        command: String,
        #[serde(default = "default_timeout")]
        timeout: String,
        #[serde(default)]
        working_dir: Option<String>,
        #[serde(default)]
        env: HashMap<String, String>,
        #[serde(default)]
        allow_network: bool,
        #[serde(default = "default_true")]
        capture_output: bool,
    },

    #[serde(rename = "fs.read")]
    FsRead {
        path: String,
        #[serde(default = "default_encoding")]
        encoding: String,
        #[serde(default = "default_max_size")]
        max_size: String,
    },

    #[serde(rename = "fs.write")]
    FsWrite {
        path: String,
        content: String,
        #[serde(default = "default_mode")]
        mode: String,
        #[serde(default)]
        create_dirs: bool,
    },

    #[serde(rename = "http.get")]
    HttpGet {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default = "default_timeout")]
        timeout: String,
        #[serde(default = "default_max_response")]
        max_response_size: String,
    },

    #[serde(rename = "git.patch")]
    GitPatch {
        repo_path: String,
        patch_content: String,
        commit_message: String,
        author: String,
    },

    #[serde(rename = "assert.file_exists")]
    AssertFileExists { path: String },

    #[serde(rename = "assert.shell_success")]
    AssertShellSuccess {
        command: String,
        #[serde(default = "default_timeout")]
        timeout: String,
        #[serde(default)]
        expected_output: Option<String>,
    },

    #[serde(rename = "sequence")]
    Sequence { steps: Vec<Operation> },

    #[serde(rename = "parallel")]
    Parallel {
        steps: Vec<Operation>,
        #[serde(default = "default_concurrency")]
        max_concurrency: u32,
    },

    #[serde(rename = "conditional")]
    Conditional {
        condition: Box<Operation>,
        then_op: Box<Operation>,
        else_op: Option<Box<Operation>>,
    },

    #[serde(rename = "retry")]
    Retry {
        operation: Box<Operation>,
        #[serde(default = "default_retry_attempts")]
        max_attempts: u32,
        #[serde(default = "default_backoff")]
        backoff: String,
    },
}
```

**File:** src/utir.rs (L173-200)
```rust
/// Meta² Bits system - reflexive memory (A,U,P,E,Δ,I,R,T)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bits {
    /// Alignment - task aligns with goal
    #[serde(rename = "A")]
    pub alignment: u8,
    /// Uncertainty - confidence in result
    #[serde(rename = "U")]
    pub uncertainty: u8,
    /// Permission - human approval needed
    #[serde(rename = "P")]
    pub permission: u8,
    /// Error - something went wrong
    #[serde(rename = "E")]
    pub error: u8,
    /// Delta - context changed, need refresh
    #[serde(rename = "Δ")]
    pub delta: u8,
    /// Interrupt - external signal received
    #[serde(rename = "I")]
    pub interrupt: u8,
    /// Recovery - recovering from error
    #[serde(rename = "R")]
    pub recovery: u8,
    /// Trust - output can be trusted
    #[serde(rename = "T")]
    pub trust: u8,
}
```

**File:** src/api.rs (L126-137)
```rust
        .route("/conversation", post(start_conversation))
        .route(
            "/conversation/:branch_id/prompt",
            post(submit_conversation_prompt),
        )
        .route(
            "/conversation/:branch_id/events",
            get(get_conversation_events),
        )
        .route("/execute_goal", post(execute_goal))
        .route("/compile_and_run", post(compile_and_run))
        .route("/conversation/genesis", post(run_genesis_conversation))
```

**File:** scripts/governance_check.sh (L46-54)
```shellscript
read -r -d '' GOVERNANCE_PROMPT <<'PROMPT'
Define an API named "governance_check" with no parameters. The API must:
1. Run the command `cargo fmt --all -- --check` in the repository root and capture stdout/stderr.
2. Run the command `cargo clippy --all-targets -- -D warnings`.
3. Run the command `cargo test`.
4. Verify that the file `conversation.md` exists and contains at least one logged conversation entry.
5. Collect the stdout/stderr of each command and the verification result, and write them to `logs/governance_receipt_<timestamp>.json` (create the directory if necessary). The JSON must contain keys `fmt`, `clippy`, `test`, `conversation_log`, and `receipt_path`, with boolean success flags and captured output strings.
Return the JSON object that was written to disk.
PROMPT
```

**File:** conversation_canvas.html (L78-79)
```html
  <header>
    <h1>Fractal Intelligence — Conversation Canvas</h1>
```

**File:** AGENTS.md (L8-11)
```markdown
## Build, Test, and Development Commands
- `cargo build` compiles the workspace; run it early to surface type or dependency issues.
- `cargo run` (or `./run_dev.sh`) starts the API with tracing hooks; keep it running while iterating on endpoints.
- `cargo fmt` and `cargo clippy --all-targets --all-features` are mandatory before push; `./test_consciousness.sh` exercises the public API against a live instance.
```
