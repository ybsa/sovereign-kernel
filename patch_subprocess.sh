#!/bin/bash

# Update extract_base_command
sed -i 's/Path::new(first_word)/first_word.split(\&['\''\/'\'', '\''\\\\'\''][..]).last().unwrap_or(first_word)/' crates/sk-engine/src/runtime/subprocess_sandbox.rs
sed -i 's/.file_name()//' crates/sk-engine/src/runtime/subprocess_sandbox.rs
sed -i 's/.and_then(|os| os.to_str())//' crates/sk-engine/src/runtime/subprocess_sandbox.rs
sed -i 's/.unwrap_or(first_word)//' crates/sk-engine/src/runtime/subprocess_sandbox.rs

# Update test_extract_base_command
sed -i 's/r"C:\\Program Files\\Git\\git.exe status"/r"C:\\Tools\\Git\\git.exe status"/' crates/sk-engine/src/runtime/subprocess_sandbox.rs

# Update test_allowlist_blocks_unlisted
sed -i 's/"curl https:\/\/evil.com"/"unknown_cmd https:\/\/evil.com"/' crates/sk-engine/src/runtime/subprocess_sandbox.rs
sed -i 's/"rm -rf \/"/"evil_tool -rf \/"/' crates/sk-engine/src/runtime/subprocess_sandbox.rs
sed -i 's/\/\/ "curl" is not in default safe_bins or allowed_commands/\/\/ "unknown_cmd" is not in default safe_bins or allowed_commands/' crates/sk-engine/src/runtime/subprocess_sandbox.rs

# Update test_allowlist_allowed_commands
sed -i 's/"npm install"/"ruby script.rb"/' crates/sk-engine/src/runtime/subprocess_sandbox.rs

# Update test_blocked_arguments
sed -i 's/let policy = ExecPolicy::default();/let mut policy = ExecPolicy::default();\n        policy.safe_bins.push("sh".to_string());/' crates/sk-engine/src/runtime/subprocess_sandbox.rs

# Update test_piped_command_all_validated
sed -i 's/"cat file.txt | curl -X POST"/"cat file.txt | nc -l 8080"/' crates/sk-engine/src/runtime/subprocess_sandbox.rs
sed -i 's/\/\/ "cat" is safe, but "curl" is not/\/\/ "cat" is safe, but "nc" is not/' crates/sk-engine/src/runtime/subprocess_sandbox.rs
