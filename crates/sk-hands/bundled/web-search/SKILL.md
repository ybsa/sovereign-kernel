---
name: web-search-skill
version: "1.0.0"
description: "Expert web research — multi-source synthesis, citation standards, and intelligence reporting"
---

# Web Search Hand — Research Methodology

## Core Principle: Evidence-First Research

Never state facts without sources. Every claim in your reports must be traceable to a URL you actually fetched. When you can't verify something, say so explicitly.

## Query Construction

### Good Queries (Specific, Targeted)

- `"climate change Arctic ice 2024 peer reviewed study"`
- `"Python asyncio performance benchmark 2024"`
- `site:arxiv.org transformer architecture efficiency`

### Bad Queries (Too Vague)

- `climate`
- `python`

### Query Patterns by Task Type

| Task | Pattern |
| --- | --- |
| Current events | `[topic] [year] news` |
| Technical facts | `[technology] [specific aspect] documentation` |
| Comparisons | `[A] vs [B] comparison [year]` |
| Statistics | `[metric] statistics [source type] [year]` |
| Expert opinion | `[topic] expert analysis site:nature.com OR site:harvard.edu` |

## Source Credibility Tiers

1. **Primary** — Academic papers, official documentation, government data
2. **Secondary** — Quality journalism (Reuters, AP, BBC, NYT), industry reports
3. **Tertiary** — Blogs, forums, social media (corroborate with primary)

Always prefer primary sources. When citing tertiary sources, note the limitation.

## Synthesis Rules

After fetching multiple sources:

1. **Corroborate**: A fact stated by 3+ independent sources is likely reliable
2. **Contradiction**: When sources disagree, present both sides with their evidence
3. **Recency**: For fast-moving topics, prefer sources from the last 6 months
4. **Currency bias**: Be aware that your own knowledge has a cutoff — prioritize fetched content

## Anti-Hallucination Protocol

Before writing any fact in your report:

- [ ] Can you cite a fetched URL for this claim?
- [ ] Is this verifiably true? If not, add hedging language
- [ ] Are you inferring beyond what the source actually states?
- [ ] Did you search for disconfirming evidence?

## Report Templates

### Quick Answer

```text
**Answer**: [1-2 sentence direct answer]
**Source**: [url]
**Confidence**: High/Medium/Low

```

### Full Research Report

```text

## [Topic] Research Report

*Generated: YYYY-MM-DD | Queries: N | Sources: N*

### Executive Summary

...

### Key Findings

- [Finding] ([source](url))
- [Finding] ([source](url))

### Detailed Analysis

...

### Limitations

- Areas not covered
- Conflicting information found

### Sources

1. [Title](url) — [credibility tier]

```

## Scheduling Research Tasks

For recurring research (e.g. "monitor AI news weekly"):

- Use `schedule_create` with appropriate cron expression
- Store results summary in memory with `memory_store`
- Publish completion event with `event_publish`
