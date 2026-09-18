use anyhow::{bail, Context, Result};

use crate::{
    app::AppState,
    domain::{
        character::Character,
        chat::PersistedMessage,
        memory::{
            build_memory_query, cosine_similarity, extraction_messages, extraction_repair_messages,
            parse_memory_candidates, select_memory_context, MemoryDraft, ResolvedMemoryEntry,
        },
        persona::Persona,
        settings::{AppSettings, GenerationSettings},
        template::TemplateContext,
    },
    storage::{
        create_memory_with_vector, list_memories_without_current_vector, mark_messages_processed,
        memory_vectors_for_context, replace_memory_vector, unprocessed_messages,
    },
};

const EXTRACTION_MESSAGE_BATCH: usize = 12;

pub async fn retrieve_memories(
    state: &AppState,
    settings: &AppSettings,
    character: &Character,
    persona: Option<&Persona>,
    chat_id: &str,
    history: &[PersistedMessage],
) -> Result<Vec<ResolvedMemoryEntry>> {
    if !settings.memory.enabled {
        return Ok(Vec::new());
    }
    let Some(model) = settings.memory.embedding_model.as_deref() else {
        return Ok(Vec::new());
    };
    if history.is_empty() {
        return Ok(Vec::new());
    }

    let query = build_memory_query(&character.name, &character.scenario, history);
    let query_vector = state
        .providers
        .embed(
            &settings.provider.active_provider,
            &effective_ollama_url(settings, state),
            model,
            &query,
        )
        .await
        .map_err(|error| anyhow::anyhow!("memory embedding failed: {error}"))?;
    let candidates = memory_vectors_for_context(&state.database, &character.id, chat_id, model)
        .await?
        .into_iter()
        .filter_map(|candidate| {
            cosine_similarity(&query_vector, &candidate.vector)
                .map(|similarity| (candidate.entry, similarity))
        })
        .collect();
    Ok(select_memory_context(
        candidates,
        TemplateContext::new(
            &character.name,
            persona.map(|persona| persona.name.as_str()),
        ),
        settings.memory.similarity_threshold,
        settings.memory.retrieval_count as usize,
        settings.memory.context_budget as usize,
    ))
}

pub async fn extract_chat_memories(
    state: &AppState,
    settings: &AppSettings,
    character_id: &str,
    chat_id: &str,
    require_interval: bool,
) -> Result<MemoryExtractionResult> {
    if !settings.memory.enabled {
        bail!("Enable Semantic Memory in Settings before extracting memories.");
    }
    let embedding_model = settings
        .memory
        .embedding_model
        .as_deref()
        .context("Choose an Embedding Model in Settings before extracting memories.")?;
    let extraction_model = settings
        .memory
        .extraction_model
        .as_deref()
        .or(settings.provider.character_creator_model.as_deref())
        .or(settings.provider.selected_model.as_deref())
        .context(
            "Choose a Memory Extraction Model, Character Creator Model, or chat model first.",
        )?;
    let pending = unprocessed_messages(&state.database, chat_id).await?;
    if require_interval && pending.len() < settings.memory.extraction_interval as usize {
        return Ok(MemoryExtractionResult::default());
    }
    if pending.is_empty() {
        return Ok(MemoryExtractionResult::default());
    }
    let batch = pending
        .into_iter()
        .take(EXTRACTION_MESSAGE_BATCH)
        .collect::<Vec<_>>();
    let generation = extraction_generation(&settings.generation);
    let base_url = effective_ollama_url(settings, state);
    let output = state
        .providers
        .generate_text(
            &settings.provider.active_provider,
            &base_url,
            extraction_model,
            extraction_messages(&batch),
            generation.clone(),
        )
        .await
        .map_err(|error| anyhow::anyhow!("memory extraction failed: {error}"))?;
    let candidates = match parse_memory_candidates(&output) {
        Ok(candidates) => candidates,
        Err(_) => {
            let repaired = state
                .providers
                .generate_text(
                    &settings.provider.active_provider,
                    &base_url,
                    extraction_model,
                    extraction_repair_messages(&output),
                    generation,
                )
                .await
                .map_err(|error| anyhow::anyhow!("memory extraction repair failed: {error}"))?;
            parse_memory_candidates(&repaired).map_err(|_| {
                anyhow::anyhow!("The memory model returned an invalid structured response.")
            })?
        }
    };

    let source_start_message_id = batch.first().map(|message| message.id.clone());
    let source_end_message_id = batch.last().map(|message| message.id.clone());
    let mut result = MemoryExtractionResult {
        processed_messages: batch.len(),
        ..Default::default()
    };
    for candidate in candidates {
        let vector = state
            .providers
            .embed(
                &settings.provider.active_provider,
                &base_url,
                embedding_model,
                &candidate.content,
            )
            .await
            .map_err(|error| anyhow::anyhow!("memory embedding failed: {error}"))?;
        let draft = MemoryDraft {
            chat_id: Some(chat_id.to_owned()),
            content: candidate.content,
            importance: candidate.importance,
            source_start_message_id: source_start_message_id.clone(),
            source_end_message_id: source_end_message_id.clone(),
            ..Default::default()
        };
        match create_memory_with_vector(
            &state.database,
            character_id,
            draft,
            false,
            embedding_model,
            &vector,
        )
        .await
        {
            Ok(_) => result.created += 1,
            Err(error) if error.to_string().contains("already exists") => result.duplicates += 1,
            Err(error) => return Err(error),
        }
    }
    if let Some(last) = source_end_message_id {
        mark_messages_processed(&state.database, chat_id, &last).await?;
    }
    Ok(result)
}

pub async fn rebuild_memory_index(
    state: &AppState,
    settings: &AppSettings,
) -> Result<MemoryReindexResult> {
    let model = settings
        .memory
        .embedding_model
        .as_deref()
        .context("Choose an Embedding Model before rebuilding the memory index.")?;
    let memories = list_memories_without_current_vector(&state.database, model).await?;
    let total = memories.len();
    let base_url = effective_ollama_url(settings, state);
    let mut result = MemoryReindexResult {
        total,
        ..Default::default()
    };
    for memory in memories {
        match state
            .providers
            .embed(
                &settings.provider.active_provider,
                &base_url,
                model,
                &memory.content,
            )
            .await
        {
            Ok(vector) => {
                match replace_memory_vector(&state.database, &memory.id, model, &vector).await {
                    Ok(()) => result.reindexed += 1,
                    Err(error) => {
                        result.failed += 1;
                        tracing::warn!(error = ?error, memory_id = %memory.id, "could not persist rebuilt memory vector");
                    }
                }
            }
            Err(error) => {
                result.failed += 1;
                tracing::warn!(error = ?error, memory_id = %memory.id, "could not rebuild memory vector");
            }
        }
    }
    Ok(result)
}

pub fn effective_ollama_url(settings: &AppSettings, state: &AppState) -> String {
    settings
        .provider
        .ollama_base_url
        .clone()
        .unwrap_or_else(|| state.default_ollama_base_url.clone())
}

fn extraction_generation(base: &GenerationSettings) -> GenerationSettings {
    GenerationSettings {
        temperature: 0.2,
        context_length: base.context_length,
        max_response_length: base.max_response_length.min(768),
    }
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct MemoryExtractionResult {
    pub processed_messages: usize,
    pub created: usize,
    pub duplicates: usize,
}

#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct MemoryReindexResult {
    pub total: usize,
    pub reindexed: usize,
    pub failed: usize,
}
