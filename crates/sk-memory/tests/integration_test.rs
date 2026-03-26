use sk_memory::MemorySubstrate;
use sk_types::memory::*;
use sk_types::AgentId;
use std::collections::HashMap;

#[tokio::test]
async fn test_memory_roundtrip_json() {
    let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
    let agent_id = AgentId::new();

    // 1. Add some data
    substrate
        .set(agent_id, "test_key", serde_json::json!({"foo": "bar"}))
        .await
        .unwrap();

    let _ = substrate
        .remember(
            agent_id,
            "Sovereign Kernel is a powerful agent framework.",
            MemorySource::Observation,
            "test",
            HashMap::new(),
        )
        .await
        .unwrap();

    substrate
        .add_entity(Entity {
            id: "entity_1".into(),
            name: "Sovereign Kernel".into(),
            entity_type: EntityType::Concept,
            properties: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // 2. Export
    let export_data = substrate.export(ExportFormat::Json).await.unwrap();

    // 3. Import into a NEW substrate
    let new_substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
    let report = new_substrate
        .import(&export_data, ExportFormat::Json)
        .await
        .unwrap();

    assert_eq!(report.kv_imported, 1);
    assert_eq!(report.memories_imported, 1);
    assert_eq!(report.entities_imported, 1);

    // 4. Verify data in new substrate
    let val = new_substrate.get(agent_id, "test_key").await.unwrap();
    assert_eq!(val, Some(serde_json::json!({"foo": "bar"})));

    let recall_results = new_substrate.recall("Sovereign", 1, None).await.unwrap();
    assert_eq!(recall_results.len(), 1);
    assert_eq!(
        recall_results[0].content,
        "Sovereign Kernel is a powerful agent framework."
    );
}

#[tokio::test]
async fn test_memory_roundtrip_markdown() {
    let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
    let agent_id = AgentId::new();

    // 1. Add some data
    substrate
        .remember(
            agent_id,
            "Markdown memories should be restorable.",
            MemorySource::Observation,
            "test",
            HashMap::new(),
        )
        .await
        .unwrap();

    // 2. Export
    let md_data = substrate.export(ExportFormat::Markdown).await.unwrap();

    // 3. Import into a NEW substrate
    let new_substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
    let report = new_substrate
        .import(&md_data, ExportFormat::Markdown)
        .await
        .unwrap();

    assert_eq!(report.memories_imported, 1);

    // 4. Verify recall
    let recall_results = new_substrate.recall("restorable", 1, None).await.unwrap();
    assert_eq!(recall_results.len(), 1);
    assert_eq!(
        recall_results[0].content,
        "Markdown memories should be restorable."
    );
}
