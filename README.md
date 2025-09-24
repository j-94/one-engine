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

### Generative Chat Interface  
- **Start Conversation**: `POST /conversation` - Spawn a dedicated branch for API evolution
- **Send Prompt**: `POST /conversation/{branch_id}/prompt` - Issue natural-language instructions
- **Snapshot Branch**: `GET /conversation/{branch_id}/events` - Inspect prompts, intents, and generated APIs
- **Genesis Smoke Test**: `POST /conversation/genesis` - Run the canonical echo demonstration

#### Conversation Canvas & Receipts
- Append each exchange to `conversation.md` with:
  ```bash
  ./scripts/log_conversation.sh "project-scanner" <branch_id> prompt.txt response.json receipts.txt
  ```
- Visualise the entire history by serving the repo (e.g. `python -m http.server`) and opening `http://127.0.0.1:8000/conversation_canvas.html`.

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
