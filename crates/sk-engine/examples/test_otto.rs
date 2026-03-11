use sk_engine::runtime::ottos_outpost::{execute_ottos_outpost, ExecutionEnv, OttosOutpostRequest};
use std::path::Path;

#[tokio::main]
async fn main() {
    println!("Testing OTTO's Outpost (Zero-Pollution Docker execution) ...");

    // A python script that uses the specific 'requests' dependency
    let req = OttosOutpostRequest {
        language: "python".to_string(),
        execution_env: ExecutionEnv::Native,
        dependencies: vec!["requests".to_string()],
        code: r#"
import requests
print("Fetching example.com using requests module from within the Outpost...")
r = requests.get('http://example.com')
print(f"Status Code: {r.status_code}")
"#
        .to_string(),
        input_files: vec![],
    };

    let workspace = Path::new(".").to_path_buf();

    match execute_ottos_outpost(req, &workspace).await {
        Ok(res) => {
            println!("Outpost Execution Complete!");
            println!("Exit Code: {}", res.exit_code);
            println!("STDOUT:\n{}", res.stdout);
            println!("STDERR:\n{}", res.stderr);
            if res.exit_code == 0 {
                println!("SUCCESS! Python dynamic execution worked.");
            } else {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Outpost Execution Error: {}", e);
            std::process::exit(1);
        }
    }
}
