//! Web search tool.
use sk_types::ToolDefinition;
pub fn web_search_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_search".into(),
        description: "Search the web for information.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),
    }
}

pub async fn handle_web_search(query: &str) -> Result<String, sk_types::SovereignError> {
    let mut query_encoded = String::new();
    url::form_urlencoded::Serializer::new(&mut query_encoded).append_pair("q", query);
    // query_encoded is now "q=..."
    let url = format!("https://html.duckduckgo.com/html/?{}", query_encoded);

    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(format!("Failed to build client: {}", e)))?;

    let response = client.get(&url).send().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Search request failed: {}", e))
    })?;

    let html = response.text().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!(
            "Failed to read search response: {}",
            e
        ))
    })?;

    // Simple regex-based parsing of the DDG HTML results
    // Each result is in <div class="result results_links results_links_deep web-result ">
    // Title/Link: <a class="result__a" href="...">Title</a>
    // Snippet: <a class="result__snippet" ...>Snippet</a>

    use regex_lite::Regex;
    let re_result = Regex::new(r#"(?s)<div class="result[^"]*">.*?<a class="result__a" href="(?P<url>[^"]+)">(?P<title>.*?)</a>.*?<a class="result__snippet"[^>]*>(?P<snippet>.*?)</a>"#).unwrap();

    let mut out = format!("DuckDuckGo Search Results for '{}':\n\n", query);
    let mut count = 0;

    for caps in re_result.captures_iter(&html) {
        let title = caps["title"].trim();
        let link = caps["url"].trim();
        let snippet = caps["snippet"].trim();

        // Clean up title/snippet (stripping leftover HTML tags)
        let clean_title = Regex::new(r"<[^>]*>").unwrap().replace_all(title, "");
        let clean_snippet = Regex::new(r"<[^>]*>").unwrap().replace_all(snippet, "");

        // Decode URL (DDG often uses /l/?kh=...&uddg=URL)
        let final_url = if link.contains("uddg=") {
            let encoded = link.split("uddg=").nth(1).unwrap_or(link);
            url::form_urlencoded::parse(encoded.as_bytes())
                .filter(|(k, _)| k == "uddg")
                .map(|(_, v)| v.into_owned())
                .next()
                .unwrap_or_else(|| {
                    // Try direct decode if filter failed
                    url::form_urlencoded::parse(encoded.as_bytes())
                        .map(|(_, v)| v.into_owned())
                        .next()
                        .unwrap_or(encoded.to_string())
                })
        } else {
            link.to_string()
        };

        out.push_str(&format!(
            "{}. [{}]({})\n   {}\n\n",
            count + 1,
            clean_title,
            final_url,
            clean_snippet
        ));
        count += 1;
        if count >= 8 {
            break;
        }
    }

    if count == 0 {
        return Ok("No results found on DuckDuckGo.".to_string());
    }

    Ok(out)
}
