# Mistral Integration Guide

> Technical reference for integrating Devstral models into ClawedCommand's AI agent system.

## Model Selection

### Devstral 2 (123B) — Server-Side Competitive

| Attribute | Detail |
|---|---|
| **API Model ID** | `devstral-2-2512` |
| **HuggingFace** | `mistralai/Devstral-2-123B-Instruct-2512` |
| **Architecture** | Dense transformer (not MoE) |
| **Parameters** | 123B |
| **Context Window** | 256K tokens |
| **License** | Modified MIT |
| **SWE-bench** | 72.2% |
| **Pricing** | $0.40 input / $2.00 output per 1M tokens |
| **Hardware** | 4x H100 minimum |

Dense architecture was chosen over MoE specifically because multi-step agentic tool-calling tasks suffer when knowledge is fragmented across experts.

### Devstral Small 2 (24B) — Local Single-Player

| Attribute | Detail |
|---|---|
| **API Model ID** | `devstral-small-2-2512` |
| **HuggingFace** | `mistralai/Devstral-Small-2-24B-Instruct-2512` |
| **Architecture** | Dense transformer |
| **Parameters** | 24B |
| **Context Window** | 256K tokens (384K via Ollama) |
| **License** | Apache 2.0 |
| **SWE-bench** | 68.0% |
| **Pricing** | $0.10 input / $0.30 output per 1M tokens |
| **Hardware** | Single RTX 4090 at Q4_K_M (~14GB) |
| **Vision** | Yes (multimodal — could be used for map screenshots) |

### Both Models Support
- Native tool use / function calling
- Structured output via tool call responses
- Fill-in-the-middle (FIM) for code editing
- Streaming responses
- Optimal inference temperature: **0.2**

---

## Inference Architecture

### Routing Strategy

```
Player issues command
        │
        ▼
┌──────────────────┐
│  Mode Detection   │
└────────┬─────────┘
         │
    ┌────┴────┐
    │         │
    ▼         ▼
Competitive  Local/Practice
    │         │
    ▼         ▼
Mistral API  Local Engine
(Devstral 2) (Devstral Small 2)
    │         │
    └────┬────┘
         │
         ▼
  Tool Call Response
         │
         ▼
  Command Queue → ECS
```

### Server-Side (Competitive/Ranked)

Call the Mistral API directly. All players use the same model to ensure fairness.

```python
from mistralai import Mistral

client = Mistral(api_key=os.environ["MISTRAL_API_KEY"])

response = client.chat.complete(
    model="devstral-2-2512",
    messages=messages,
    tools=GAME_TOOLS,
    tool_choice="auto",
    temperature=0.2,
)
```

### Client-Side (Single-Player/Practice)

Deploy Devstral Small 2 locally. Two options ranked by quality:

**Option 1: vLLM (recommended — best tool-calling quality)**
```bash
vllm serve mistralai/Devstral-Small-2-24B-Instruct-2512 \
    --max-model-len 65536 \
    --tool-call-parser mistral \
    --enable-auto-tool-choice \
    --port 8080
```

**Option 2: Ollama (easier setup, slightly lower quality)**
```bash
ollama run devstral-small-2
```

Both expose an OpenAI-compatible endpoint, so the Rust client code is identical regardless of backend — just change the base URL.

### Quantization Options for Local Deployment

| Quantization | Size | Quality | Fits On |
|---|---|---|---|
| Q3_K_M | 11.5 GB | Moderate | 16GB VRAM |
| **Q4_K_M** | **14.3 GB** | **Good (recommended)** | **RTX 4090 (24GB)** |
| Q5_K_M | 16.8 GB | Very Good | 24GB VRAM |
| Q6_K | 19.3 GB | Excellent | 24GB VRAM |
| Q8_0 | 25.1 GB | Near-lossless | 32GB+ RAM |
| BF16 | 47.2 GB | Full precision | 48GB+ VRAM |

Q4_K_M on a single RTX 4090 gives ~57K usable context — sufficient for game state + conversation history.

