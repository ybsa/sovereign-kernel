const AGENT_REGISTRY = {
    "default": {
        rag_chunks: 3,
        memory_depth: 5,
        inject_tools: true,
        temperature: 0.7,
        verbosity: "normal",
        stream: true,
        max_tokens: 500,
        model: "gemma3",
        timeout_ms: 10000,
        tool_access: [],
        max_tool_calls: 5,
        sandbox: true,
        persist: true,
        session_scope: "task",
        summarize_at: 20,
        fallback_model: "gemma3",
        retry_count: 2,
        backoff_ms: 1000
    },
    "shell_agent": {
        role_prompt: "Run shell commands. Be precise and careful.",
        rag_chunks: 1,
        temperature: 0.1,
        max_tokens: 200,
        tool_access: ["shell"],
        max_tool_calls: 3,
        session_scope: "task",
    },
    "browser_agent": {
        role_prompt: "Control the browser. Navigate, click, extract.",
        rag_chunks: 2,
        temperature: 0.3,
        max_tokens: 400,
        tool_access: ["browser"],
        max_tool_calls: 10,
        session_scope: "task",
    },
    "research_agent": {
        role_prompt: "Search and summarize. Be thorough.",
        rag_chunks: 6,
        memory_depth: 10,
        temperature: 0.7,
        max_tokens: 1000,
        model: "claude-haiku-4-5",
        tool_access: ["browser", "files"],
        session_scope: "user",
    },
    "file_agent": {
        role_prompt: "Manage files. Read, write, organize.",
        rag_chunks: 2,
        temperature: 0.2,
        max_tokens: 300,
        tool_access: ["files"],
        max_tool_calls: 5,
        session_scope: "task",
    }
};

let currentAgent = "default";

function selectAgent(agentName) {
    currentAgent = agentName;
    
    // Update buttons
    document.querySelectorAll('.agent-btn').forEach(btn => {
        btn.classList.remove('active');
        if (btn.innerText === agentName) {
            btn.classList.add('active');
        }
    });

    // Merge with default for missing keys
    const config = { ...AGENT_REGISTRY.default, ...AGENT_REGISTRY[agentName] };

    // Update UI with animation
    for (const [key, value] of Object.entries(config)) {
        const span = document.getElementById(`val-${key}`);
        if (span) {
            if (span.innerText != formatValue(value)) {
                span.style.opacity = '0';
                setTimeout(() => {
                    span.innerText = formatValue(value);
                    span.style.opacity = '1';
                }, 200);
            }
        }
    }

    // Update layers if needed (can add specific logic for layer descriptions here)
}

function formatValue(val) {
    if (Array.isArray(val)) {
        return val.length === 0 ? "[]" : `[${val.join(', ')}]`;
    }
    if (typeof val === 'boolean') {
        return val ? "True" : "False";
    }
    return val;
}

function exploreField(category) {
    const modal = document.getElementById('modal');
    const modalBody = document.getElementById('modal-body');
    
    let content = "";
    const config = { ...AGENT_REGISTRY.default, ...AGENT_REGISTRY[currentAgent] };

    switch(category) {
        case 'context':
            content = `
                <h2>Context Configuration</h2>
                <p>Controls how much history and external data is injected into the prompt.</p>
                <pre>
rag_chunks: ${config.rag_chunks} (Retrieval chunks)
memory_depth: ${config.memory_depth} (Historical turns)
inject_tools: ${config.inject_tools} (Enable/Disable definitions)
                </pre>
            `;
            break;
        case 'behavior':
            content = `
                <h2>Behavioral Parameters</h2>
                <p>Tunes the "personality" and output characteristics of the agent.</p>
                <pre>
temperature: ${config.temperature}
verbosity: "${config.verbosity}"
stream: ${config.stream}
                </pre>
            `;
            break;
        case 'cost':
            content = `
                <h2>Cost & Performance</h2>
                <p>Limits resource consumption and sets model targets.</p>
                <pre>
max_tokens: ${config.max_tokens}
model: "${config.model}"
timeout_ms: ${config.timeout_ms}
                </pre>
            `;
            break;
        case 'access':
            content = `
                <h2>Permission Sandbox</h2>
                <p>Defines what the agent is allowed to do within the host system.</p>
                <pre>
tool_access: ${formatValue(config.tool_access)}
max_tool_calls: ${config.max_tool_calls}
sandbox: ${config.sandbox}
                </pre>
            `;
            break;
        case 'memory':
            content = `
                <h2>Persistence & Scope</h2>
                <p>Determines how information is stored and shared across sessions.</p>
                <pre>
persist: ${config.persist}
session_scope: "${config.session_scope}"
summarize_at: ${config.summarize_at}
                </pre>
            `;
            break;
        case 'fallback':
            content = `
                <h2>Resilience Policy</h2>
                <p>Defines what happens if the primary model fails or times out.</p>
                <pre>
fallback_model: "${config.fallback_model}"
retry_count: ${config.retry_count}
backoff_ms: ${config.backoff_ms}
                </pre>
            `;
            break;
    }

    modalBody.innerHTML = content;
    modal.style.display = 'block';
}

function closeModal() {
    document.getElementById('modal').style.display = 'none';
}

// Close modal when clicking outside
window.onclick = function(event) {
    const modal = document.getElementById('modal');
    if (event.target == modal) {
        closeModal();
    }
}

// Initial state
selectAgent('shell_agent');
selectAgent('default'); // Start with default then can switch
