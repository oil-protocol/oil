use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use regex::{Regex, escape as regex_escape};
use serde_json::{json, Value};
use indexmap::IndexMap;

const PROGRAM_ID: &str = "rigwXYKkE8rXiiyu6eFs3ZuDNH2eYHb1y87tYqwDJhk";

fn main() -> anyhow::Result<()> {
    println!("Generating IDL from program source...");
    
    // Find workspace root (look for Cargo.toml with [workspace])
    let workspace_root = find_workspace_root()?;
    println!("Workspace root: {}", workspace_root.display());
    
    let api_src = workspace_root.join("api/src/instruction.rs");
    let program_src_dir = workspace_root.join("program/src");
    let output_path = workspace_root.join("api/idl.json");
    
    // Read instruction definitions
    let instruction_rs = fs::read_to_string(&api_src)?;
    
    // Parse instruction enum to get discriminators (preserve order)
    let instructions = parse_instruction_enum(&instruction_rs)?;
    
    // Parse instruction structs to get args
    let args_map = parse_instruction_args(&instruction_rs)?;
    
    // Parse account lists from instruction handlers
    let accounts_map = parse_account_lists(&program_src_dir)?;
    
    // Build IDL (preserve enum definition order)
    let mut idl_instructions = Vec::new();
    
    for (name, discriminator) in &instructions {
        let instruction_name = to_camel_case(name);
        let args = args_map.get(name).cloned().unwrap_or_default();
        let accounts = accounts_map.get(name).cloned().unwrap_or_default();
        let docs = get_docs_for_instruction(name)?;
        
        idl_instructions.push(json!({
            "name": instruction_name,
            "discriminant": {
                "type": "u8",
                "value": discriminator
            },
            "docs": docs,
            "accounts": accounts,
            "args": args
        }));
    }
    
    // Instructions are already in enum definition order, no sorting needed
    
    // Build full IDL in correct order: version, name, instructions, accounts, types, events, errors, metadata
    // Use IndexMap to preserve insertion order
    let accounts = get_account_definitions();
    let types = get_type_definitions();
    let events = get_event_definitions();
    let errors = get_error_definitions();
    
    let mut idl_map: IndexMap<&str, Value> = IndexMap::new();
    idl_map.insert("version", json!("0.0.1"));
    idl_map.insert("name", json!("oil"));
    idl_map.insert("instructions", json!(idl_instructions));
    idl_map.insert("accounts", json!(accounts));
    idl_map.insert("types", json!(types));
    idl_map.insert("events", json!(events));
    idl_map.insert("errors", json!(errors));
    idl_map.insert("metadata", json!({
        "address": PROGRAM_ID,
        "origin": "steel"
    }));
    
    // Serialize IndexMap (preserves order with serde feature)
    let idl_json = serde_json::to_string_pretty(&idl_map)?;
    // fs::write truncates the file if it exists, so this always overwrites
    fs::write(&output_path, idl_json)?;
    
    println!("✓ IDL generated successfully at {}", output_path.display());
    println!("  Generated {} instructions", idl_instructions.len());
    println!("  Generated {} accounts", accounts.len());
    println!("  Generated {} types", types.len());
    println!("  Generated {} events", events.len());
    println!("  Generated {} errors", errors.len());
    
    Ok(())
}

fn parse_instruction_enum(content: &str) -> anyhow::Result<Vec<(String, u8)>> {
    let mut instructions = Vec::new();
    
    // Pattern: InstructionName = value,
    // Match in order to preserve enum definition order
    let re = Regex::new(r"(\w+)\s*=\s*(\d+)")?;
    
    for cap in re.captures_iter(content) {
        let name = cap.get(1).unwrap().as_str().to_string();
        let value: u8 = cap.get(2).unwrap().as_str().parse()?;
        instructions.push((name, value));
    }
    
    Ok(instructions)
}

