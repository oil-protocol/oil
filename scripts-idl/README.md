# IDL Generator

This script generates the IDL (Interface Definition Language) JSON file for the Oil program by parsing the Rust source code.

## Usage

```bash
cd scripts-idl
cargo run --bin generate-idl
```

This will:
1. Parse instruction definitions from `api/src/instruction.rs`
2. Extract instruction discriminators from the `OilInstruction` enum
3. Extract instruction arguments from struct definitions
4. Extract account lists from comments in instruction handlers
5. Generate `api/idl.json`

## How It Works

The script:
- **Instructions**: Reads the `OilInstruction` enum to get all instruction names and discriminators
- **Args**: Parses instruction struct definitions (e.g., `Automate`, `Deploy`, `Contribute`) to extract argument types
- **Accounts**: Extracts account lists from `// Account order:` comments in instruction handler files
- **Docs**: Extracts documentation from `///` comments in instruction handlers

## Adding New Instructions

When adding a new instruction:

1. Add the instruction to `OilInstruction` enum in `api/src/instruction.rs`
2. Create the instruction struct with args in `api/src/instruction.rs`
3. Create the instruction handler in `program/src/`
4. Add a comment with account order: `// Account order: account1, account2, ...`
5. Run the script to regenerate the IDL

## Limitations

- Account properties (isMut, isSigner) are inferred from naming conventions
- Some account addresses are hardcoded (system program, token program, etc.)
- Complex account structures may need manual adjustment in the generated IDL
