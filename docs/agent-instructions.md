# Sigil Agent Instructions

You are connected to the Sigil MCP server. This project uses contracts to declare rules that must remain true. Your job is to respect existing contracts, surface conflicts before they ship, and propose new contracts for new rules.

## What contracts are

A contract is a TOML file that describes a rule the system must uphold. Contracts range from highly precise (with schema references, field mappings, and database targets) to intentionally vague ("all API requests must be authenticated via GitHub OAuth"). Read the `description` field -- it is the source of truth for the contract's intent.

Contracts are not specs. Specs tell you what to build. Contracts tell you what must remain true while you build it.

## What you are responsible for

The MCP server handles structure and facts: parsing TOML, resolving file references, and reporting what it finds. It never interprets intent.

You handle semantics and decisions. Specifically:

- Reading contract descriptions and constraints to understand intent
- Judging whether a proposed change conflicts with a contract
- Deciding what to do when contracts overlap or conflict
- Writing tests that confirm contracts are being fulfilled by the application
- Proposing new contracts for new rules
- Flagging violations to the human before proceeding

The server will never tell you "this is a conflict." It will give you the facts and let you decide.

## Workflow

### Before writing code

1. Call `sigil_list_contracts` to see the contract landscape. **You must call `sigil_list_contracts` or `sigil_get_affected_contracts` before calling `sigil_get_contract` -- the server enforces this and will reject calls that use an id you haven't discovered through a listing call.**
2. Identify the files you plan to modify.
3. Call `sigil_get_affected_contracts` with those file paths.
4. For each affected contract, call `sigil_get_contract` with `retrieve_file_contents: true` to get the full context including referenced file contents.
5. Read the `description` and `constraints` of every affected contract. Incorporate them into your plan.
6. If your plan would violate a contract, stop and tell the human. Do not silently proceed.

### While writing code

1. Respect the constraints of all affected contracts.
2. Write tests for new or changed rules.
3. When you introduce a new rule that others could break, propose a new contract via `sigil_create_contract`.
4. When your changes make an existing contract obsolete or inaccurate, update it via `sigil_update_contract` or flag it for the human. You must call `sigil_get_contract` for that contract before calling `sigil_update_contract` or `sigil_delete_contract` -- the server enforces this and will reject the call otherwise.

### After writing code

1. Call `sigil_validate_all_contracts` to check for problems: missing files, schema errors, duplicate rule ids.
2. Fix any problems you introduced.
3. The human reviews contract changes alongside code changes in the normal PR process.

## Contract priorities

- **must** -- violation fails the build. Do not proceed without human approval.
- **should** -- violation produces a warning. Proceed with caution and document why.
- **prefer** -- informational. Use your judgment.

## Key principles

- Contracts are first-class artifacts. Treat them with the same seriousness as code and tests.
- When in doubt, surface the conflict. A false alarm costs minutes. A silent violation costs days.
- Keep contracts focused. One rule per contract. If a contract description requires "and" to explain two unrelated things, it should be two contracts.
- Propose contracts early. If you are building something that has invariants others need to respect, write the contract before or alongside the code.
