# Contract Driven Development (CDD)

CDD is a development methodology and toolset for AI-driven software development. It solves a specific problem: when an AI agent starts working on an existing codebase, it doesn't know what it can't break. Specs tell agents what to build. Contracts tell agents what must remain true.

A **contract** is a YAML file that describes a behavior the system must uphold. Contracts range from highly precise (with schema references, field mappings, and database targets) to intentionally vague ("all API requests must be authenticated via GitHub OAuth"). The agent interprets the contract based on how much detail is provided and fills in the gaps using its understanding of the codebase.

CDD consists of two components:

- **A contract document format** with a fixed, documented YAML structure
- **An MCP server** that exposes contracts to AI agents for discovery, context loading, conflict surfacing, and lifecycle management

There is no CLI. Contracts are authored by humans (in an editor) or by agents (through the MCP server), and are always committed to version control alongside the code they constrain.

## The Border Between AI and Tool

The MCP server handles **structure and facts**. It parses YAML, resolves file references, tracks which contracts touch which files, finds test annotations, and reports what it finds. It never interprets intent or makes judgments.

The agent handles **semantics and decisions**. It reads contract descriptions and constraints to understand intent. It judges whether a proposed change conflicts with a contract. It decides whether orphaned tests should be deleted or preserved. It writes tests and adds annotations.

This boundary is strict. The MCP server will never tell the agent "this is a conflict." It will say "these two contracts both reference the same database table, and here are their constraints." The agent decides what that means.

## Quick Start

1. Create a `cdd.config.yaml` in your project root:

```yaml
contracts_dir: contracts/
test_link_pattern: 'fulfills-contract\("([^"]+)"\)'
test_dirs:
  - tests/
  - src/**/*.test.*
```

2. Create a `contracts/` directory.

3. Write your first contract:

```yaml
# contracts/no-pii-in-logs.contract.yaml
id: no-pii-in-logs
version: "1.0.0"
name: No PII in Logs
priority: must
description: >
  Log output must never contain personally identifiable information.
  This includes email addresses, IP addresses, full names, and any
  field marked as PII in the data model.
domain: compliance
tags: [logging, privacy, pii]
applies_to: "**"
```

4. Connect the MCP server to your AI agent and start working. The agent will discover contracts automatically.

## The Contract Format

Every contract is a YAML file stored in the contracts directory (default: `contracts/`). The filename must match the contract's `id` field, with the suffix `.contract.yaml`.

A JSON Schema for validation is provided at [`schema/contract.schema.json`](schema/contract.schema.json).

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique identifier, kebab-case. Must match the filename. |
| `version` | string | Semver. Used to track contract evolution. |
| `name` | string | Human-readable name. Displayed in listings and summaries. |
| `description` | string | Plain language description of the behavior. The most important field -- this is what the agent reads to understand the contract's intent. |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `priority` | string | One of `must`, `should`, or `prefer`. Defaults to `must`. Controls CI behavior and helps agents weigh contracts against each other. |
| `domain` | string | Freeform grouping label (e.g., "ingestion", "auth", "billing"). |
| `tags` | list of strings | Additional labels for discovery and filtering. |
| `applies_to` | string or list | Glob patterns for files this contract applies to. Use `"**"` for global contracts. |
| `trigger` | object | What initiates this contract's behavior. Freeform, but `type` is used structurally. |
| `refs` | list of strings | File paths this contract is concerned with. |
| `behaviors` | list of objects | Expected behaviors with `id`, `description`, optional `constraints` and `refs`. |
| `depends_on` | list of objects | Other contracts that must be satisfied. Each has `contract` and optional `reason`. |
| `testing` | object | Metadata about how to test. `fixtures` is used for ref resolution; rest is freeform. |
| `notes` | string | Freeform space for context, historical decisions, edge cases. |

### Examples

**Precise contract** -- full Kafka ingestion pipeline with schemas, migrations, and fixture files:

```yaml
id: device-data-ingestion
version: "1.2.0"
name: Device Data Ingestion
description: >
  When a message arrives on the device-data Kafka topic, the service
  must validate the payload against the DeviceMessage Avro schema.
  Valid messages are stored in the device_data table. Invalid messages
  are published to the device-data-dlq topic with the original payload
  preserved and an error reason attached.
domain: ingestion
tags: [kafka, storage, device-data]
trigger:
  type: kafka-message
  topic: device-data
  consumer_group: device-ingestion-service
refs:
  - schemas/DeviceMessage.avsc
  - db/migrations/003_device_data.sql
behaviors:
  - id: store-valid-message
    description: Valid messages are persisted to the device_data table
    refs:
      - db/migrations/003_device_data.sql
    constraints:
      - device_id must not be null
      - received_at must be set from the Kafka message timestamp
      - payload must pass DeviceMessage schema validation before storage
  - id: dead-letter-invalid
    description: >
      Invalid messages are published to the device-data-dlq topic
    constraints:
      - original payload must be preserved byte-for-byte
      - error reason must be included as a message header
      - the dlq message must include the original topic and partition
depends_on:
  - contract: device-registration
    reason: >
      device_id in the message must reference a device that has been
      registered.
testing:
  strategy: integration
  fixtures:
    - fixtures/valid-device-message.json
    - fixtures/invalid-device-message.json
    - fixtures/unknown-device-message.json
```