GGUF quantizations available from:
- [Unsloth](https://huggingface.co/unsloth/Devstral-Small-2-24B-Instruct-2512-GGUF)
- [Bartowski](https://huggingface.co/bartowski/mistralai_Devstral-Small-2-24B-Instruct-2512-GGUF)

---

## Tool Use / Function Calling

### Defining Game Tools

Mistral uses OpenAI-compatible tool definitions. Each MCP tool maps to a Mistral function:

```python
GAME_TOOLS = [
    {
        "type": "function",
        "function": {
            "name": "get_units",
            "description": "Query own units, optionally filtered by type, location, or status",
            "parameters": {
                "type": "object",
                "properties": {
                    "unit_type": {
                        "type": "string",
                        "description": "Filter by unit type (e.g., 'infantry', 'cavalry', 'worker')",
                        "enum": ["infantry", "cavalry", "vehicle", "worker", "all"]
                    },
                    "region": {
                        "type": "object",
                        "description": "Filter by map region (bounding box)",
                        "properties": {
                            "x_min": {"type": "integer"},
                            "y_min": {"type": "integer"},
                            "x_max": {"type": "integer"},
                            "y_max": {"type": "integer"}
                        }
                    },
                    "status": {
                        "type": "string",
                        "enum": ["idle", "moving", "attacking", "gathering", "all"]
                    }
                }
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "move_units",
            "description": "Order units to move to a target grid position",
            "parameters": {
                "type": "object",
                "properties": {
                    "unit_ids": {
                        "type": "array",
                        "items": {"type": "integer"},
                        "description": "IDs of units to move"
                    },
                    "target": {
                        "type": "object",
                        "properties": {
                            "x": {"type": "integer"},
                            "y": {"type": "integer"}
                        },
                        "required": ["x", "y"]
                    }
                },
                "required": ["unit_ids", "target"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "attack_units",
            "description": "Order units to attack a target enemy unit or position",
            "parameters": {
                "type": "object",
                "properties": {
                    "unit_ids": {
                        "type": "array",
                        "items": {"type": "integer"},
                        "description": "IDs of units to issue attack order to"
                    },
                    "target_id": {
                        "type": "integer",
                        "description": "Enemy entity ID to attack"
                    },
                    "target_position": {
                        "type": "object",
                        "description": "Attack-move to this position (if no target_id)",
                        "properties": {
                            "x": {"type": "integer"},
                            "y": {"type": "integer"}
                        }
                    }
                },
                "required": ["unit_ids"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "build",
            "description": "Place a building at the specified grid position",
            "parameters": {
                "type": "object",
                "properties": {
                    "building_type": {
                        "type": "string",
                        "enum": ["command_center", "barracks", "refinery", "vehicle_factory",
                                 "tech_lab", "power_plant", "radar_station", "armory", "supply_depot"]
                    },
                    "position": {
                        "type": "object",
                        "properties": {
                            "x": {"type": "integer"},
                            "y": {"type": "integer"}
                        },
                        "required": ["x", "y"]
                    }
                },
                "required": ["building_type", "position"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "train_unit",
            "description": "Queue a unit for production at a building",
            "parameters": {
                "type": "object",
                "properties": {
                    "building_id": {
                        "type": "integer",
                        "description": "ID of the production building"
                    },
                    "unit_type": {
                        "type": "string",
                        "enum": ["worker", "infantry", "heavy_infantry", "cavalry",
                                 "light_vehicle", "heavy_vehicle"]
                    }
                },
                "required": ["building_id", "unit_type"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_visible_enemies",
            "description": "Get all enemy units and buildings currently visible through fog of war",
            "parameters": {
                "type": "object",
                "properties": {
                    "region": {
                        "type": "object",
                        "description": "Optional: limit to a map region",
                        "properties": {
                            "x_min": {"type": "integer"},
                            "y_min": {"type": "integer"},
                            "x_max": {"type": "integer"},
                            "y_max": {"type": "integer"}
                        }
                    }
                }
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_resources",
            "description": "Get current resource counts for the player",
            "parameters": {
                "type": "object",
                "properties": {}
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_buildings",
            "description": "Query own buildings, optionally filtered by type",
            "parameters": {
                "type": "object",
                "properties": {
                    "building_type": {
                        "type": "string",
                        "description": "Filter by building type, or omit for all"
                    }
                }
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "get_map_info",
            "description": "Get terrain data for a map region (elevation, passability, resources)",
            "parameters": {
                "type": "object",
                "properties": {
                    "region": {
                        "type": "object",
                        "properties": {
                            "x_min": {"type": "integer"},
                            "y_min": {"type": "integer"},
                            "x_max": {"type": "integer"},
                            "y_max": {"type": "integer"}
                        },
                        "required": ["x_min", "y_min", "x_max", "y_max"]
                    }
                },
                "required": ["region"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "set_rally_point",
            "description": "Set the rally point for a production building",
            "parameters": {
                "type": "object",
                "properties": {
                    "building_id": {"type": "integer"},
                    "position": {
                        "type": "object",
                        "properties": {
                            "x": {"type": "integer"},
                            "y": {"type": "integer"}
                        },
                        "required": ["x", "y"]
                    }
                },
                "required": ["building_id", "position"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "patrol",
            "description": "Set units to patrol between waypoints",
            "parameters": {
                "type": "object",
                "properties": {
                    "unit_ids": {
                        "type": "array",
                        "items": {"type": "integer"}
                    },
                    "waypoints": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "x": {"type": "integer"},
                                "y": {"type": "integer"}
                            },
                            "required": ["x", "y"]
                        }
                    }
                },
                "required": ["unit_ids", "waypoints"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "gather_resource",
            "description": "Send workers to gather from a resource deposit",
            "parameters": {
                "type": "object",
                "properties": {
                    "worker_ids": {
                        "type": "array",
                        "items": {"type": "integer"}
                    },
                    "resource_id": {
                        "type": "integer",
                        "description": "ID of the resource deposit entity"
                    }
                },
                "required": ["worker_ids", "resource_id"]
            }
        }
    },
    {
        "type": "function",
        "function": {
            "name": "execute_strategy",
            "description": "Execute a reusable strategy script in the WASM sandbox",
            "parameters": {
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Strategy script code to execute"
                    },
                    "name": {
                        "type": "string",
                        "description": "Optional name to save this strategy for reuse"
                    }
                },
                "required": ["code"]
            }
        }
    }
]
```

### Tool Call Flow

The full conversation loop between the model and the game:

```python
import json
from mistralai import Mistral

client = Mistral(api_key=api_key)

def run_agent_turn(game_state_summary: str, player_instruction: str):
    """Execute one full agent turn: player instruction → tool calls → game commands."""

    messages = [
        {
            "role": "system",
            "content": (
                "You are the AI commander for a ClawedCommand army. "
                "You receive natural language orders from your player and execute them "
                "by calling game tools. Always query the game state before acting. "
                "Think step-by-step about strategy before issuing commands."
            )
        },
        {
            "role": "user",
            "content": f"Current game state:\n{game_state_summary}\n\nOrder: {player_instruction}"
        }
    ]

    # Allow up to 10 tool-call rounds per turn
    for _ in range(10):
        response = client.chat.complete(
            model="devstral-small-2-2512",
            messages=messages,
            tools=GAME_TOOLS,
            tool_choice="auto",
            temperature=0.2,
        )

        assistant_msg = response.choices[0].message

        if not assistant_msg.tool_calls:
            # Model is done — return its final text response
            return assistant_msg.content

        # Process tool calls
        messages.append(assistant_msg)
        for tool_call in assistant_msg.tool_calls:
            fn_name = tool_call.function.name
            fn_args = json.loads(tool_call.function.arguments)

            # Execute against the game engine
            result = execute_game_tool(fn_name, fn_args)

            messages.append({
                "role": "tool",
                "name": fn_name,
                "content": json.dumps(result),
                "tool_call_id": tool_call.id,
            })

    return "Agent reached maximum tool call rounds."
```

### MCP ↔ Mistral Tool Format Conversion

MCP tool definitions map almost 1:1 to Mistral's format. The key differences:

| MCP | Mistral |
|---|---|
| `tool.inputSchema` | `function.parameters` (same JSON Schema) |
| Result is `content[]` array | Result is a JSON string in `role: "tool"` |
| No call IDs | `tool_call_id` must be echoed back |
| Arguments as object | Arguments as **stringified JSON** |

Conversion (Rust pseudocode for `cc_agent`):

```rust
/// Convert an MCP tool definition to Mistral function calling format
fn mcp_to_mistral_tool(mcp_tool: &McpTool) -> MistralTool {
    MistralTool {
        r#type: "function".to_string(),
        function: MistralFunction {
            name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone(),
            parameters: mcp_tool.input_schema.clone(), // Same JSON Schema
        },
    }
}

/// Convert a Mistral tool call response to an MCP tool execution request
fn mistral_to_mcp_call(tool_call: &MistralToolCall) -> McpCallRequest {
    McpCallRequest {
        name: tool_call.function.name.clone(),
        arguments: serde_json::from_str(&tool_call.function.arguments).unwrap(),
    }
}

/// Convert an MCP tool result to a Mistral tool message
fn mcp_result_to_mistral_message(
    tool_call_id: &str,
    result: &McpToolResult,
) -> MistralMessage {
    let content = result.content.iter()
        .filter_map(|item| match item {
            McpContent::Text(t) => Some(t.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    MistralMessage {
        role: "tool".to_string(),
        content,
        tool_call_id: Some(tool_call_id.to_string()),
        name: None,
    }
}
```

---

## Rust Client Integration

### Using reqwest (recommended for production)

Both the Mistral API and local vLLM/Ollama expose an OpenAI-compatible endpoint. A single HTTP client works for both:

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    tools: Vec<Tool>,
    tool_choice: String,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
}

pub struct MistralClient {
    http: Client,
    base_url: String,  // "https://api.mistral.ai/v1" or "http://localhost:8080/v1"
    api_key: String,
    model: String,
}

impl MistralClient {
    /// Create client for Mistral API (competitive mode)
    pub fn remote(api_key: String) -> Self {
        Self {
            http: Client::new(),
            base_url: "https://api.mistral.ai/v1".to_string(),
            api_key,
            model: "devstral-2-2512".to_string(),
        }
    }

    /// Create client for local inference (single-player mode)
    pub fn local(port: u16) -> Self {
        Self {
            http: Client::new(),
            base_url: format!("http://localhost:{}/v1", port),
            api_key: "not-needed".to_string(),
            model: "mistralai/Devstral-Small-2-24B-Instruct-2512".to_string(),
        }
    }

    pub async fn chat(&self, request: ChatRequest) -> Result<ChatResponse, reqwest::Error> {
        self.http
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .json()
            .await
    }
}
```

### Unofficial Rust Crate

`mistralai-client` on crates.io provides a typed wrapper but is community-maintained. For ClawedCommand, the `reqwest` approach above is preferred since:
- Both API and local endpoints use the same OpenAI-compatible format
- We control serialization/deserialization exactly
- No dependency on third-party crate update cycles

---

## Comparison Models for Fine-Tuning

In addition to Devstral, we evaluate multiple models to find the best tool-calling performance for game commands.

### Qwen2.5-Coder-32B-Instruct (Primary Candidate)

| Attribute | Detail |
|---|---|
| **HuggingFace** | `Qwen/Qwen2.5-Coder-32B-Instruct` |
| **Parameters** | 32B |
| **Context** | 128K tokens |
| **License** | Apache 2.0 |
| **Aider Score** | 73.7 |
| **McEval** | 65.9 (40+ languages) |
| **Training** | LoRA via Unsloth on 1x A100 80GB |
| **Inference** | vLLM on A100, or Q4_K_M on RTX 4090 for local |

Top open-weight coding model at its size. Strong baseline code understanding means less training data needed.

### Codestral via Mistral API (Quick Baseline)

| Attribute | Detail |
|---|---|
| **API Model ID** | `codestral-latest` |
| **Training** | Mistral API fine-tuning (~$3/M training tokens, ~30 min turnaround) |
| **Purpose** | Validate data quality before spending GPU hours |

Fastest path to a fine-tuned model. Upload JSONL, start job, get results quickly.

### xLAM-2-8B (Stretch Goal)

| Attribute | Detail |
|---|---|
| **HuggingFace** | `Salesforce/xLAM-2-8b-fc-r` |
| **Parameters** | 8B |
| **Specialty** | Purpose-built for function calling (#3 on Berkeley FC Leaderboard) |
| **Training** | Very fast LoRA via Unsloth on any GPU |
| **Inference** | Lightweight, runs on consumer hardware |

Dark horse candidate. Outperforms GPT-4o on tau-bench (56.2% vs 52.9%). If the game primarily needs accurate tool calling over creative code generation, this could be the best local inference option.

### Training Framework: Unsloth + TRL

All local fine-tuning uses Unsloth (2x speed, 70% less VRAM vs vanilla) with TRL's SFTTrainer. LoRA config starting point: rank 32, alpha 64, dropout 0.0, all linear layers targeted. Training configs in `training/configs/`.

---

## Fine-Tuning Pipeline

### Overview

```
Game Replays → Replay Converter → Training JSONL → Mistral Fine-Tune API → Fine-Tuned Model
                                                           │
                                        ┌──────────────────┴──────────────────┐
                                        │                                     │
                                  API Fine-Tune                    Self-Hosted Fine-Tune
                                  (Mistral hosted)                 (mistral-finetune repo)
                                        │                                     │
                                  Hosted model                     Local checkpoint
                                  (use via API)                    (deploy via vLLM)
```

### Training Data Format (JSONL)

Each line represents one complete interaction: player instruction → tool calls → result. The `tools` array must be present alongside `messages`.

```json
{
  "messages": [
    {
      "role": "system",
      "content": "You are the AI commander for a ClawedCommand army. Execute the player's orders using available tools."
    },
    {
      "role": "user",
      "content": "Game state: 12 infantry idle at (10,5). 3 enemy scouts visible at (20,8). 500 credits, 1 barracks.\n\nOrder: Push those scouts back and start training more infantry."
    },
    {
      "role": "assistant",
      "tool_calls": [
        {
          "id": "aB3cD4eF5",
          "type": "function",
          "function": {
            "name": "attack_units",
            "arguments": "{\"unit_ids\": [1,2,3,4,5,6,7,8,9,10,11,12], \"target_position\": {\"x\": 20, \"y\": 8}}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "content": "{\"status\": \"ok\", \"units_moving\": 12}",
      "tool_call_id": "aB3cD4eF5"
    },
    {
      "role": "assistant",
      "tool_calls": [
        {
          "id": "xY7zW8qR9",
          "type": "function",
          "function": {
            "name": "get_buildings",
            "arguments": "{\"building_type\": \"barracks\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "content": "{\"buildings\": [{\"id\": 100, \"type\": \"barracks\", \"queue\": [], \"position\": {\"x\": 5, \"y\": 3}}]}",
      "tool_call_id": "xY7zW8qR9"
    },
    {
      "role": "assistant",
      "tool_calls": [
        {
          "id": "pQ1rS2tU3",
          "type": "function",
          "function": {
            "name": "train_unit",
            "arguments": "{\"building_id\": 100, \"unit_type\": \"infantry\"}"
          }
        }
      ]
    },
    {
      "role": "tool",
      "content": "{\"status\": \"queued\", \"queue_position\": 1, \"train_time\": 15}",
      "tool_call_id": "pQ1rS2tU3"
    },
    {
      "role": "assistant",
      "content": "All 12 infantry are attack-moving toward the enemy scouts at (20,8). I've also queued another infantry unit at the barracks — it'll be ready in 15 seconds."
    }
  ],
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "attack_units",
        "description": "Order units to attack a target enemy unit or position",
        "parameters": {
          "type": "object",
          "properties": {
            "unit_ids": {"type": "array", "items": {"type": "integer"}},
            "target_id": {"type": "integer"},
            "target_position": {"type": "object", "properties": {"x": {"type": "integer"}, "y": {"type": "integer"}}}
          },
          "required": ["unit_ids"]
        }
      }
    },
    {
      "type": "function",
      "function": {
        "name": "get_buildings",
        "description": "Query own buildings, optionally filtered by type",
        "parameters": {
          "type": "object",
          "properties": {
            "building_type": {"type": "string"}
          }
        }
      }
    },
    {
      "type": "function",
      "function": {
        "name": "train_unit",
        "description": "Queue a unit for production at a building",
        "parameters": {
          "type": "object",
          "properties": {
            "building_id": {"type": "integer"},
            "unit_type": {"type": "string"}
          },
          "required": ["building_id", "unit_type"]
        }
      }
    }
  ]
}
```

### Critical Training Data Requirements

1. **Tool call IDs**: Exactly 9 random alphanumeric characters
2. **Arguments**: Must be stringified JSON, not parsed objects
3. **Message ordering**: user → assistant (tool_calls) → tool (result) → assistant (next call or final text)
4. **Loss masking**: Only computed on assistant messages (including tool_calls)
5. **`tools` array**: Must be present at the top level of each JSONL entry
6. **Weight control**: Add `"weight": 0` to any assistant message to exclude from training

### API Fine-Tuning

```python
from mistralai import Mistral

client = Mistral(api_key=os.environ["MISTRAL_API_KEY"])

# Upload training data
train_file = client.files.upload(
    file={"file_name": "cc_train.jsonl", "content": open("cc_train.jsonl", "rb")}
)
eval_file = client.files.upload(
    file={"file_name": "cc_eval.jsonl", "content": open("cc_eval.jsonl", "rb")}
)

# Create and start fine-tuning job
job = client.fine_tuning.jobs.create(
    model="mistral-small-latest",
    training_files=[{"file_id": train_file.id, "weight": 1}],
    validation_files=[eval_file.id],
    hyperparameters={
        "training_steps": 300,
        "learning_rate": 0.0001,
    },
    auto_start=True,
)

# Monitor
import time
while True:
    status = client.fine_tuning.jobs.get(job_id=job.id)
    print(f"Status: {status.status}")
    if status.status in ["SUCCESS", "FAILED", "CANCELLED"]:
        break
    time.sleep(30)

# Use the fine-tuned model
response = client.chat.complete(
    model=status.fine_tuned_model,  # e.g., "ft:mistral-small-latest:xxx:20260228"
    messages=[...],
    tools=GAME_TOOLS,
    tool_choice="auto",
    temperature=0.2,
)
```

**API fine-tuning pricing**: ~$4 minimum per job + ~$1-9 per million training tokens + $2-4/month model storage.

**Supported base models for API fine-tuning**:
- `open-mistral-7b`
- `mistral-small-latest`
- `codestral-latest`
- `open-mistral-nemo`
- `mistral-large-latest`
- `ministral-8b-latest`
- `ministral-3b-latest`

### Self-Hosted Fine-Tuning

For more control or to fine-tune the actual Devstral weights, use the `mistral-finetune` repo:

```bash
git clone https://github.com/mistralai/mistral-finetune
cd mistral-finetune
pip install -r requirements.txt
```

Config (`training/configs/cc_finetune.yaml`):
```yaml
model_id_or_path: "/models/Devstral-Small-2-24B-Instruct-2512"
run_dir: "/checkpoints/cc_v1"
seq_len: 8192
batch_size: 4
max_steps: 500

optim:
  lr: 1e-4
  weight_decay: 0.1
  pct_start: 0.1

lora:
  rank: 64
  alpha: 16
  dropout: 0.05
  target_modules: ["q_proj", "v_proj"]

data:
  instruct_data: "/data/cc_tool_use.jsonl:5.,/data/general_instruct.jsonl:1."
  eval_instruct_data: "/data/cc_eval.jsonl"

eval_freq: 50
log_freq: 10
seed: 42
```

The `5.,1.` weighting means game-specific tool-use data is sampled 5x more often than general instruction data.

```bash
# Validate dataset
python -m utils.validate_data --train_yaml training/configs/cc_finetune.yaml

# Train (8x H100 node)
torchrun --nproc-per-node 8 --master_port $RANDOM -m train training/configs/cc_finetune.yaml
```

### Replay → Training Data Pipeline

This is the `tools/replay_converter/` component in the project structure:

```
Game Replay (binary)
    │
    ▼
Extract frames: [(tick, game_state, commands_issued)]
    │
    ▼
Cluster into "turns": group commands by player intent
    │
    ▼
Generate natural language descriptions (can use LLM for this)
    │
    ▼
Format as JSONL: {messages: [...], tools: [...]}
    │
    ▼
Split: 90% train / 10% eval
```

### Training Data Strategy

**Phase 1 — Bootstrap (50 gold + 500-1000 synthetic)**:
- Hand-author 50 gold examples covering: basic commands, economy, multi-step tactics, complex strategy, negative examples, error recovery
- Generate 500-1000 synthetic variations via Claude API (`training/scripts/generate_synthetic.py`)
- Quality filtering: JSON schema validation, tool call ID format, argument types, message ordering, balanced tool distribution
- Validate with `training/scripts/validate_data.py`, convert formats with `training/scripts/convert_format.py`
- Quick baseline: fine-tune Codestral via Mistral API (`training/scripts/train_mistral_api.py`)
- Full training: Unsloth LoRA on Qwen2.5-Coder-32B and Devstral Small 2 (`training/scripts/train_unsloth.py`)
- Evaluate all models on identical held-out set (`training/scripts/evaluate.py`)

**Phase 2 — Self-play augmentation**:
- Run the winning fine-tuned model against scripted AI opponents
- Record successful games as replays
- Convert replays to training data
- Filter for games where the agent won or performed well

**Phase 3 — Human replay data**:
- Record human player sessions (with consent)
- Pair human commands with the game states they saw
- Highest quality data — human strategic reasoning

**Phase 4 (Stretch) — DPO alignment**:
- Use game outcomes (win/loss) as preference signal
- Generate paired completions (good move vs bad move) from replays
- DPO training via TRL's DPOTrainer for strategic improvement beyond SFT

### Evaluation Harness

Measure fine-tuned model quality:

| Metric | How |
|---|---|
| **Tool call accuracy** | Does the model call the right tools with valid arguments? |
| **Win rate vs scripted AI** | Play 100 games against difficulty tiers |
| **Instruction following** | Given specific orders, does it execute them correctly? |
| **Economy efficiency** | Resources gathered per minute, supply utilization |
| **Response latency** | Time from instruction to first tool call |
| **Token efficiency** | Tokens consumed per successful game action |

---

## Cost Estimation

### Server-Side (Competitive Play)

Assumptions: ~20 tool calls per player turn, ~500 tokens input + 200 tokens output per call.

| Per Turn | Devstral 2 |
|---|---|
| Input tokens | ~10,000 → $0.004 |
| Output tokens | ~4,000 → $0.008 |
| **Total per turn** | **~$0.012** |

At ~60 turns per game → **~$0.72 per player per game** (or ~$1.44 per match for both players).

### Local (Single-Player)

Free after hardware cost. Player needs:
- RTX 4090 (24GB VRAM) for Q4_K_M quantization
- Or Mac with 32GB+ unified memory for Q8_0

### Fine-Tuning

| Item | Cost |
|---|---|
| API fine-tune (1000 examples, ~500K tokens) | ~$5-10 |
| Self-hosted (8x H100 for 1 hour) | ~$25-30 (cloud) |
| Model storage (API) | $2-4/month |

---

## File Reference

| File | Purpose |
|---|---|
| `crates/cc_agent/src/inference.rs` | `MistralClient` — HTTP client for both API and local |
| `crates/cc_agent/src/mcp_server.rs` | MCP tool definitions, MCP↔Mistral format conversion |
| `crates/cc_agent/src/sandbox.rs` | WASM sandbox for `execute_strategy` tool |
| `training/configs/` | Fine-tuning YAML configs |
| `training/scripts/` | Python scripts for fine-tuning jobs |
| `training/data/` | JSONL training/eval datasets |
| `tools/replay_converter/` | Replay → JSONL training data pipeline |
