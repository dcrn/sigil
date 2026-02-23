# CDD Agent Instructions

You are connected to a Contract Driven Development (CDD) MCP server. This project uses contracts to declare behaviors that must remain true. Your job is to respect existing contracts, surface conflicts before they ship, and propose new contracts for new behaviors.

## What contracts are

A contract is a YAML file that describes a behavior the system must uphold. Contracts range from highly precise (with schema references, field mappings, and database targets) to intentionally vague ("all API requests must be authenticated via GitHub OAuth"). Read the `description` field -- it is the source of truth for the contract's intent.

Contracts are not specs. Specs tell you what to build. Contracts tell you what must remain true while you build it.

## What you are responsible for

The MCP server handles structure and facts: parsing YAML, resolving file references, scanning test annotations, and reporting what it finds. It never interprets intent.

You handle semantics and decisions. Specifically:

- Reading contract descriptions and constraints to understand intent
- Judging whether a proposed change conflicts with a contract
- Deciding what to do when contracts overlap or conflict
- Writing tests with `fulfills-contract` annotations
- Proposing new contracts for new behaviors
- Flagging violations to the human before proceeding

The server will never tell you "this is a conflict." It will give you the facts and let you decide.

## Workflow

### Before writing code

1. Call `cdd_list_contracts` to see the contract landscape.
2. Identify the files you plan to modify.
3. Call `cdd_get_affected_contracts` with those file paths.
4. For each affected contract, call `cdd_get_contract` with `resolve_refs: true` to get the full context including referenced file contents.
5. Read the `description` and `constraints` of every affected contract. Incorporate them into your plan.
6. If your plan would violate a contract, stop and tell the human. Do not silently proceed.

### While writing code

1. Respect the constraints of all affected contracts.
2. Write tests for new or changed behavior. Annotate them with `fulfills-contract("<contract-id>")` in a comment.
3. When you introduce a new behavior that others could break, propose a new contract via `cdd_create_contract`.
4. When your changes make an existing contract obsolete or inaccurate, update it via `cdd_update_contract` or flag it for the human.

### After writing code

1. Call `cdd_get_contract_health` to check for problems: unlinked contracts, orphaned annotations, broken refs.
2. Fix any problems you introduced.
3. The human reviews contract changes alongside code changes in the normal PR process.

## Contract priorities

- **must** -- violation fails the build. Do not proceed without human approval.
- **should** -- violation produces a warning. Proceed with caution and document why.
- **prefer** -- informational. Use your judgment.

## Test annotations

Link tests to contracts with a comment annotation. The pattern is configured per project, but the default is:

```
fulfills-contract("<contract-id>")
```

One test can fulfill multiple contracts. One contract can be fulfilled by many tests. Reference the contract `id`, not the file path.

## Key principles

- Contracts are first-class artifacts. Treat them with the same seriousness as code and tests.
- When in doubt, surface the conflict. A false alarm costs minutes. A silent violation costs days.
- Keep contracts focused. One behavior per contract. If a contract description requires "and" to explain two unrelated things, it should be two contracts.
- Propose contracts early. If you are building something that has invariants others need to respect, write the contract before or alongside the code.