**Loose contract** -- a cross-cutting concern with no file refs:

```yaml
id: api-authentication
version: "1.0.0"
name: API Authentication
description: >
  All HTTP API endpoints must require authentication via GitHub OAuth.
  Unauthenticated requests must receive a 401 response.
domain: auth
tags: [http, security, github]
trigger:
  type: http
  description: Any inbound API request
behaviors:
  - id: reject-unauthenticated
    description: Requests without a valid GitHub token return 401
    constraints:
      - Response body must include a clear error message
      - The WWW-Authenticate header must be set
  - id: propagate-identity
    description: Authenticated user identity is available to downstream handlers
```

**Minimal contract** -- just a global constraint:

```yaml
id: no-pii-in-logs
version: "1.0.0"
name: No PII in Logs
priority: must
description: >
  Log output must never contain personally identifiable information.
domain: compliance
tags: [logging, privacy, pii]
applies_to: "**"
```

## Test Linking

Tests link to contracts through annotations in comments. The annotation pattern is configurable via `test_link_pattern` in `cdd.config.yaml`. The default pattern is:

```
fulfills-contract("<contract-id>")
```

Examples:

```python
# fulfills-contract("device-data-ingestion")
def test_valid_message_stored():
    ...
```

```typescript
// fulfills-contract("device-data-ingestion")
describe('device message processing', () => { ... });
```

### Rules

- Annotations reference contract ids, not file paths
- One test can fulfill multiple contracts
- One contract can be fulfilled by many tests
- The MCP server finds annotations via regex scan of configured test directories

## The MCP Server

The MCP server is the sole programmatic interface to the contract system. It exposes the following tools:

| Tool | Purpose |
|------|---------|
| `cdd_list_contracts` | List all contracts with summary info. Starting point for planning. |
| `cdd_get_contract` | Retrieve a single contract with full detail. Optionally resolves file refs. |
| `cdd_get_affected_contracts` | Given file paths, return all contracts that care about those files. |
| `cdd_find_related` | Find contracts sharing refs, triggers, or dependencies with a given contract. |
| `cdd_get_linked_tests` | Return all tests linked to a contract via annotations. |
| `cdd_get_contract_health` | Overall health: unlinked contracts, orphaned annotations, broken refs. |
| `cdd_create_contract` | Create a new contract file with validation. |
| `cdd_update_contract` | Update an existing contract. Returns a diff. |
| `cdd_delete_contract` | Delete a contract and report downstream impact. |
| `cdd_check_structural` | Fast structural validation for CI. No AI needed. |
| `cdd_review_changeset` | Bundle affected contracts with resolved context for agent review. |

See the [contracts](contracts/) directory for detailed behavioral contracts for each tool.

## CI/CD Integration

CDD enforcement in CI has two layers:

### Layer 1: Structural Checks (No AI, Fast, Blocking)

Deterministic checks that run on every push. Uses `cdd_check_structural` internally.

- Broken refs (contract references a file that doesn't exist)
- Schema validation errors
- Orphaned annotations (test references a non-existent contract)
- Missing test coverage (contract has zero linked tests)
- Dependency integrity (`depends_on` references non-existent contracts)

### Layer 2: AI Contract Review (Agent-Powered, Deeper)

An AI agent reviews the changeset against affected contracts:

1. Determine which files changed
2. Get affected contracts with resolved refs
3. Agent evaluates each contract against the diff
4. Verdict per contract: `pass`, `fail`, or `needs_human_review`

### Priority and Enforcement

- `must` contracts fail the build when violated
- `should` contracts produce PR warnings but don't block
- `prefer` contracts are informational only

Overrides are available via PR comments: `cdd-override: <contract-id> -- "reason"`.

## Agent Workflow

### Planning

1. Receive a task
2. `cdd_list_contracts` to see the contract landscape
3. Identify files to modify
4. `cdd_get_affected_contracts` with those files
5. `cdd_get_contract` with `resolve_refs: true` for each affected contract
6. Incorporate constraints into the plan
7. Flag violations to the human before proceeding

### Implementation

1. Write code respecting contract constraints
2. Write tests with `fulfills-contract` annotations
3. Propose new contracts for new behaviors via `cdd_create_contract`

### Review

1. `cdd_get_contract_health` to check coverage
2. Human reviews contract changes alongside code in the normal PR process

## Project Structure

```
cdd/
  schema/
    contract.schema.json    # JSON Schema for contract validation
  contracts/                # Contract files (we dogfood CDD here)
    *.contract.yaml
  cdd.config.yaml           # CDD configuration for this project
  README.md
```

## License

MIT