fn parse_instruction_args(content: &str) -> anyhow::Result<HashMap<String, Vec<Value>>> {
    let mut args_map = HashMap::new();
    
    // Pattern: pub struct InstructionName { ... }
    let struct_re = Regex::new(r"#\[repr\(C\)\]\s*#\[derive\([^)]+\)\]\s*pub struct (\w+)\s*\{([^}]+)\}")?;
    let field_re = Regex::new(r"pub\s+(\w+):\s*([^,]+)")?;
    
    for cap in struct_re.captures_iter(content) {
        let struct_name = cap.get(1).unwrap().as_str();
        let fields = cap.get(2).unwrap().as_str();
        
        // Skip empty structs
        if fields.trim().is_empty() {
            args_map.insert(struct_name.to_string(), vec![]);
            continue;
        }
        
        let mut args = Vec::new();
        for field_cap in field_re.captures_iter(fields) {
            let field_name = field_cap.get(1).unwrap().as_str();
            let field_type = field_cap.get(2).unwrap().as_str().trim();
            
            // Convert Rust type to IDL type
            let idl_type = rust_to_idl_type(field_type);
            
            args.push(json!({
                "name": field_name,
                "type": idl_type
            }));
        }
        
        args_map.insert(struct_name.to_string(), args);
    }
    
    Ok(args_map)
}

fn rust_to_idl_type(rust_type: &str) -> Value {
    let rust_type = rust_type.trim();
    
    // Handle arrays like [u8; 8] or [u8; 32]
    if let Some(cap) = Regex::new(r"\[u8;\s*(\d+)\]").unwrap().captures(rust_type) {
        let size: usize = cap.get(1).unwrap().as_str().parse().unwrap();
        if size == 8 {
            return json!("u64");
        } else if size == 32 {
            return json!("publicKey");
        } else if size == 4 {
            return json!("u32");
        }
    }
    
    // Handle nested arrays like [[u8; 8]; 4]
    if rust_type.contains("[[u8; 8];") {
        return json!({
            "array": ["u64", 4]
        });
    }
    
    match rust_type {
        "u8" => json!("u8"),
        "u64" => json!("u64"),
        "i64" => json!("i64"),
        _ => json!("u64"), // Default fallback
    }
}

fn find_workspace_root() -> anyhow::Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    
    loop {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }
        
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => anyhow::bail!("Could not find workspace root (Cargo.toml with [workspace])"),
        }
    }
}

fn parse_account_lists(program_dir: &Path) -> anyhow::Result<HashMap<String, Vec<Value>>> {
    let mut accounts_map = HashMap::new();
    
    // Read all instruction handler files
    for entry in fs::read_dir(&program_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)?;
            
            // Extract instruction name from filename (e.g., contribute.rs -> Contribute)
            let file_stem = path.file_stem().unwrap().to_str().unwrap();
            
            // Handle _with_session variants
            if file_stem.ends_with("_with_session") {
                let base_name = file_stem.strip_suffix("_with_session").unwrap();
                let instruction_name = format!("{}WithSession", to_pascal_case(base_name));
                
                if let Some(accounts) = extract_accounts_from_comment(&content) {
                    accounts_map.insert(instruction_name, accounts);
                }
            } else {
                let instruction_name = to_pascal_case(file_stem);
                
                // Look for account order comments
                if let Some(accounts) = extract_accounts_from_comment(&content) {
                    accounts_map.insert(instruction_name, accounts);
                }
            }
        }
    }
    
    Ok(accounts_map)
}

fn extract_accounts_from_comment(content: &str) -> Option<Vec<Value>> {
    // Look for comment pattern: // Account order: account1, account2, ...
    // Stop at the next non-comment line or end of comment block
    let re = Regex::new(r"(?m)^\s*//\s*Account order:\s*([^\n]+)").ok()?;
    
    if let Some(cap) = re.captures(content) {
        let account_list = cap.get(1).unwrap().as_str();
        
        // Split by comma only (not newlines, as the comment is on one line)
        let accounts: Vec<&str> = account_list
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        
        let mut idl_accounts = Vec::new();
        for account in accounts {
            // Clean up account name - remove any trailing comments or extra whitespace
            let account = account.split("//").next().unwrap().trim();
            
            // Skip if it looks like code (contains operators, keywords, etc.)
            if account.contains('=') || account.contains('(') || account.contains(')') || 
               account.contains('{') || account.contains('}') || account.contains(';') ||
               account == "let" || account == "if" || account == "return" {
                continue;
            }
            
            if account.is_empty() {
                continue;
            }
            
            // Convert snake_case to camelCase for IDL
            let account_name = to_camel_case(account);
            
            // Determine if account is signer, writable, etc.
            let (is_mut, is_signer, address) = determine_account_properties(&account_name);
            
            let mut account_json = json!({
                "name": account_name,
                "isMut": is_mut,
                "isSigner": is_signer
            });
            
            if let Some(addr) = address {
                account_json["address"] = json!(addr);
            }
            
            idl_accounts.push(account_json);
        }
        
        return Some(idl_accounts);
    }
    
    None
}

