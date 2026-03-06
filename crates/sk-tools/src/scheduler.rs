use sk_types::ToolDefinition;

/// Tool to create a new scheduled job (cron, regular interval, or specific time).
pub fn schedule_create_tool() -> ToolDefinition {
    ToolDefinition {
        name: "schedule_create".into(),
        description: "Create a new scheduled background job. E.g., run every 1 hour, or at a specific cron expression, executing a SystemEvent or interacting with another agent.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Human readable name for the scheduled job (max 128 chars, alphanumeric + spaces/underscores/hyphens)."
                },
                "schedule_type": {
                    "type": "string",
                    "enum": ["every", "cron"],
                    "description": "The type of schedule: 'every' for intervals in seconds, or 'cron' for standard cron expressions."
                },
                "every_secs": {
                    "type": "integer",
                    "description": "If schedule_type is 'every', the interval in seconds (60 to 86400)."
                },
                "cron_expr": {
                    "type": "string",
                    "description": "If schedule_type is 'cron', a 5-field cron expression, e.g. '0 9 * * 1-5'."
                },
                "task_description": {
                    "type": "string",
                    "description": "What the scheduled job should do. This is the text payload or prompt sent back."
                }
            },
            "required": ["name", "schedule_type", "task_description"]
        }),
    }
}

/// Tool to list all currently scheduled jobs for this agent.
pub fn schedule_list_tool() -> ToolDefinition {
    ToolDefinition {
        name: "schedule_list".into(),
        description: "List all scheduled jobs belonging to you, to see their IDs, schedules, and when they will run next.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

/// Tool to delete a scheduled job.
pub fn schedule_delete_tool() -> ToolDefinition {
    ToolDefinition {
        name: "schedule_delete".into(),
        description: "Delete a scheduled job by its CronJobId.".into(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "job_id": {
                    "type": "string",
                    "description": "The unique UUID of the cron job to delete."
                }
            },
            "required": ["job_id"]
        }),
    }
}
