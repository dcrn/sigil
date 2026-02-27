# Sigil

Sigil is a development methodology and toolset for AI-driven software development. It solves a specific problem: when an AI agent starts working on an existing codebase, it doesn't know what it can't break. Specs tell agents what to build. Contracts tell agents what must remain true.

A **contract** is a TOML file that describes a rule the system must uphold. Contracts range from highly precise (with file references, constraints, and domain context) to intentionally vague ("all API requests must be authenticated via GitHub OAuth"). The agent interprets the contract based on how much detail is provided and fills in the gaps using its understanding of the codebase.

Sigil consists of two components:

- **A contract document format** with a fixed, documented TOML structure
- **An MCP server** that exposes contracts to AI agents for discovery, context loading, conflict surfacing, and lifecycle management

There is no CLI. Contracts are authored by humans (in an editor) or by agents (through the MCP server), and are always committed to version control alongside the code they constrain.

## The Border Between AI and Tool

The MCP server handles **structure and facts**. It parses TOML, resolves file references, tracks which contracts touch which files, and reports what it finds. It never interprets intent or makes judgments.

The agent handles **semantics and decisions**. It reads contract descriptions and constraints to understand intent. It judges whether a proposed change conflicts with a contract. It writes tests that fulfill the contracts.

This boundary is strict. The MCP server will never tell the agent "this is a conflict." It will say "these two contracts both reference the same database table, and here are their constraints." The agent decides what that means.

## Quick Start

1. Create a `sigil.config.toml` in your project root:

```toml
contracts_dir = "contracts/"
```

2. Create a `contracts/` directory.

3. Write your first contract:

```toml
# contracts/no-pii-in-logs.contract.toml
id = "no-pii-in-logs"
version = "1.0.0"
name = "No PII in Logs"
priority = "must"
description = """
Log output must never contain personally identifiable information.
This includes email addresses, IP addresses, full names, and any
field marked as PII in the data model.
"""
domain = "compliance"
tags = ["logging", "privacy", "pii"]
applies_to = "**"
```

4. Connect the MCP server to your AI agent and start working. The agent will discover contracts automatically.

## The Contract Format

Every contract is a TOML file stored in the contracts directory (default: `contracts/`). The filename must match the contract's `id` field, with the suffix `.contract.toml`.

A JSON Schema for validation is provided at [`schema/contract.schema.json`](schema/contract.schema.json).

### Top-level Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Kebab-case, matches filename. |
| `version` | string | yes | Semver. Used to track contract evolution. |
| `name` | string | yes | Human-readable name. Displayed in listings and summaries. |
| `description` | string | yes | What this contract upholds -- source of truth for intent. |
| `priority` | string | no | `must` (default), `should`, or `prefer`. Controls CI behavior. |
| `status` | string | no | `active` (default), `draft`, or `deprecated`. |
| `domain` | string | no | Freeform grouping label (e.g., "ingestion", "auth"). |
| `tags` | string[] | no | Labels for discovery and filtering. |
| `applies_to` | string or string[] | no | Glob patterns for auto-matching files. |
| `files` | string[] | no | File paths the whole contract cares about. |
| `notes` | string | no | Freeform context, guidance, historical decisions. |

### `[trigger]`

Freeform table. `type` is the only conventionally used key. Everything else is domain context (topic, tool, consumer_group, etc.).

### `[[rules]]`

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Unique within the contract. |
| `description` | string | yes | What should happen. |
| `files` | string[] | no | Files specific to this rule. |
| `constraints` | string[] | no | Prose invariants for agents to interpret. |

### `[[changelog]]`

Tool-managed via `sigil_update_contract`.

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | string | yes | Contract version this entry describes. |
| `date` | string | no | ISO 8601 date. |
| `description` | string | yes | What changed and why. |

### Examples

**Precise contract** -- full Kafka ingestion pipeline with file references:

```toml
id = "device-data-ingestion"
version = "1.3.0"
name = "Device Data Ingestion"
description = """
When a message arrives on the device-data Kafka topic, the service
must validate the payload against the DeviceMessage Avro schema.
"""
domain = "ingestion"
tags = ["kafka", "storage", "device-data"]
files = [
    "src/ingestion/handler.ts",
    "config/kafka-consumers.yaml",
    "schemas/DeviceMessage.avsc",
]

[trigger]
type = "kafka-message"
topic = "device-data"
consumer_group = "device-ingestion-service"

[[rules]]
id = "store-valid-message"
description = "Valid messages are persisted to the device_data table."
files = ["db/migrations/003_device_data.sql"]
constraints = [
    "device_id must not be null",
    "received_at must be set from the Kafka message timestamp",
    "payload must pass DeviceMessage schema validation before storage",
    "duplicate messages (same device_id + timestamp) must be idempotent",
]

[[rules]]
id = "dead-letter-invalid"
description = "Invalid messages are published to the device-data-dlq topic."
files = ["schemas/DeadLetterEnvelope.avsc"]
constraints = [
    "original payload must be preserved byte-for-byte",
    "error reason must be included as a message header",
    "the dlq message must include the original topic, partition, and offset",
]
```

**Loose contract** -- a cross-cutting concern with no file refs:

```toml
id = "api-authentication"
version = "1.0.0"
name = "API Authentication"
description = """
All HTTP API endpoints must require authentication via GitHub OAuth.
Unauthenticated requests must receive a 401 response.
"""
domain = "auth"
tags = ["http", "security", "github"]

[trigger]
type = "http"
description = "Any inbound API request"

[[rules]]
id = "reject-unauthenticated"
description = "Requests without a valid GitHub token return 401."
constraints = [
    "Response body must include a clear error message",
    "The WWW-Authenticate header must be set",
]

[[rules]]
id = "propagate-identity"
description = "Authenticated user identity is available to downstream handlers."
```

**Minimal contract** -- just a global constraint:

```toml
id = "no-pii-in-logs"
version = "1.0.0"
name = "No PII in Logs"
priority = "must"
description = """
Log output must never contain personally identifiable information.
"""
domain = "compliance"
tags = ["logging", "privacy", "pii"]
applies_to = "**"
```

## The MCP Server

The MCP server is the sole programmatic interface to the contract system. It exposes the following tools:

| Tool | Purpose |
|------|---------|
| `sigil_list_contracts` | List all contracts with summary info. Starting point for planning. |
| `sigil_get_contract` | Retrieve a single contract with full detail. Optionally resolves file refs. |
| `sigil_get_affected_contracts` | Given file paths, return all contracts that care about those files. |
| `sigil_create_contract` | Create a new contract file with validation. |
| `sigil_update_contract` | Update an existing contract. Returns a diff. Supports `changelog_message`. |
| `sigil_delete_contract` | Delete a contract file. |
| `sigil_validate_contract` | Validate a single contract: schema compliance, missing files, structural correctness. |
| `sigil_validate_all_contracts` | Validate all contracts. Designed for CI pipelines. |
| `sigil_review_changeset` | Bundle affected contracts with full context (content and file contents) for agent review. |

See the [contracts](contracts/) directory for detailed behavioral contracts for each tool.

## CI/CD Integration

Sigil enforcement in CI has two layers:

### Layer 1: Structural Checks (No AI, Fast, Blocking)

Deterministic checks that run on every push. Uses `sigil_validate_all_contracts` internally.

- Broken refs (contract references a file that doesn't exist)
- Schema validation errors
- Duplicate rule ids within a contract
- Filename-id consistency

### Layer 2: AI Contract Review (Agent-Powered, Deeper)

An AI agent reviews the changeset against affected contracts:

1. Determine which files changed
2. Get affected contracts with resolved refs via `sigil_review_changeset`
3. Agent evaluates each contract against the diff
4. Verdict per contract: `pass`, `fail`, or `needs_human_review`

### Priority and Enforcement

- `must` contracts fail the build when violated
- `should` contracts produce PR warnings but don't block
- `prefer` contracts are informational only

Overrides are available via PR comments: `sigil-override: <contract-id> -- "reason"`.

## Agent Workflow

### Planning

1. Receive a task
2. `sigil_list_contracts` to see the contract landscape
3. Identify files to modify
4. `sigil_get_affected_contracts` with those files
5. `sigil_get_contract` with `retrieve_file_contents: true` for each affected contract
6. Incorporate constraints into the plan
7. Flag violations to the human before proceeding

### Implementation

1. Write code respecting contract constraints
2. Propose new contracts for new rules via `sigil_create_contract`

### Review

1. `sigil_validate_all_contracts` to check for missing files, schema errors, and structural issues
2. Human reviews contract changes alongside code in the normal PR process

## Project Structure

```
sigil/
  schema/
    contract.schema.json    # JSON Schema for contract validation
  contracts/                # Contract files (dogfooded)
    *.contract.toml
  sigil.config.toml           # Sigil configuration for this project
  README.md
```

## License

MIT