fn determine_account_properties(account: &str) -> (bool, bool, Option<&str>) {
    let account_lower = account.to_lowercase();
    
    // Common patterns
    match account_lower.as_str() {
        "signer" | "authority" | "payer" => (true, true, None),
        "systemprogram" | "system_program" => (false, false, Some("11111111111111111111111111111111")),
        "tokenprogram" | "token_program" => (false, false, Some("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")),
        "associatedtokenprogram" | "associated_token_program" | "ataprogram" | "ata_program" => (false, false, Some("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")),
        "mint" => (false, false, Some("oiLTuhTJc9qRDr2FcMiCUBJ3BCunNXP1LGJCG7svBSy")),
        "programsigner" | "program_signer" => (false, true, None),
        _ => {
            // Default: writable if not a program, not a signer unless explicitly named
            let is_mut = !account_lower.contains("program") && !account_lower.contains("mint") && account_lower != "executor";
            let is_signer = account_lower == "signer" || account_lower == "authority" || account_lower == "payer";
            (is_mut, is_signer, None)
        }
    }
}

fn to_camel_case(s: &str) -> String {
    // Handle PascalCase input (e.g., "AutomateWithSession" -> "automateWithSession")
    // or snake_case input (e.g., "automate_with_session" -> "automateWithSession")
    let mut result = String::new();
    let mut capitalize_next = false;
    let mut first_char = true;
    let mut prev_was_lowercase = false;
    
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if c.is_uppercase() {
            if first_char {
                result.push(c.to_ascii_lowercase());
                first_char = false;
                prev_was_lowercase = true;
            } else {
                // If previous char was lowercase, this uppercase starts a new word - keep it uppercase
                if capitalize_next || prev_was_lowercase {
                    result.push(c);
                    capitalize_next = false;
                    prev_was_lowercase = false;
                } else {
                    result.push(c.to_ascii_lowercase());
                    prev_was_lowercase = false;
                }
            }
        } else {
            if capitalize_next {
                result.push(c.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(c);
            }
            first_char = false;
            prev_was_lowercase = true;
        }
    }
    
    result
}

fn to_pascal_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c.to_ascii_lowercase());
        }
    }
    
    result
}

