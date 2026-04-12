//! Web search tool — fetches real-time search results via HTTP.
use sk_types::ToolDefinition;

pub fn web_search_tool() -> ToolDefinition {
    ToolDefinition {
        name: "web_search".into(),
        description: "Search the web for current, real-time information. Returns structured search result snippets.".into(),
        input_schema: serde_json::json!({"type":"object","properties":{"query":{"type":"string","description":"The search query"}},"required":["query"]}),
    }
}

pub async fn handle_web_search(query: &str) -> Result<String, sk_types::SovereignError> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|e| sk_types::SovereignError::ToolExecutionError(format!("HTTP client error: {}", e)))?;

    let mut query_encoded = String::new();
    url::form_urlencoded::Serializer::new(&mut query_encoded).append_pair("q", query);

    // ── Attempt 1: DuckDuckGo HTML ──────────────────────────────────────
    let ddg_url = format!("https://html.duckduckgo.com/html/?{}", query_encoded);
    if let Ok(results) = fetch_and_parse(&client, &ddg_url, SearchEngine::DuckDuckGo).await {
        if !results.is_empty() {
            return Ok(format_results(query, &results));
        }
    }

    // ── Attempt 2: Google Fallback ──────────────────────────────────────
    let google_url = format!("https://www.google.com/search?{}&num=8", query_encoded);
    if let Ok(results) = fetch_and_parse(&client, &google_url, SearchEngine::Google).await {
        if !results.is_empty() {
            return Ok(format_results(query, &results));
        }
    }

    // ── Attempt 3: Raw text fallback (last resort) ──────────────────────
    let ddg_url2 = format!("https://html.duckduckgo.com/html/?{}", query_encoded);
    let raw = fetch_raw_text(&client, &ddg_url2).await.unwrap_or_default();
    if raw.len() > 200 {
        return Ok(format!("Search Results for '{}' (raw):\n\n{}", query, raw));
    }

    Err(sk_types::SovereignError::ToolExecutionError(
        "All search engines returned empty results. The query may be rate-limited.".into(),
    ))
}

#[derive(Debug)]
struct SearchResult {
    title: String,
    snippet: String,
    url: String,
}

enum SearchEngine {
    DuckDuckGo,
    Google,
}

async fn fetch_and_parse(
    client: &reqwest::Client,
    url: &str,
    engine: SearchEngine,
) -> Result<Vec<SearchResult>, sk_types::SovereignError> {
    let resp = client.get(url).send().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("HTTP request failed: {}", e))
    })?;

    // Check for rate limiting / challenge pages
    let status = resp.status().as_u16();
    if status == 202 || status == 403 || status == 429 {
        return Err(sk_types::SovereignError::ToolExecutionError(format!(
            "Search engine returned status {}: rate-limited or blocked",
            status
        )));
    }

    let html = resp.text().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Failed to read body: {}", e))
    })?;

    let results = match engine {
        SearchEngine::DuckDuckGo => parse_ddg_results(&html),
        SearchEngine::Google => parse_google_results(&html),
    };

    Ok(results)
}

/// Parse DuckDuckGo HTML search results.
/// DDG uses <div class="result ..."> blocks containing:
///   - <a class="result__a"> for titles
///   - <a class="result__snippet"> for descriptions
///   - <a class="result__url"> for URLs
fn parse_ddg_results(html: &str) -> Vec<SearchResult> {
    use regex_lite::Regex;

    let title_re = Regex::new(r#"(?is)<a[^>]*class="result__a"[^>]*>(.*?)</a>"#).unwrap();
    let snippet_re = Regex::new(r#"(?is)<a[^>]*class="result__snippet"[^>]*>(.*?)</a>"#).unwrap();
    let url_re = Regex::new(r#"(?is)<a[^>]*class="result__url"[^>]*href="([^"]*)"[^>]*>"#).unwrap();
    let tag_strip = Regex::new(r"<[^>]+>").unwrap();

    let titles: Vec<String> = title_re
        .captures_iter(html)
        .map(|c| tag_strip.replace_all(&c[1], "").trim().to_string())
        .collect();
    let snippets: Vec<String> = snippet_re
        .captures_iter(html)
        .map(|c| tag_strip.replace_all(&c[1], "").trim().to_string())
        .collect();
    let urls: Vec<String> = url_re
        .captures_iter(html)
        .map(|c| c[1].trim().to_string())
        .collect();

    let mut results = Vec::new();
    for (i, title) in titles.iter().enumerate() {
        if !title.is_empty() {
            results.push(SearchResult {
                title: title.clone(),
                snippet: snippets.get(i).cloned().unwrap_or_default(),
                url: urls.get(i).cloned().unwrap_or_default(),
            });
        }
    }

    results.truncate(8);
    results
}

/// Parse Google search results.
fn parse_google_results(html: &str) -> Vec<SearchResult> {
    use regex_lite::Regex;

    let tag_strip = Regex::new(r"<[^>]+>").unwrap();
    let h3_re = Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>").unwrap();
    let href_re = Regex::new(r#"(?is)<a[^>]*href="(https?://[^"]+)"[^>]*>"#).unwrap();

    let mut results = Vec::new();

    // Extract h3 titles and nearby links
    let titles: Vec<String> = h3_re
        .captures_iter(html)
        .map(|c| tag_strip.replace_all(&c[1], "").trim().to_string())
        .filter(|t| !t.is_empty() && t.len() > 5)
        .collect();
    let hrefs: Vec<String> = href_re
        .captures_iter(html)
        .map(|c| c[1].trim().to_string())
        .filter(|u| !u.contains("google.com"))
        .collect();

    for (i, title) in titles.iter().enumerate() {
        results.push(SearchResult {
            title: title.clone(),
            snippet: String::new(),
            url: hrefs.get(i).cloned().unwrap_or_default(),
        });
    }

    results.truncate(8);
    results
}

/// Format search results into a readable string for the LLM.
fn format_results(query: &str, results: &[SearchResult]) -> String {
    let mut out = format!(
        "Search Results for '{}' ({} results):\n\n",
        query,
        results.len()
    );
    for (i, r) in results.iter().enumerate() {
        out.push_str(&format!("{}. {}\n", i + 1, r.title));
        if !r.snippet.is_empty() {
            out.push_str(&format!("   {}\n", r.snippet));
        }
        if !r.url.is_empty() {
            out.push_str(&format!("   URL: {}\n", r.url));
        }
        out.push('\n');
    }
    out
}

/// Last-resort fallback: strip all HTML and return raw text.
async fn fetch_raw_text(
    client: &reqwest::Client,
    url: &str,
) -> Result<String, sk_types::SovereignError> {
    let resp = client.get(url).send().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("HTTP request failed: {}", e))
    })?;
    let html = resp.text().await.map_err(|e| {
        sk_types::SovereignError::ToolExecutionError(format!("Failed to read body: {}", e))
    })?;

    use regex_lite::Regex;
    let mut text = Regex::new(r"(?is)<script[^>]*>.*?</script>")
        .unwrap()
        .replace_all(&html, "")
        .into_owned();
    text = Regex::new(r"(?is)<style[^>]*>.*?</style>")
        .unwrap()
        .replace_all(&text, "")
        .into_owned();
    text = Regex::new(r"(?is)<[^>]+>")
        .unwrap()
        .replace_all(&text, " ")
        .into_owned();
    text = Regex::new(r"[\t ]+")
        .unwrap()
        .replace_all(&text, " ")
        .into_owned();
    text = Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&text, "\n\n")
        .into_owned();

    Ok(text.trim().chars().take(5000).collect())
}