fn get_docs_for_instruction(instruction_name: &str) -> anyhow::Result<Vec<String>> {
    // Find workspace root
    let workspace_root = find_workspace_root()?;
    let program_src_dir = workspace_root.join("program/src");
    
    // Try to extract docs from instruction handler
    let base_name = if instruction_name.ends_with("WithSession") {
        instruction_name.strip_suffix("WithSession").unwrap()
    } else {
        instruction_name
    };
    
    let handler_file = program_src_dir.join(format!("{}.rs", to_snake_case(base_name)));
    let alt_handler_file = if !instruction_name.ends_with("WithSession") {
        program_src_dir.join(format!("{}_with_session.rs", to_snake_case(base_name)))
    } else {
        PathBuf::new()
    };
    
    let content = if handler_file.exists() {
        fs::read_to_string(&handler_file)?
    } else if alt_handler_file.exists() {
        fs::read_to_string(&alt_handler_file)?
    } else {
        return Ok(vec![]);
    };
    
    // Extract doc comments (look for function-level docs before process_ function)
    let re = Regex::new(r"///\s*(.+)")?;
    let mut docs = Vec::new();
    
    for cap in re.captures_iter(&content) {
        if let Some(doc) = cap.get(1) {
            let doc_text = doc.as_str().trim();
            // Skip empty docs or very short ones
            if doc_text.len() > 3 {
                docs.push(doc_text.to_string());
            }
        }
    }
    
    // Limit to first few meaningful docs
    Ok(docs.into_iter().take(5).collect())
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

fn get_account_definitions() -> Vec<Value> {
    // Parse account definitions from state files
    if let Ok(workspace_root) = find_workspace_root() {
        let state_dir = workspace_root.join("api/src/state");
        if let Ok(state_mod) = fs::read_to_string(state_dir.join("mod.rs")) {
            // Extract account enum discriminators
            if let Ok(account_enum_re) = Regex::new(r"(\w+)\s*=\s*(\d+)") {
                let mut accounts = Vec::new();
                
                for cap in account_enum_re.captures_iter(&state_mod) {
                    let name = cap.get(1).unwrap().as_str();
                    if let Ok(discriminator_value) = cap.get(2).unwrap().as_str().parse::<u8>() {
                        // Read the account struct file
                        let account_file = state_dir.join(format!("{}.rs", name.to_lowercase()));
                        if let Ok(content) = fs::read_to_string(&account_file) {
                            if let Some(account_def) = parse_account_struct(&content, name, discriminator_value) {
                                accounts.push(account_def);
                            }
                        }
                    }
                }
                
                return accounts;
            }
        }
    }
    
    vec![]
}

fn parse_account_struct(content: &str, name: &str, discriminator: u8) -> Option<Value> {
    // Find the struct definition
    let struct_re = Regex::new(r"(?s)#\[repr\(C\)\].*?pub struct (\w+)\s*\{([^}]+)\}").ok()?;
    let field_re = Regex::new(r"pub\s+(\w+):\s*([^,]+)").ok()?;
    
    if let Some(cap) = struct_re.captures(content) {
        let _struct_name = cap.get(1).unwrap().as_str();
        let fields_str = cap.get(2).unwrap().as_str();
        
        // Extract doc comments
        let doc_re = Regex::new(r"///\s*(.+)").ok()?;
        let mut docs = Vec::new();
        for doc_cap in doc_re.captures_iter(content) {
            if let Some(doc) = doc_cap.get(1) {
                docs.push(doc.as_str().trim().to_string());
            }
        }
        
        // Parse fields
        let mut fields = Vec::new();
        for field_cap in field_re.captures_iter(fields_str) {
            let field_name = field_cap.get(1).unwrap().as_str();
            let field_type = field_cap.get(2).unwrap().as_str().trim();
            
            fields.push(json!({
                "name": field_name,
                "type": rust_to_idl_type(field_type)
            }));
        }
        
        // Build discriminator array (8 bytes, little-endian)
        let discriminator_bytes = vec![discriminator, 0, 0, 0, 0, 0, 0, 0];
        
        return Some(json!({
            "name": name,
            "discriminator": discriminator_bytes,
            "docs": docs,
            "type": {
                "kind": "struct",
                "fields": fields
            }
        }));
    }
    
    None
}

fn get_type_definitions() -> Vec<Value> {
    // Parse type definitions (like Numeric)
    let _workspace_root = match find_workspace_root() {
        Ok(root) => root,
        Err(_) => return vec![],
    };
    
    // Look for Numeric type definition
    // Numeric is defined in steel crate, so we add it manually
    let mut types = Vec::new();
    
    // Numeric is typically defined in steel or as a type alias
    // For now, add it manually based on common patterns
    types.push(json!({
        "name": "Numeric",
        "docs": ["Fixed-point helper backed by I80F48 from the steel crate."],
        "type": {
            "kind": "struct",
            "fields": [
                {
                    "name": "bits",
                    "type": {
                        "array": ["u8", 16]
                    }
                }
            ]
        }
    }));
    
    types
}

fn get_event_definitions() -> Vec<Value> {
    // Parse event definitions from event.rs
    let workspace_root = match find_workspace_root() {
        Ok(root) => root,
        Err(_) => return vec![],
    };
    let event_file = workspace_root.join("api/src/event.rs");
    
    if let Ok(content) = fs::read_to_string(&event_file) {
        parse_event_definitions(&content)
    } else {
        vec![]
    }
}

fn parse_event_definitions(content: &str) -> Vec<Value> {
    let mut events = Vec::new();
    
    // Find event enum
    if let Ok(event_enum_re) = Regex::new(r"pub enum OilEvent\s*\{([^}]+)\}") {
        if let Some(cap) = event_enum_re.captures(content) {
            let enum_body = cap.get(1).unwrap().as_str();
            if let Ok(variant_re) = Regex::new(r"(\w+)\s*=\s*(\d+)") {
                for variant_cap in variant_re.captures_iter(enum_body) {
                    let name = variant_cap.get(1).unwrap().as_str();
                    if let Ok(discriminator_value) = variant_cap.get(2).unwrap().as_str().parse::<u64>() {
                        // Find the corresponding event struct
                        let struct_name = format!("{}Event", name);
                        // Escape the struct name for regex
                        let escaped_name = regex_escape(&struct_name);
                        // Build regex pattern - use string replacement to avoid format string issues
                        let base_pattern = r"(?s)#\[repr\(C\)\].*?pub struct NAME\s*\{([^}]+)\}";
                        let struct_pattern = base_pattern.replace("NAME", &escaped_name);
                        if let Ok(struct_re) = Regex::new(&struct_pattern) {
                            if let Some(struct_cap) = struct_re.captures(content) {
                                let fields_str = struct_cap.get(1).unwrap().as_str();
                                if let Ok(field_re) = Regex::new(r"pub\s+(\w+):\s*([^,]+)") {
                                    let mut fields = Vec::new();
                                    for field_cap in field_re.captures_iter(fields_str) {
                                        let field_name = field_cap.get(1).unwrap().as_str();
                                        let field_type = field_cap.get(2).unwrap().as_str().trim();
                                        
                                        fields.push(json!({
                                            "name": field_name,
                                            "type": rust_to_idl_type(field_type),
                                            "index": false
                                        }));
                                    }
                                    
                                    // Build discriminator array (8 bytes, little-endian)
                                    let disc_bytes = discriminator_value.to_le_bytes();
                                    let discriminator_bytes: Vec<u8> = disc_bytes.to_vec();
                                    
                                    events.push(json!({
                                        "name": struct_name,
                                        "discriminator": discriminator_bytes,
                                        "fields": fields
                                    }));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    events
}

fn get_error_definitions() -> Vec<Value> {
    // Parse error definitions from error.rs
    let workspace_root = match find_workspace_root() {
        Ok(root) => root,
        Err(_) => return vec![
            json!({
                "code": 0,
                "name": "AmountTooSmall",
                "msg": "Amount too small"
            }),
            json!({
                "code": 1,
                "name": "NotAuthorized",
                "msg": "Not authorized"
            })
        ],
    };
    let error_file = workspace_root.join("api/src/error.rs");
    
    if let Ok(content) = fs::read_to_string(&error_file) {
        parse_error_definitions(&content)
    } else {
        vec![
            json!({
                "code": 0,
                "name": "AmountTooSmall",
                "msg": "Amount too small"
            }),
            json!({
                "code": 1,
                "name": "NotAuthorized",
                "msg": "Not authorized"
            })
        ]
    }
}

fn parse_error_definitions(content: &str) -> Vec<Value> {
    let mut errors = Vec::new();
    
    // Find error enum
    if let Ok(error_enum_re) = Regex::new(r"pub enum OilError\s*\{([^}]+)\}") {
        if let Some(cap) = error_enum_re.captures(content) {
            let enum_body = cap.get(1).unwrap().as_str();
            
            // Match each variant: #[error("msg")] Name = value,
            if let Ok(variant_re) = Regex::new(r"(?s)#\[error\([^)]+\)\].*?(\w+)\s*=\s*(\d+)") {
                if let Ok(msg_re) = Regex::new(r#"#\[error\("([^"]+)"\)\]"#) {
                    for variant_cap in variant_re.captures_iter(enum_body) {
                        let name = variant_cap.get(1).unwrap().as_str();
                        if let Ok(code) = variant_cap.get(2).unwrap().as_str().parse::<u32>() {
                            // Extract error message - find the message for this specific variant
                            let variant_start = variant_cap.get(0).unwrap().start();
                            let variant_text = &enum_body[variant_start..];
                            let msg = if let Some(msg_cap) = msg_re.captures(variant_text) {
                                msg_cap.get(1).unwrap().as_str().to_string()
                            } else {
                                format!("{}", name)
                            };
                            
                            errors.push(json!({
                                "code": code,
                                "name": name,
                                "msg": msg
                            }));
                        }
                    }
                }
            }
        }
    }
    
    if errors.is_empty() {
        // Fallback to hardcoded errors
        vec![
            json!({
                "code": 0,
                "name": "AmountTooSmall",
                "msg": "Amount too small"
            }),
            json!({
                "code": 1,
                "name": "NotAuthorized",
                "msg": "Not authorized"
            })
        ]
    } else {
        errors
    }
}
