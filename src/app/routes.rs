use std::{convert::Infallible, time::Duration};

use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post},
    Json, Router,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing::{error, info, warn};

use crate::{
    app::{extract_chat_memories, rebuild_memory_index, retrieve_memories, AppState, APP_VERSION},
    domain::{
        character::{Character, CharacterDraft, CharacterSummary},
        character_creator::{
            apply_regenerated_field, build_character_draft_messages,
            build_field_regeneration_messages, build_repair_messages, parse_generated_draft,
            validate_creation_request, CharacterCreatorField,
        },
        chat::{Chat, CreateChatRequest, PersistedMessage, RenameChatRequest},
        conversation::{validate_message_content, ConversationMessage},
        image::{normalize_visual_prompt, ChatImage, ImageGenerationRequest},
        image_prompt::{portrait_prompt_messages, scene_prompt_messages},
        lorebook::{Lorebook, LorebookDraft, LorebookEntry, LorebookEntryDraft},
        memory::{MemoryDraft, MemoryEntry},
        persona::{Persona, PersonaDraft},
        prompt::build_chat_request_with_context,
        settings::{AppSettings, ProviderKind},
        template::{resolve_template, TemplateContext},
    },
    importer::parse_character_card,
    providers::{ImageGenerationError, ProviderError, ProviderModel},
    storage::{
        append_assistant_message, append_user_message, copy_avatar, count_chats_for_persona,
        count_memories, create_character, create_chat, create_chat_image, create_lorebook,
        create_lorebook_entry, create_memory_with_vector, create_persona, delete_character,
        delete_chat, delete_chat_image, delete_lorebook, delete_lorebook_entry, delete_memory,
        delete_persona, duplicate_character, duplicate_lorebook, duplicate_persona,
        ensure_avatar_exists, get_character, get_chat_for_character, get_lorebook,
        get_lorebook_entry, get_memory, get_persona, list_character_lorebooks, list_characters,
        list_chat_images, list_chat_lorebooks, list_chat_messages, list_chats_for_character,
        list_lorebook_entries, list_lorebooks, list_memories, list_message_lore_entries,
        list_message_memories, list_personas, list_resolvable_lore_entries, load_settings,
        record_message_lore_entries, record_message_memories, remove_avatar,
        remove_chat_image_file, rename_chat, save_settings, set_character_lorebooks,
        set_chat_lorebooks, set_chat_persona, store_avatar, store_image, update_character,
        update_lorebook, update_lorebook_entry, update_memory_with_vector, update_persona,
        AvatarStorageError,
    },
};

pub fn router(state: AppState) -> Router {
    let avatar_directory = state.avatar_directory.clone();
    let image_directory = state.image_directory.clone();
    Router::new()
        .route("/api/health", get(health_check))
        .route("/api/about", get(about))
        .route("/api/settings", get(get_settings).put(update_settings))
        .route(
            "/api/memories",
            get(list_existing_memories).post(create_new_memory),
        )
        .route("/api/memories/status", get(memory_status))
        .route("/api/memories/rebuild", post(rebuild_memories))
        .route(
            "/api/memories/{id}",
            get(get_existing_memory)
                .put(update_existing_memory)
                .delete(delete_existing_memory),
        )
        .route("/api/providers/text/test", post(test_text_connection))
        // Retain these v1.4 route aliases so an already-cached app shell can
        // still reconnect after a server upgrade. They use the configured
        // active text backend just like the current endpoints.
        .route("/api/providers/ollama/test", post(test_text_connection))
        .route("/api/providers/a1111/test", post(test_a1111_connection))
        .route(
            "/api/providers/a1111/models",
            get(list_a1111_models).post(list_a1111_models),
        )
        .route(
            "/api/providers/text/models",
            get(list_text_models).post(list_text_models),
        )
        .route(
            "/api/providers/ollama/models",
            get(list_text_models).post(list_text_models),
        )
        .route(
            "/api/providers/embeddings/models",
            get(list_embedding_models).post(list_embedding_models),
        )
        .route(
            "/api/providers/text/model",
            axum::routing::put(select_text_model),
        )
        .route(
            "/api/providers/ollama/model",
            axum::routing::put(select_text_model),
        )
        .route(
            "/api/characters",
            get(list_character_summaries).post(create_new_character),
        )
        .route("/api/characters/import", post(import_character_card))
        .route(
            "/api/character-avatar/generate",
            post(generate_character_avatar),
        )
        .route(
            "/api/generated-avatars/{reference}",
            axum::routing::delete(discard_generated_avatar),
        )
        .route(
            "/api/character-creator/generate",
            post(generate_character_draft),
        )
        .route(
            "/api/character-creator/regenerate",
            post(regenerate_character_draft_field),
        )
        .route(
            "/api/characters/{id}",
            get(get_existing_character)
                .put(update_existing_character)
                .delete(delete_existing_character),
        )
        .route(
            "/api/characters/{id}/duplicate",
            post(duplicate_existing_character),
        )
        .route(
            "/api/personas",
            get(list_existing_personas).post(create_new_persona),
        )
        .route(
            "/api/personas/{id}",
            get(get_existing_persona)
                .put(update_existing_persona)
                .delete(delete_existing_persona),
        )
        .route(
            "/api/personas/{id}/duplicate",
            post(duplicate_existing_persona),
        )
        .route(
            "/api/lorebooks",
            get(list_existing_lorebooks).post(create_new_lorebook),
        )
        .route("/api/lorebooks/import", post(import_lorebook))
        .route(
            "/api/lorebooks/{id}",
            get(get_existing_lorebook)
                .put(update_existing_lorebook)
                .delete(delete_existing_lorebook),
        )
        .route(
            "/api/lorebooks/{id}/duplicate",
            post(duplicate_existing_lorebook),
        )
        .route("/api/lorebooks/{id}/export", get(export_lorebook))
        .route(
            "/api/lorebooks/{id}/entries",
            get(list_existing_lore_entries).post(create_new_lore_entry),
        )
        .route(
            "/api/lorebooks/{lorebook_id}/entries/{entry_id}",
            get(get_existing_lore_entry)
                .put(update_existing_lore_entry)
                .delete(delete_existing_lore_entry),
        )
        .route(
            "/api/characters/{character_id}/lorebooks",
            get(list_character_lorebook_associations).put(set_character_lorebook_associations),
        )
        .route(
            "/api/characters/{character_id}/chats",
            get(list_character_chats).post(create_new_chat),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}",
            get(get_chat_detail)
                .put(rename_existing_chat)
                .delete(delete_existing_chat),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/persona",
            axum::routing::put(change_chat_persona),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/lorebooks",
            get(list_chat_lorebook_associations).put(set_chat_lorebook_associations),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/messages",
            get(list_existing_chat_messages),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/messages/{message_id}/lore",
            get(list_message_lore_context),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/messages/{message_id}/context",
            get(list_message_context),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/memories/extract",
            post(extract_memories_for_chat),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/images",
            get(list_existing_chat_images).post(generate_chat_scene_image),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/images/interrupt",
            post(interrupt_chat_scene_image),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/images/{image_id}",
            axum::routing::delete(delete_existing_chat_image),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/generate",
            post(generate_chat_response),
        )
        .route(
            "/api/characters/{character_id}/chats/{chat_id}/partial-assistant",
            post(persist_partial_assistant_message),
        )
        .route("/api/avatars", post(upload_avatar))
        .nest_service("/avatars", ServeDir::new(avatar_directory))
        .nest_service("/images", ServeDir::new(image_directory))
        .fallback_service(ServeDir::new("public").append_index_html_on_directories(true))
        .layer(DefaultBodyLimit::max(5 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn about() -> Json<AboutResponse> {
    Json(AboutResponse {
        name: "NyxAI",
        version: APP_VERSION,
        description: "Self-hosted, local-first roleplay chat.",
    })
}

async fn get_settings(State(state): State<AppState>) -> Result<Json<SettingsResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    Ok(Json(settings_response(&settings, &state)))
}

async fn update_settings(
    State(state): State<AppState>,
    Json(mut settings): Json<AppSettings>,
) -> Result<Json<SettingsResponse>, ApiError> {
    settings
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    if let Some(persona_id) = settings.persona.default_persona_id.as_deref() {
        if get_persona(&state.database, persona_id).await?.is_none() {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Choose an available default persona."
            )));
        }
    }
    save_settings(&state.database, &settings).await?;
    Ok(Json(settings_response(&settings, &state)))
}

async fn test_text_connection(
    State(state): State<AppState>,
) -> Result<Json<ConnectionTestResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    let base_url = effective_text_url(&settings, &state);
    let report = state
        .providers
        .test_connection(&settings.provider.active_provider, &base_url)
        .await
        .map_err(ApiError::provider)?;

    Ok(Json(ConnectionTestResponse {
        connected: report.connected,
        model_count: report.model_count,
        message: if report.model_count == 0 {
            "Connected to the configured local text backend. No models were discovered.".to_owned()
        } else {
            format!(
                "Connected to the configured local text backend. {} model(s) available.",
                report.model_count
            )
        },
    }))
}

async fn test_a1111_connection(
    State(state): State<AppState>,
) -> Result<Json<ImageConnectionTestResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    let report = state
        .image_providers
        .test_connection(&effective_a1111_url(&settings, &state))
        .await
        .map_err(ApiError::image_provider)?;
    Ok(Json(ImageConnectionTestResponse {
        connected: report.connected,
        model_count: report.model_count,
        message: match report.model_count {
            Some(count) => format!("Connected to the image backend. {count} model(s) found."),
            None => "Connected to the image backend.".to_owned(),
        },
    }))
}

async fn list_a1111_models(
    State(state): State<AppState>,
) -> Result<Json<ImageModelsResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    let models = state
        .image_providers
        .list_models(&effective_a1111_url(&settings, &state))
        .await
        .map_err(ApiError::image_provider)?;
    let selected_model_available = settings
        .image
        .selected_model
        .as_ref()
        .map(|model| models.iter().any(|available| available == model))
        .unwrap_or(true);
    Ok(Json(ImageModelsResponse {
        models,
        selected_model: settings.image.selected_model,
        selected_model_available,
    }))
}

async fn list_text_models(State(state): State<AppState>) -> Result<Json<ModelsResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    let base_url = effective_text_url(&settings, &state);
    let models = state
        .providers
        .list_models(&settings.provider.active_provider, &base_url)
        .await
        .map_err(ApiError::provider)?;
    let selected_model_available = settings
        .provider
        .selected_model
        .as_ref()
        .is_none_or(|selected| models.iter().any(|model| model.name == *selected));

    Ok(Json(ModelsResponse {
        models,
        selected_model: settings.provider.selected_model,
        selected_model_available,
    }))
}

async fn select_text_model(
    State(state): State<AppState>,
    Json(request): Json<SelectModelRequest>,
) -> Result<Json<ModelsResponse>, ApiError> {
    let model_name = request.model.trim();
    if model_name.is_empty() || model_name.chars().count() > 256 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Choose a valid local model identifier."
        )));
    }

    let mut settings = load_settings(&state.database).await?;
    let base_url = effective_text_url(&settings, &state);
    // Discovery is optional for compatible local servers. Keep Ollama's
    // installed-model validation, while allowing an explicit identifier only
    // when the active adapter advertises manual entry as a fallback.
    let capabilities = state
        .providers
        .capabilities(&settings.provider.active_provider);
    let models = match state
        .providers
        .list_models(&settings.provider.active_provider, &base_url)
        .await
    {
        Ok(models) => models,
        Err(crate::providers::ProviderError::UnsupportedCapability("model discovery"))
            if capabilities.manual_model_entry =>
        {
            Vec::new()
        }
        Err(error) => return Err(ApiError::provider(error)),
    };

    if !capabilities.manual_model_entry && !models.iter().any(|model| model.name == model_name) {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "That model is not currently available in the configured backend. Refresh the list and choose another model."
        )));
    }

    settings.provider.selected_model = Some(model_name.to_owned());
    save_settings(&state.database, &settings).await?;
    let selected_model_available =
        models.is_empty() || models.iter().any(|model| model.name == model_name);

    Ok(Json(ModelsResponse {
        models,
        selected_model: settings.provider.selected_model,
        selected_model_available,
    }))
}

async fn list_embedding_models(
    State(state): State<AppState>,
) -> Result<Json<ModelsResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    let models = state
        .providers
        .list_models(
            &settings.memory.embedding_provider,
            &effective_embedding_url(&settings, &state),
        )
        .await
        .map_err(ApiError::provider)?;
    let selected_model_available = settings
        .memory
        .embedding_model
        .as_ref()
        .is_none_or(|selected| models.iter().any(|model| model.name == *selected));
    Ok(Json(ModelsResponse {
        models,
        selected_model: settings.memory.embedding_model,
        selected_model_available,
    }))
}

async fn generate_character_draft(
    State(state): State<AppState>,
    Json(request): Json<CharacterCreatorGenerateRequest>,
) -> Result<Json<CharacterCreatorDraftResponse>, ApiError> {
    validate_creation_request(&request.prompt, request.alternate_greeting_count)
        .map_err(ApiError::bad_request)?;
    let settings = load_settings(&state.database).await?;
    let model =
        resolve_character_creator_model(&state, &settings, request.model.as_deref()).await?;
    let _activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "An image generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    let generation = character_creator_generation_settings(&settings.generation);

    let first_output = state
        .providers
        .generate_text(
            &settings.provider.active_provider,
            &effective_text_url(&settings, &state),
            &model,
            build_character_draft_messages(&request.prompt, request.alternate_greeting_count),
            generation.clone(),
        )
        .await
        .map_err(ApiError::provider)?;

    let draft = match parse_generated_draft(&first_output, request.alternate_greeting_count) {
        Ok(draft) => draft,
        Err(first_error) => {
            warn!(error = ?first_error, "character creator returned malformed draft; attempting one repair");
            let repaired_output = state
                .providers
                .generate_text(
                    &settings.provider.active_provider,
                    &effective_text_url(&settings, &state),
                    &model,
                    build_repair_messages(
                        &request.prompt,
                        &first_output,
                        request.alternate_greeting_count,
                    ),
                    generation,
                )
                .await
                .map_err(ApiError::provider)?;
            parse_generated_draft(&repaired_output, request.alternate_greeting_count)
                .map_err(ApiError::creator_output)?
        }
    };

    Ok(Json(CharacterCreatorDraftResponse { draft }))
}

async fn regenerate_character_draft_field(
    State(state): State<AppState>,
    Json(request): Json<CharacterCreatorRegenerateRequest>,
) -> Result<Json<CharacterCreatorDraftResponse>, ApiError> {
    validate_creation_request(&request.original_prompt, request.alternate_greeting_count)
        .map_err(ApiError::bad_request)?;
    if request.draft.name.trim().is_empty() {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Give the character a name before regenerating a field."
        )));
    }
    if request.instruction.chars().count() > 4_000 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Field instructions can be at most 4000 characters."
        )));
    }

    let settings = load_settings(&state.database).await?;
    let model =
        resolve_character_creator_model(&state, &settings, request.model.as_deref()).await?;
    let _activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "An image generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    let output = state
        .providers
        .generate_text(
            &settings.provider.active_provider,
            &effective_text_url(&settings, &state),
            &model,
            build_field_regeneration_messages(
                &request.draft,
                request.field,
                &request.original_prompt,
                &request.instruction,
                request.alternate_greeting_count,
            ),
            character_creator_generation_settings(&settings.generation),
        )
        .await
        .map_err(ApiError::provider)?;
    let draft = apply_regenerated_field(
        &output,
        request.field,
        &request.draft,
        request.alternate_greeting_count,
    )
    .map_err(ApiError::creator_output)?;

    Ok(Json(CharacterCreatorDraftResponse { draft }))
}

async fn resolve_character_creator_model(
    state: &AppState,
    settings: &AppSettings,
    requested_model: Option<&str>,
) -> Result<String, ApiError> {
    let requested_model = requested_model
        .map(str::trim)
        .filter(|model| !model.is_empty());
    if requested_model.is_some_and(|model| model.chars().count() > 256) {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Choose a valid Character Creator model."
        )));
    }
    let model = requested_model
        .map(str::to_owned)
        .or_else(|| settings.provider.character_creator_model.clone())
        .or_else(|| settings.provider.selected_model.clone())
        .ok_or_else(|| {
            ApiError::bad_request(anyhow::anyhow!(
                "Choose a Character Creator model or a chat model in Settings first."
            ))
        })?;
    let capabilities = state
        .providers
        .capabilities(&settings.provider.active_provider);
    let models = match state
        .providers
        .list_models(
            &settings.provider.active_provider,
            &effective_text_url(settings, state),
        )
        .await
    {
        Ok(models) => models,
        Err(crate::providers::ProviderError::UnsupportedCapability("model discovery"))
            if capabilities.manual_model_entry =>
        {
            Vec::new()
        }
        Err(error) => return Err(ApiError::provider(error)),
    };
    if !capabilities.manual_model_entry && !models.iter().any(|available| available.name == model) {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "That Character Creator model is not currently available. Refresh local models and choose another one."
        )));
    }
    Ok(model)
}

fn character_creator_generation_settings(
    settings: &crate::domain::settings::GenerationSettings,
) -> crate::domain::settings::GenerationSettings {
    let mut creator_settings = settings.clone();
    // Structured drafts with several greetings need enough space to finish;
    // this remains bounded and does not add a second advanced-settings surface.
    creator_settings.max_response_length = creator_settings.max_response_length.clamp(1_536, 4_096);
    creator_settings
}

async fn list_character_chats(
    State(state): State<AppState>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<Chat>>, ApiError> {
    ensure_character_exists(&state, &character_id).await?;
    Ok(Json(
        list_chats_for_character(&state.database, &character_id).await?,
    ))
}

async fn create_new_chat(
    State(state): State<AppState>,
    Path(character_id): Path<String>,
    Json(request): Json<CreateChatRequest>,
) -> Result<(StatusCode, Json<ChatDetailResponse>), ApiError> {
    let character = ensure_character_exists(&state, &character_id).await?;
    let raw_greeting = match request.greeting_id.as_deref() {
        Some(greeting_id) => character
            .alternate_greetings
            .iter()
            .find(|greeting| greeting.id == greeting_id)
            .map(|greeting| greeting.content.clone())
            .ok_or_else(|| {
                ApiError::bad_request(anyhow::anyhow!(
                    "That greeting is no longer available. Refresh the character and try again."
                ))
            })?,
        None => character.first_message.clone(),
    };
    let persona_selection = request.persona_id.clone();
    let title = request.validated_title().map_err(ApiError::bad_request)?;
    let persona = resolve_new_chat_persona(&state, persona_selection).await?;
    let greeting = resolve_template(
        &raw_greeting,
        TemplateContext::new(
            &character.name,
            persona.as_ref().map(|persona| persona.name.as_str()),
        ),
    );
    let chat = create_chat(
        &state.database,
        &character_id,
        &title,
        Some(&greeting),
        persona.as_ref().map(|persona| persona.id.as_str()),
    )
    .await?
    .ok_or_else(|| ApiError::not_found("That character no longer exists."))?;
    let messages = list_chat_messages(&state.database, &chat.id).await?;
    let images = list_chat_images(&state.database, &chat.id).await?;
    let lorebooks = list_chat_lorebooks(&state.database, &chat.id).await?;

    Ok((
        StatusCode::CREATED,
        Json(ChatDetailResponse {
            character,
            chat,
            persona,
            messages,
            images,
            lorebooks,
        }),
    ))
}

async fn get_chat_detail(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<Json<ChatDetailResponse>, ApiError> {
    Ok(Json(chat_detail(&state, &character_id, &chat_id).await?))
}

async fn list_existing_chat_messages(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<Json<Vec<PersistedMessage>>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    Ok(Json(list_chat_messages(&state.database, &chat_id).await?))
}

async fn list_existing_chat_images(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<Json<Vec<ChatImage>>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    Ok(Json(list_chat_images(&state.database, &chat_id).await?))
}

async fn generate_character_avatar(
    State(state): State<AppState>,
    Json(mut draft): Json<CharacterDraft>,
) -> Result<Json<AvatarCandidateResponse>, ApiError> {
    draft = draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    let settings = load_settings(&state.database).await?;
    let _activity = state.resources.try_begin_image().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Another AI generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    let prompt =
        generate_visual_prompt(&state, &settings, portrait_prompt_messages(&draft)).await?;
    let image =
        generate_local_image(&state, &settings, prompt.clone(), ImageDimensions::Avatar).await?;
    let avatar_path = store_avatar(&state.avatar_directory, &image.bytes)
        .await
        .map_err(ApiError::avatar)?;
    let avatar_url =
        crate::storage::avatar_url(&avatar_path).expect("generated avatar names are safe");
    Ok(Json(AvatarCandidateResponse {
        avatar_path,
        avatar_url,
        prompt,
    }))
}

async fn discard_generated_avatar(
    State(state): State<AppState>,
    Path(reference): Path<String>,
) -> Result<StatusCode, ApiError> {
    remove_avatar(&state.avatar_directory, &reference)
        .await
        .map_err(ApiError::avatar)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn generate_chat_scene_image(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<(StatusCode, Json<ChatImage>), ApiError> {
    let detail = chat_detail(&state, &character_id, &chat_id).await?;
    let settings = load_settings(&state.database).await?;
    let _activity = state.resources.try_begin_image().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Another AI generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    let history = detail
        .messages
        .iter()
        .map(|message| ConversationMessage {
            role: message.role,
            content: message.content.clone(),
        })
        .collect::<Vec<_>>();
    let prompt = generate_visual_prompt(
        &state,
        &settings,
        scene_prompt_messages(
            &detail.character.name,
            &detail.character.description,
            &detail.character.scenario,
            detail.persona.as_ref(),
            &history,
        ),
    )
    .await?;
    let image =
        generate_local_image(&state, &settings, prompt.clone(), ImageDimensions::Chat).await?;
    let file_path = store_image(&state.image_directory, &image.bytes)
        .await
        .map_err(ApiError::avatar)?;
    match create_chat_image(
        &state.database,
        &detail.chat.id,
        &file_path,
        &prompt,
        settings.image.selected_model.as_deref(),
        settings.image.chat_width,
        settings.image.chat_height,
    )
    .await
    {
        Ok(image) => Ok((StatusCode::CREATED, Json(image))),
        Err(error) => {
            if let Err(cleanup_error) =
                remove_chat_image_file(&state.image_directory, &file_path).await
            {
                warn!(error = ?cleanup_error, "could not remove unsaved generated image");
            }
            Err(ApiError::from(error))
        }
    }
}

async fn interrupt_chat_scene_image(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    let settings = load_settings(&state.database).await?;
    if !settings.image.enabled {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Enable Image Generation in Settings first."
        )));
    }
    state
        .image_providers
        .interrupt(&effective_a1111_url(&settings, &state))
        .await
        .map_err(ApiError::image_provider)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn delete_existing_chat_image(
    State(state): State<AppState>,
    Path((character_id, chat_id, image_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    let reference = delete_chat_image(&state.database, &chat_id, &image_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That generated image no longer exists."))?;
    if let Err(error) = remove_chat_image_file(&state.image_directory, &reference).await {
        warn!(error = ?error, "could not remove deleted generated image file");
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn rename_existing_chat(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
    Json(request): Json<RenameChatRequest>,
) -> Result<Json<Chat>, ApiError> {
    let title = request.validated_title().map_err(ApiError::bad_request)?;
    let chat = rename_chat(&state.database, &character_id, &chat_id, &title)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    Ok(Json(chat))
}

async fn delete_existing_chat(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let images = list_chat_images(&state.database, &chat_id).await?;
    delete_chat(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    for image in images {
        let reference = image.image_url.trim_start_matches("/images/");
        if let Err(error) = remove_chat_image_file(&state.image_directory, reference).await {
            warn!(error = ?error, image = %reference, "could not remove deleted chat image file");
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn generate_chat_response(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
    Json(request): Json<GenerateChatRequest>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let detail = chat_detail(&state, &character_id, &chat_id).await?;
    validate_message_content(&request.content).map_err(ApiError::bad_request)?;
    validate_generation_id(&request.generation_id).map_err(ApiError::bad_request)?;

    let settings = load_settings(&state.database).await?;
    let selected_model = settings.provider.selected_model.clone().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Choose a local text model before sending a message."
        ))
    })?;
    let activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "An image generation is active. Wait for it to finish or stop it first."
        ))
    })?;

    append_user_message(&state.database, &detail.chat.id, &request.content).await?;
    let history = list_chat_messages(&state.database, &detail.chat.id).await?;
    let lore_entries =
        list_resolvable_lore_entries(&state.database, &detail.character.id, &detail.chat.id)
            .await?;
    let retrieved_memories = match retrieve_memories(
        &state,
        &settings,
        &detail.character,
        detail.persona.as_ref(),
        &detail.chat.id,
        &history,
    )
    .await
    {
        Ok(memories) => memories,
        Err(error) => {
            warn!(error = ?error, chat_id = %detail.chat.id, "semantic memory retrieval was unavailable");
            Vec::new()
        }
    };
    let prompt_build = build_chat_request_with_context(
        &detail.character,
        detail.persona.as_ref(),
        &history,
        &settings.generation,
        &lore_entries,
        &retrieved_memories,
    )
    .map_err(ApiError::bad_request)?;
    let lore_entry_ids = prompt_build
        .lore_used
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    let lore_names = prompt_build
        .lore_used
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    let memory_ids = prompt_build
        .memories_used
        .iter()
        .map(|entry| entry.id.clone())
        .collect::<Vec<_>>();
    let memory_names = prompt_build
        .memories_used
        .iter()
        .map(|entry| entry.content.clone())
        .collect::<Vec<_>>();
    let base_url = effective_text_url(&settings, &state);
    let upstream = state
        .providers
        .stream_chat(
            &settings.provider.active_provider,
            &base_url,
            &selected_model,
            prompt_build.messages,
            settings.generation.clone(),
        )
        .await
        .map_err(ApiError::provider)?;
    let database = state.database.clone();
    let memory_state = state.clone();
    let memory_settings = settings.clone();
    let memory_character_id = detail.character.id.clone();
    let persisted_chat_id = detail.chat.id;
    let generation_id = request.generation_id;

    // Dropping the SSE response cancels the upstream request. The browser then
    // persists any useful partial response through the idempotent endpoint.
    let stream = async_stream::stream! {
        let _activity = activity;
        let mut upstream = upstream;
        let mut content = String::new();
        if !lore_names.is_empty() || !memory_names.is_empty() {
            yield Ok(sse_json("context", StreamContext { lore_used: lore_names.clone(), memories_used: memory_names.clone() }));
        }
        while let Some(next) = upstream.next().await {
            match next {
                Ok(chunk) => {
                    if !chunk.content.is_empty() {
                        content.push_str(&chunk.content);
                        yield Ok(sse_json("delta", StreamDelta { content: chunk.content }));
                    }
                    if chunk.done {
                        if let Err(error) = persist_completed_assistant_message(
                            &database,
                            &persisted_chat_id,
                            &content,
                            &generation_id,
                            &lore_entry_ids,
                            &memory_ids,
                        ).await {
                            warn!(error = ?error, "could not save completed assistant message");
                            yield Ok(sse_json("error", StreamError { error: "The response was generated but could not be saved." }));
                            return;
                        }
                        schedule_automatic_memory_extraction(
                            memory_state.clone(),
                            memory_settings.clone(),
                            memory_character_id.clone(),
                            persisted_chat_id.clone(),
                        );
                        yield Ok(sse_json("done", StreamDone {}));
                        return;
                    }
                }
                Err(error) => {
                    warn!(error = ?error, "text inference streaming request failed");
                    if let Err(save_error) = persist_completed_assistant_message(
                        &database,
                        &persisted_chat_id,
                        &content,
                        &generation_id,
                        &lore_entry_ids,
                        &memory_ids,
                    ).await {
                        warn!(error = ?save_error, "could not save partial assistant message");
                    }
                    yield Ok(sse_json("error", StreamError { error: error.user_message() }));
                    return;
                }
            }
        }

        if let Err(error) = persist_completed_assistant_message(
            &database,
            &persisted_chat_id,
            &content,
            &generation_id,
            &lore_entry_ids,
            &memory_ids,
        ).await {
            warn!(error = ?error, "could not save completed assistant message");
            yield Ok(sse_json("error", StreamError { error: "The response was generated but could not be saved." }));
            return;
        }
        schedule_automatic_memory_extraction(
            memory_state.clone(),
            memory_settings.clone(),
            memory_character_id.clone(),
            persisted_chat_id.clone(),
        );
        yield Ok(sse_json("done", StreamDone {}));
    };

    Ok(Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    ))
}

async fn persist_partial_assistant_message(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
    Json(request): Json<PartialAssistantRequest>,
) -> Result<Json<PersistedMessage>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    validate_generation_id(&request.generation_id).map_err(ApiError::bad_request)?;
    let message = append_assistant_message(
        &state.database,
        &chat_id,
        &request.content,
        Some(&request.generation_id),
    )
    .await?;
    Ok(Json(message))
}

async fn change_chat_persona(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
    Json(request): Json<SetChatPersonaRequest>,
) -> Result<Json<ChatDetailResponse>, ApiError> {
    if let Some(persona_id) = request.persona_id.as_deref() {
        if get_persona(&state.database, persona_id).await?.is_none() {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "That persona is no longer available. Choose another persona."
            )));
        }
    }
    set_chat_persona(
        &state.database,
        &character_id,
        &chat_id,
        request.persona_id.as_deref(),
    )
    .await?
    .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    Ok(Json(chat_detail(&state, &character_id, &chat_id).await?))
}

async fn chat_detail(
    state: &AppState,
    character_id: &str,
    chat_id: &str,
) -> Result<ChatDetailResponse, ApiError> {
    let character = ensure_character_exists(state, character_id).await?;
    let chat = get_chat_for_character(&state.database, character_id, chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    let messages = list_chat_messages(&state.database, &chat.id).await?;
    let images = list_chat_images(&state.database, &chat.id).await?;
    let lorebooks = list_chat_lorebooks(&state.database, &chat.id).await?;
    let persona = match chat.persona_id.as_deref() {
        Some(persona_id) => get_persona(&state.database, persona_id).await?,
        None => None,
    };
    Ok(ChatDetailResponse {
        character,
        chat,
        persona,
        messages,
        images,
        lorebooks,
    })
}

async fn resolve_new_chat_persona(
    state: &AppState,
    requested_persona: Option<Option<String>>,
) -> Result<Option<Persona>, ApiError> {
    let (requested_persona, is_explicit) = match requested_persona {
        Some(persona_id) => (persona_id, true),
        None => {
            let settings = load_settings(&state.database).await?;
            match settings.persona.default_persona_id {
                Some(persona_id) => (Some(persona_id), false),
                None => {
                    let personas = list_personas(&state.database).await?;
                    ((personas.len() == 1).then(|| personas[0].id.clone()), false)
                }
            }
        }
    };
    let Some(persona_id) = requested_persona else {
        return Ok(None);
    };
    match get_persona(&state.database, &persona_id).await? {
        Some(persona) => Ok(Some(persona)),
        None if is_explicit => Err(ApiError::bad_request(anyhow::anyhow!(
            "That persona is no longer available. Choose another persona."
        ))),
        None => Ok(None),
    }
}

async fn ensure_character_exists(
    state: &AppState,
    character_id: &str,
) -> Result<Character, ApiError> {
    get_character(&state.database, character_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That character no longer exists."))
}

async fn ensure_lorebook_exists(state: &AppState, lorebook_id: &str) -> Result<Lorebook, ApiError> {
    get_lorebook(&state.database, lorebook_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That lorebook no longer exists."))
}

async fn persist_completed_assistant_message(
    database: &sqlx::SqlitePool,
    chat_id: &str,
    content: &str,
    generation_id: &str,
    lore_entry_ids: &[String],
    memory_ids: &[String],
) -> anyhow::Result<()> {
    if content.trim().is_empty() {
        return Ok(());
    }
    let message = append_assistant_message(database, chat_id, content, Some(generation_id)).await?;
    record_message_lore_entries(database, &message.id, lore_entry_ids).await?;
    record_message_memories(database, &message.id, memory_ids).await
}

/// Extraction deliberately happens after the response stream finishes and is
/// skipped while another text/image task owns the local generation resource.
/// It never holds up the user-visible chat response.
fn schedule_automatic_memory_extraction(
    state: AppState,
    initial_settings: AppSettings,
    character_id: String,
    chat_id: String,
) {
    if !initial_settings.memory.enabled || !initial_settings.memory.automatic_extraction {
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(700)).await;
        let Some(_activity) = state.resources.try_begin_text() else {
            return;
        };
        let settings = match load_settings(&state.database).await {
            Ok(settings) if settings.memory.enabled && settings.memory.automatic_extraction => {
                settings
            }
            Ok(_) => return,
            Err(error) => {
                warn!(error = ?error, "could not load settings for automatic memory extraction");
                return;
            }
        };
        if let Err(error) =
            extract_chat_memories(&state, &settings, &character_id, &chat_id, true).await
        {
            warn!(error = ?error, chat_id = %chat_id, "automatic semantic memory extraction failed");
        }
    });
}

fn validate_generation_id(generation_id: &str) -> anyhow::Result<()> {
    if uuid::Uuid::parse_str(generation_id).is_err() {
        anyhow::bail!("Generation ID is invalid.");
    }
    Ok(())
}

async fn list_existing_memories(
    State(state): State<AppState>,
    Query(query): Query<MemoryListQuery>,
) -> Result<Json<Vec<MemoryEntry>>, ApiError> {
    Ok(Json(
        list_memories(
            &state.database,
            query.character_id.as_deref(),
            query.chat_id.as_deref(),
            query.search.as_deref(),
        )
        .await?,
    ))
}

async fn get_existing_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MemoryEntry>, ApiError> {
    get_memory(&state.database, &id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That memory no longer exists."))
}

async fn create_new_memory(
    State(state): State<AppState>,
    Json(request): Json<MemoryCreateRequest>,
) -> Result<(StatusCode, Json<MemoryEntry>), ApiError> {
    ensure_character_exists(&state, &request.character_id).await?;
    let draft = request
        .draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    let settings = load_settings(&state.database).await?;
    let model = configured_embedding_model(&settings)?;
    let _activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Another AI generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    let vector = state
        .providers
        .embed(
            &settings.memory.embedding_provider,
            &effective_embedding_url(&settings, &state),
            &model,
            &draft.content,
        )
        .await
        .map_err(ApiError::provider)?;
    let memory = create_memory_with_vector(
        &state.database,
        &request.character_id,
        draft,
        true,
        &model,
        &vector,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok((StatusCode::CREATED, Json(memory)))
}

async fn update_existing_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(draft): Json<MemoryDraft>,
) -> Result<Json<MemoryEntry>, ApiError> {
    get_memory(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That memory no longer exists."))?;
    let draft = draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    let settings = load_settings(&state.database).await?;
    let model = configured_embedding_model(&settings)?;
    let _activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Another AI generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    let vector = state
        .providers
        .embed(
            &settings.memory.embedding_provider,
            &effective_embedding_url(&settings, &state),
            &model,
            &draft.content,
        )
        .await
        .map_err(ApiError::provider)?;
    update_memory_with_vector(&state.database, &id, draft, &model, &vector)
        .await
        .map_err(ApiError::bad_request)?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That memory no longer exists."))
}

async fn delete_existing_memory(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    delete_memory(&state.database, &id)
        .await?
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::not_found("That memory no longer exists."))
}

async fn memory_status(
    State(state): State<AppState>,
) -> Result<Json<MemoryStatusResponse>, ApiError> {
    let settings = load_settings(&state.database).await?;
    Ok(Json(MemoryStatusResponse {
        total_memories: count_memories(&state.database).await?,
        enabled: settings.memory.enabled,
        embedding_model: settings.memory.embedding_model,
    }))
}

async fn rebuild_memories(
    State(state): State<AppState>,
) -> Result<Json<crate::app::MemoryReindexResult>, ApiError> {
    let settings = load_settings(&state.database).await?;
    let _activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Another AI generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    rebuild_memory_index(&state, &settings)
        .await
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn extract_memories_for_chat(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<Json<crate::app::MemoryExtractionResult>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    let settings = load_settings(&state.database).await?;
    let _activity = state.resources.try_begin_text().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Another AI generation is active. Wait for it to finish or stop it first."
        ))
    })?;
    extract_chat_memories(&state, &settings, &character_id, &chat_id, false)
        .await
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn list_message_context(
    State(state): State<AppState>,
    Path((character_id, chat_id, message_id)): Path<(String, String, String)>,
) -> Result<Json<MessageContextResponse>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    if !list_chat_messages(&state.database, &chat_id)
        .await?
        .iter()
        .any(|message| message.id == message_id)
    {
        return Err(ApiError::not_found(
            "That message does not belong to this chat.",
        ));
    }
    Ok(Json(MessageContextResponse {
        lore: list_message_lore_entries(&state.database, &message_id).await?,
        memories: list_message_memories(&state.database, &message_id).await?,
    }))
}

fn configured_embedding_model(settings: &AppSettings) -> Result<String, ApiError> {
    if !settings.memory.enabled {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Enable Semantic Memory in Settings before managing memories."
        )));
    }
    settings.memory.embedding_model.clone().ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!(
            "Choose an Embedding Model in Settings before managing memories."
        ))
    })
}

async fn list_character_summaries(
    State(state): State<AppState>,
) -> Result<Json<Vec<CharacterSummary>>, ApiError> {
    let characters = list_characters(&state.database).await?;
    Ok(Json(
        characters.iter().map(CharacterSummary::from).collect(),
    ))
}

async fn list_existing_personas(
    State(state): State<AppState>,
) -> Result<Json<Vec<Persona>>, ApiError> {
    Ok(Json(list_personas(&state.database).await?))
}

async fn list_existing_lorebooks(
    State(state): State<AppState>,
) -> Result<Json<Vec<Lorebook>>, ApiError> {
    Ok(Json(list_lorebooks(&state.database).await?))
}

async fn get_existing_lorebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Lorebook>, ApiError> {
    get_lorebook(&state.database, &id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That lorebook no longer exists."))
}

async fn create_new_lorebook(
    State(state): State<AppState>,
    Json(draft): Json<LorebookDraft>,
) -> Result<(StatusCode, Json<Lorebook>), ApiError> {
    Ok((
        StatusCode::CREATED,
        Json(
            create_lorebook(&state.database, draft)
                .await
                .map_err(ApiError::bad_request)?,
        ),
    ))
}

async fn update_existing_lorebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(draft): Json<LorebookDraft>,
) -> Result<Json<Lorebook>, ApiError> {
    update_lorebook(&state.database, &id, draft)
        .await
        .map_err(ApiError::bad_request)?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That lorebook no longer exists."))
}

async fn delete_existing_lorebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    delete_lorebook(&state.database, &id)
        .await?
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::not_found("That lorebook no longer exists."))
}

async fn duplicate_existing_lorebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Lorebook>), ApiError> {
    duplicate_lorebook(&state.database, &id)
        .await?
        .map(|book| (StatusCode::CREATED, Json(book)))
        .ok_or_else(|| ApiError::not_found("That lorebook no longer exists."))
}

async fn export_lorebook(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<LorebookExport>, ApiError> {
    let lorebook = ensure_lorebook_exists(&state, &id).await?;
    let entries = list_lorebook_entries(&state.database, &id).await?;
    Ok(Json(LorebookExport {
        format: "nyxai-lorebook".to_owned(),
        version: 1,
        lorebook: LorebookDraft {
            name: lorebook.name,
            description: lorebook.description,
            enabled: lorebook.enabled,
        },
        entries: entries.into_iter().map(LorebookEntryDraft::from).collect(),
    }))
}

async fn import_lorebook(
    State(state): State<AppState>,
    Json(mut export): Json<LorebookExport>,
) -> Result<(StatusCode, Json<Lorebook>), ApiError> {
    if export.format != "nyxai-lorebook" || export.version != 1 {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Unsupported lorebook import format."
        )));
    }
    let existing = list_lorebooks(&state.database).await?;
    export.lorebook.name = duplicate_safe_lorebook_name(&export.lorebook.name, &existing);
    // Validate the entire imported structure before creating anything. This
    // prevents a malformed entry from leaving a half-imported lorebook behind.
    let entries = export
        .entries
        .into_iter()
        .map(LorebookEntryDraft::normalize_and_validate)
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(ApiError::bad_request)?;
    let lorebook = create_lorebook(&state.database, export.lorebook)
        .await
        .map_err(ApiError::bad_request)?;
    for entry in entries {
        create_lorebook_entry(&state.database, &lorebook.id, entry)
            .await
            .map_err(ApiError::bad_request)?;
    }
    Ok((StatusCode::CREATED, Json(lorebook)))
}

fn duplicate_safe_lorebook_name(source: &str, existing: &[Lorebook]) -> String {
    let base = if source.trim().is_empty() {
        "Imported Lorebook"
    } else {
        source.trim()
    };
    if !existing
        .iter()
        .any(|book| book.name.eq_ignore_ascii_case(base))
    {
        return base.to_owned();
    }
    for suffix in 2..10_000 {
        let candidate = format!("{base} ({suffix})");
        if !existing
            .iter()
            .any(|book| book.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    format!("{base} Copy")
}

async fn list_existing_lore_entries(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<LorebookEntry>>, ApiError> {
    ensure_lorebook_exists(&state, &id).await?;
    Ok(Json(list_lorebook_entries(&state.database, &id).await?))
}

async fn get_existing_lore_entry(
    State(state): State<AppState>,
    Path((lorebook_id, entry_id)): Path<(String, String)>,
) -> Result<Json<LorebookEntry>, ApiError> {
    get_lorebook_entry(&state.database, &lorebook_id, &entry_id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That lore entry no longer exists."))
}

async fn create_new_lore_entry(
    State(state): State<AppState>,
    Path(lorebook_id): Path<String>,
    Json(draft): Json<LorebookEntryDraft>,
) -> Result<(StatusCode, Json<LorebookEntry>), ApiError> {
    create_lorebook_entry(&state.database, &lorebook_id, draft)
        .await
        .map_err(ApiError::bad_request)?
        .map(|entry| (StatusCode::CREATED, Json(entry)))
        .ok_or_else(|| ApiError::not_found("That lorebook no longer exists."))
}

async fn update_existing_lore_entry(
    State(state): State<AppState>,
    Path((lorebook_id, entry_id)): Path<(String, String)>,
    Json(draft): Json<LorebookEntryDraft>,
) -> Result<Json<LorebookEntry>, ApiError> {
    update_lorebook_entry(&state.database, &lorebook_id, &entry_id, draft)
        .await
        .map_err(ApiError::bad_request)?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That lore entry no longer exists."))
}

async fn delete_existing_lore_entry(
    State(state): State<AppState>,
    Path((lorebook_id, entry_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    delete_lorebook_entry(&state.database, &lorebook_id, &entry_id)
        .await?
        .map(|_| StatusCode::NO_CONTENT)
        .ok_or_else(|| ApiError::not_found("That lore entry no longer exists."))
}

async fn list_character_lorebook_associations(
    State(state): State<AppState>,
    Path(character_id): Path<String>,
) -> Result<Json<Vec<Lorebook>>, ApiError> {
    ensure_character_exists(&state, &character_id).await?;
    Ok(Json(
        list_character_lorebooks(&state.database, &character_id).await?,
    ))
}

async fn set_character_lorebook_associations(
    State(state): State<AppState>,
    Path(character_id): Path<String>,
    Json(request): Json<LorebookAssociationRequest>,
) -> Result<Json<Vec<Lorebook>>, ApiError> {
    ensure_character_exists(&state, &character_id).await?;
    set_character_lorebooks(&state.database, &character_id, &request.lorebook_ids)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(
        list_character_lorebooks(&state.database, &character_id).await?,
    ))
}

async fn list_chat_lorebook_associations(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
) -> Result<Json<Vec<Lorebook>>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    Ok(Json(list_chat_lorebooks(&state.database, &chat_id).await?))
}

async fn set_chat_lorebook_associations(
    State(state): State<AppState>,
    Path((character_id, chat_id)): Path<(String, String)>,
    Json(request): Json<LorebookAssociationRequest>,
) -> Result<Json<Vec<Lorebook>>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    set_chat_lorebooks(&state.database, &chat_id, &request.lorebook_ids)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Json(list_chat_lorebooks(&state.database, &chat_id).await?))
}

async fn list_message_lore_context(
    State(state): State<AppState>,
    Path((character_id, chat_id, message_id)): Path<(String, String, String)>,
) -> Result<Json<Vec<LorebookEntry>>, ApiError> {
    get_chat_for_character(&state.database, &character_id, &chat_id)
        .await?
        .ok_or_else(|| ApiError::not_found("That chat does not belong to this character."))?;
    let messages = list_chat_messages(&state.database, &chat_id).await?;
    if !messages.iter().any(|message| message.id == message_id) {
        return Err(ApiError::not_found(
            "That message does not belong to this chat.",
        ));
    }
    Ok(Json(
        list_message_lore_entries(&state.database, &message_id).await?,
    ))
}

async fn get_existing_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Persona>, ApiError> {
    get_persona(&state.database, &id)
        .await?
        .map(Json)
        .ok_or_else(|| ApiError::not_found("That persona no longer exists."))
}

async fn create_new_persona(
    State(state): State<AppState>,
    Json(draft): Json<PersonaDraft>,
) -> Result<(StatusCode, Json<Persona>), ApiError> {
    let draft = draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    validate_avatar_path(&state, draft.avatar_path.as_deref()).await?;
    let persona = create_persona(&state.database, draft).await?;
    Ok((StatusCode::CREATED, Json(persona)))
}

async fn update_existing_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(draft): Json<PersonaDraft>,
) -> Result<Json<Persona>, ApiError> {
    let previous = get_persona(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That persona no longer exists."))?;
    let draft = draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    validate_avatar_path(&state, draft.avatar_path.as_deref()).await?;
    let persona = update_persona(&state.database, &id, draft)
        .await?
        .ok_or_else(|| ApiError::not_found("That persona no longer exists."))?;
    remove_replaced_avatar(&state, previous.avatar_path, persona.avatar_path.clone()).await;
    Ok(Json(persona))
}

async fn duplicate_existing_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Persona>), ApiError> {
    let source = get_persona(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That persona no longer exists."))?;
    let copied_avatar_path = match source.avatar_path.as_deref() {
        Some(avatar_path) => Some(
            copy_avatar(&state.avatar_directory, avatar_path)
                .await
                .map_err(ApiError::avatar)?,
        ),
        None => None,
    };

    match duplicate_persona(&state.database, &id, copied_avatar_path.clone()).await? {
        Some(persona) => Ok((StatusCode::CREATED, Json(persona))),
        None => {
            if let Some(avatar_path) = copied_avatar_path {
                if let Err(error) = remove_avatar(&state.avatar_directory, &avatar_path).await {
                    warn!(error = ?error, avatar = %avatar_path, "could not remove orphaned duplicate persona avatar");
                }
            }
            Err(ApiError::not_found("That persona no longer exists."))
        }
    }
}

async fn delete_existing_persona(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PersonaDeletionResponse>, ApiError> {
    let chats_reassigned = count_chats_for_persona(&state.database, &id).await?;
    let persona = delete_persona(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That persona no longer exists."))?;

    let mut settings = load_settings(&state.database).await?;
    if settings.persona.default_persona_id.as_deref() == Some(&persona.id) {
        settings.persona.default_persona_id = None;
        save_settings(&state.database, &settings).await?;
    }
    if let Some(avatar_path) = persona.avatar_path {
        if let Err(error) = remove_avatar(&state.avatar_directory, &avatar_path).await {
            warn!(error = ?error, avatar = %avatar_path, "could not remove deleted persona avatar");
        }
    }
    Ok(Json(PersonaDeletionResponse { chats_reassigned }))
}

async fn get_existing_character(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Character>, ApiError> {
    let character = get_character(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That character no longer exists."))?;
    Ok(Json(character))
}

async fn create_new_character(
    State(state): State<AppState>,
    Json(draft): Json<CharacterDraft>,
) -> Result<(StatusCode, Json<Character>), ApiError> {
    let draft = draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    validate_draft_avatar(&state, &draft).await?;
    let character = create_character(&state.database, draft).await?;
    Ok((StatusCode::CREATED, Json(character)))
}

async fn import_character_card(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<CharacterImportResponse>), ApiError> {
    let mut card_bytes = None;
    while let Some(field) = multipart.next_field().await.map_err(|error| {
        warn!(error = ?error, "could not read character card upload");
        ApiError::bad_request(anyhow::anyhow!("Character card could not be read."))
    })? {
        if field.name() != Some("card") {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Choose a JSON or PNG character card."
            )));
        }
        if card_bytes.is_some() {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Upload only one character card at a time."
            )));
        }
        card_bytes = Some(field.bytes().await.map_err(|error| {
            warn!(error = ?error, "could not read character card bytes");
            ApiError::bad_request(anyhow::anyhow!("Character card could not be read."))
        })?);
    }

    let card_bytes = card_bytes.ok_or_else(|| {
        ApiError::bad_request(anyhow::anyhow!("Choose a JSON or PNG character card."))
    })?;
    let mut imported = parse_character_card(&card_bytes).map_err(|error| {
        warn!(error = ?error, "character card import was rejected");
        ApiError::bad_request(anyhow::anyhow!(error.user_message()))
    })?;
    info!(format = ?imported.format, "normalizing imported character card");

    let existing = list_characters(&state.database).await?;
    imported.draft.name = unique_import_name(&imported.draft.name, &existing);
    let stored_avatar = match imported.avatar_bytes.as_deref() {
        Some(bytes) => Some(
            store_avatar(&state.avatar_directory, bytes)
                .await
                .map_err(ApiError::avatar)?,
        ),
        None => None,
    };
    if let Some(avatar_path) = &stored_avatar {
        imported.draft.avatar_path = Some(avatar_path.clone());
    }
    let warnings = imported.warnings;

    match create_character(&state.database, imported.draft).await {
        Ok(character) => Ok((
            StatusCode::CREATED,
            Json(CharacterImportResponse {
                character,
                warnings,
            }),
        )),
        Err(error) => {
            if let Some(avatar_path) = stored_avatar {
                if let Err(cleanup_error) =
                    remove_avatar(&state.avatar_directory, &avatar_path).await
                {
                    warn!(error = ?cleanup_error, avatar = %avatar_path, "could not remove orphaned imported avatar");
                }
            }
            Err(error.into())
        }
    }
}

fn unique_import_name(proposed_name: &str, existing: &[Character]) -> String {
    let base = proposed_name.trim();
    if !existing
        .iter()
        .any(|character| character.name.eq_ignore_ascii_case(base))
    {
        return base.to_owned();
    }

    for suffix in 2..=9_999 {
        let ending = format!(" ({suffix})");
        let prefix: String = base.chars().take(120 - ending.chars().count()).collect();
        let candidate = format!("{prefix}{ending}");
        if !existing
            .iter()
            .any(|character| character.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }

    format!("{} (import)", base.chars().take(111).collect::<String>())
}

async fn update_existing_character(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(draft): Json<CharacterDraft>,
) -> Result<Json<Character>, ApiError> {
    let previous = get_character(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That character no longer exists."))?;
    let draft = draft
        .normalize_and_validate()
        .map_err(ApiError::bad_request)?;
    validate_draft_avatar(&state, &draft).await?;
    let character = update_character(&state.database, &id, draft)
        .await?
        .ok_or_else(|| ApiError::not_found("That character no longer exists."))?;

    remove_replaced_avatar(&state, previous.avatar_path, character.avatar_path.clone()).await;

    Ok(Json(character))
}

async fn delete_existing_character(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    // SQLite cascades the metadata. Collect file references first so the
    // corresponding generated image assets do not become orphaned on disk.
    let chats = list_chats_for_character(&state.database, &id).await?;
    let mut generated_images = Vec::new();
    for chat in chats {
        generated_images.extend(list_chat_images(&state.database, &chat.id).await?);
    }
    let character = delete_character(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That character no longer exists."))?;

    if let Some(avatar_path) = character.avatar_path {
        if let Err(error) = remove_avatar(&state.avatar_directory, &avatar_path).await {
            warn!(error = ?error, avatar = %avatar_path, "could not remove deleted character avatar");
        }
    }
    for image in generated_images {
        let reference = image.image_url.trim_start_matches("/images/");
        if let Err(error) = remove_chat_image_file(&state.image_directory, reference).await {
            warn!(error = ?error, image = %reference, "could not remove deleted character image file");
        }
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn duplicate_existing_character(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<(StatusCode, Json<Character>), ApiError> {
    let source = get_character(&state.database, &id)
        .await?
        .ok_or_else(|| ApiError::not_found("That character no longer exists."))?;
    let copied_avatar_path = match source.avatar_path.as_deref() {
        Some(avatar_path) => Some(
            copy_avatar(&state.avatar_directory, avatar_path)
                .await
                .map_err(ApiError::avatar)?,
        ),
        None => None,
    };

    match duplicate_character(&state.database, &id, copied_avatar_path.clone()).await? {
        Some(character) => Ok((StatusCode::CREATED, Json(character))),
        None => {
            if let Some(avatar_path) = copied_avatar_path {
                if let Err(error) = remove_avatar(&state.avatar_directory, &avatar_path).await {
                    warn!(error = ?error, avatar = %avatar_path, "could not remove orphaned duplicate avatar");
                }
            }
            Err(ApiError::not_found("That character no longer exists."))
        }
    }
}

async fn upload_avatar(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<AvatarUploadResponse>), ApiError> {
    let mut avatar_bytes = None;
    while let Some(field) = multipart.next_field().await.map_err(|error| {
        warn!(error = ?error, "could not read avatar upload");
        ApiError::bad_request(anyhow::anyhow!("Avatar upload could not be read."))
    })? {
        if field.name() != Some("avatar") {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Upload an avatar image."
            )));
        }
        if avatar_bytes.is_some() {
            return Err(ApiError::bad_request(anyhow::anyhow!(
                "Upload only one avatar image."
            )));
        }
        avatar_bytes = Some(field.bytes().await.map_err(|error| {
            warn!(error = ?error, "could not read avatar upload bytes");
            ApiError::bad_request(anyhow::anyhow!("Avatar upload could not be read."))
        })?);
    }

    let avatar_path = store_avatar(
        &state.avatar_directory,
        &avatar_bytes
            .ok_or_else(|| ApiError::bad_request(anyhow::anyhow!("Choose an avatar image.")))?,
    )
    .await
    .map_err(ApiError::avatar)?;
    let avatar_url =
        crate::storage::avatar_url(&avatar_path).expect("a generated avatar path must be safe");

    Ok((
        StatusCode::CREATED,
        Json(AvatarUploadResponse {
            avatar_path,
            avatar_url,
        }),
    ))
}

async fn validate_draft_avatar(state: &AppState, draft: &CharacterDraft) -> Result<(), ApiError> {
    validate_avatar_path(state, draft.avatar_path.as_deref()).await
}

async fn validate_avatar_path(state: &AppState, avatar_path: Option<&str>) -> Result<(), ApiError> {
    if let Some(avatar_path) = avatar_path {
        ensure_avatar_exists(&state.avatar_directory, avatar_path)
            .await
            .map_err(ApiError::avatar)?;
    }
    Ok(())
}

async fn remove_replaced_avatar(
    state: &AppState,
    previous_avatar_path: Option<String>,
    current_avatar_path: Option<String>,
) {
    if previous_avatar_path == current_avatar_path {
        return;
    }
    if let Some(avatar_path) = previous_avatar_path {
        if let Err(error) = remove_avatar(&state.avatar_directory, &avatar_path).await {
            warn!(error = ?error, avatar = %avatar_path, "could not remove replaced avatar");
        }
    }
}

fn effective_provider_url(
    settings: &AppSettings,
    state: &AppState,
    provider: &ProviderKind,
) -> String {
    match provider {
        ProviderKind::Ollama => settings
            .provider
            .ollama_base_url
            .clone()
            .unwrap_or_else(|| state.default_ollama_base_url.clone()),
        ProviderKind::OpenaiCompatible => settings
            .provider
            .openai_compatible_base_url
            .clone()
            .unwrap_or_else(|| state.default_openai_compatible_base_url.clone()),
    }
}

fn effective_text_url(settings: &AppSettings, state: &AppState) -> String {
    effective_provider_url(settings, state, &settings.provider.active_provider)
}

fn effective_embedding_url(settings: &AppSettings, state: &AppState) -> String {
    effective_provider_url(settings, state, &settings.memory.embedding_provider)
}

fn effective_a1111_url(settings: &AppSettings, state: &AppState) -> String {
    settings
        .image
        .a1111_base_url
        .clone()
        .unwrap_or_else(|| state.default_a1111_base_url.clone())
}

async fn generate_visual_prompt(
    state: &AppState,
    settings: &AppSettings,
    messages: Vec<ConversationMessage>,
) -> Result<String, ApiError> {
    if !settings.image.enabled {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Enable Image Generation in Settings first."
        )));
    }
    let model = settings
        .image
        .image_prompt_model
        .clone()
        .or_else(|| settings.provider.character_creator_model.clone())
        .or_else(|| settings.provider.selected_model.clone())
        .ok_or_else(|| {
            ApiError::bad_request(anyhow::anyhow!(
                "Choose an Image Prompt Model, Character Creator Model, or chat model first."
            ))
        })?;
    let output = state
        .providers
        .generate_text(
            &settings.provider.active_provider,
            &effective_text_url(settings, state),
            &model,
            messages,
            settings.generation.clone(),
        )
        .await
        .map_err(ApiError::provider)?;
    normalize_visual_prompt(&output).map_err(ApiError::creator_output)
}

enum ImageDimensions {
    Avatar,
    Chat,
}

async fn generate_local_image(
    state: &AppState,
    settings: &AppSettings,
    prompt: String,
    kind: ImageDimensions,
) -> Result<crate::providers::GeneratedImage, ApiError> {
    if !settings.image.enabled {
        return Err(ApiError::bad_request(anyhow::anyhow!(
            "Enable Image Generation in Settings first."
        )));
    }
    let base_url = effective_a1111_url(settings, state);
    let text_lifecycle_supported = state
        .providers
        .capabilities(&settings.provider.active_provider)
        .model_lifecycle;
    if settings.image.single_gpu_memory_mode && text_lifecycle_supported {
        if let Some(model) = settings
            .image
            .image_prompt_model
            .as_ref()
            .or(settings.provider.character_creator_model.as_ref())
            .or(settings.provider.selected_model.as_ref())
        {
            if let Err(error) = state
                .providers
                .unload_model(
                    &settings.provider.active_provider,
                    &effective_text_url(settings, state),
                    model,
                )
                .await
            {
                warn!(error = ?error, "could not unload text model before image generation");
            }
        }
    } else if settings.image.single_gpu_memory_mode {
        info!("configured text backend has no model lifecycle API; skipping text-model unload before image generation");
    }
    let (width, height) = match kind {
        ImageDimensions::Avatar => (settings.image.avatar_width, settings.image.avatar_height),
        ImageDimensions::Chat => (settings.image.chat_width, settings.image.chat_height),
    };
    let generated = state
        .image_providers
        .generate(
            &base_url,
            ImageGenerationRequest {
                prompt,
                negative_prompt: settings.image.negative_prompt.clone(),
                width,
                height,
                steps: settings.image.steps,
                cfg_scale: settings.image.cfg_scale,
                sampler_name: settings.image.sampler_name.clone(),
                model: settings.image.selected_model.clone(),
            },
        )
        .await
        .map_err(ApiError::image_provider)?;
    if settings.image.single_gpu_memory_mode {
        if let Err(error) = state.image_providers.release_memory(&base_url).await {
            warn!(error = ?error, "image generation succeeded but A1111 cleanup was unavailable");
        }
        if settings.image.restore_text_model_after_generation && text_lifecycle_supported {
            if let Some(model) = settings.provider.selected_model.as_deref() {
                if let Err(error) = state
                    .providers
                    .preload_model(
                        &settings.provider.active_provider,
                        &effective_text_url(settings, state),
                        model,
                    )
                    .await
                {
                    warn!(error = ?error, "image generation succeeded but chat model restore failed");
                }
            }
        } else if settings.image.restore_text_model_after_generation {
            info!("configured text backend has no model lifecycle API; skipping text-model preload after image generation");
        }
    }
    Ok(generated)
}

fn settings_response(settings: &AppSettings, state: &AppState) -> SettingsResponse {
    SettingsResponse {
        appearance: settings.appearance.clone(),
        provider: settings.provider.clone(),
        generation: settings.generation.clone(),
        persona: settings.persona.clone(),
        image: settings.image.clone(),
        memory: settings.memory.clone(),
        effective_ollama_base_url: effective_provider_url(settings, state, &ProviderKind::Ollama),
        effective_openai_compatible_base_url: effective_provider_url(
            settings,
            state,
            &ProviderKind::OpenaiCompatible,
        ),
        effective_text_base_url: effective_text_url(settings, state),
        effective_a1111_base_url: effective_a1111_url(settings, state),
        capabilities: state
            .providers
            .capabilities(&settings.provider.active_provider),
    }
}

fn sse_json<T: Serialize>(event: &str, value: T) -> Event {
    let json = serde_json::to_string(&value).expect("SSE response types must serialize");
    Event::default().event(event).data(json)
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize)]
struct AboutResponse {
    name: &'static str,
    version: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct SettingsResponse {
    appearance: crate::domain::settings::AppearanceSettings,
    provider: crate::domain::settings::ProviderSettings,
    generation: crate::domain::settings::GenerationSettings,
    persona: crate::domain::settings::PersonaSettings,
    image: crate::domain::settings::ImageSettings,
    memory: crate::domain::settings::MemorySettings,
    effective_ollama_base_url: String,
    effective_openai_compatible_base_url: String,
    effective_text_base_url: String,
    effective_a1111_base_url: String,
    capabilities: crate::providers::ProviderCapabilities,
}

#[derive(Serialize)]
struct ConnectionTestResponse {
    connected: bool,
    model_count: usize,
    message: String,
}

#[derive(Serialize)]
struct ImageConnectionTestResponse {
    connected: bool,
    model_count: Option<usize>,
    message: String,
}

#[derive(Serialize)]
struct ImageModelsResponse {
    models: Vec<String>,
    selected_model: Option<String>,
    selected_model_available: bool,
}

#[derive(Serialize)]
struct AvatarCandidateResponse {
    avatar_path: String,
    avatar_url: String,
    prompt: String,
}

#[derive(Serialize)]
struct ModelsResponse {
    models: Vec<ProviderModel>,
    selected_model: Option<String>,
    selected_model_available: bool,
}

#[derive(Deserialize)]
struct SelectModelRequest {
    model: String,
}

#[derive(Deserialize)]
struct CharacterCreatorGenerateRequest {
    prompt: String,
    alternate_greeting_count: u8,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Deserialize)]
struct CharacterCreatorRegenerateRequest {
    field: CharacterCreatorField,
    draft: CharacterDraft,
    original_prompt: String,
    #[serde(default)]
    instruction: String,
    alternate_greeting_count: u8,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Serialize)]
struct CharacterCreatorDraftResponse {
    draft: CharacterDraft,
}

#[derive(Serialize)]
struct ChatDetailResponse {
    character: Character,
    chat: Chat,
    persona: Option<Persona>,
    messages: Vec<PersistedMessage>,
    images: Vec<ChatImage>,
    lorebooks: Vec<Lorebook>,
}

#[derive(Deserialize)]
struct SetChatPersonaRequest {
    #[serde(default)]
    persona_id: Option<String>,
}

#[derive(Deserialize)]
struct LorebookAssociationRequest {
    #[serde(default)]
    lorebook_ids: Vec<String>,
}

#[derive(Deserialize)]
struct MemoryListQuery {
    #[serde(default)]
    character_id: Option<String>,
    #[serde(default)]
    chat_id: Option<String>,
    #[serde(default)]
    search: Option<String>,
}

#[derive(Deserialize)]
struct MemoryCreateRequest {
    character_id: String,
    #[serde(flatten)]
    draft: MemoryDraft,
}

#[derive(Serialize)]
struct MemoryStatusResponse {
    total_memories: i64,
    enabled: bool,
    embedding_model: Option<String>,
}

#[derive(Serialize)]
struct MessageContextResponse {
    lore: Vec<LorebookEntry>,
    memories: Vec<MemoryEntry>,
}

/// The intentionally small, versioned interchange format for exported world
/// information. It contains no database IDs or attachment information, so an
/// imported lorebook is always independent of the source installation.
#[derive(Deserialize, Serialize)]
struct LorebookExport {
    format: String,
    version: u32,
    lorebook: LorebookDraft,
    #[serde(default)]
    entries: Vec<LorebookEntryDraft>,
}

#[derive(Serialize)]
struct PersonaDeletionResponse {
    chats_reassigned: i64,
}

#[derive(Deserialize)]
struct GenerateChatRequest {
    content: String,
    generation_id: String,
}

#[derive(Deserialize)]
struct PartialAssistantRequest {
    content: String,
    generation_id: String,
}

#[derive(Serialize)]
struct StreamDelta {
    content: String,
}

#[derive(Serialize)]
struct StreamContext {
    lore_used: Vec<String>,
    memories_used: Vec<String>,
}

#[derive(Serialize)]
struct StreamDone {}

#[derive(Serialize)]
struct StreamError {
    error: &'static str,
}

#[derive(Serialize)]
struct AvatarUploadResponse {
    avatar_path: String,
    avatar_url: String,
}

#[derive(Serialize)]
struct CharacterImportResponse {
    character: Character,
    warnings: Vec<String>,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: error.to_string(),
        }
    }

    fn not_found(message: &str) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.to_owned(),
        }
    }

    fn avatar(error: AvatarStorageError) -> Self {
        match error {
            AvatarStorageError::InvalidImage
            | AvatarStorageError::TooLarge
            | AvatarStorageError::InvalidReference => Self {
                status: StatusCode::BAD_REQUEST,
                message: error.user_message().to_owned(),
            },
            error @ AvatarStorageError::Storage(_) => {
                warn!(error = ?error, "avatar storage request failed");
                Self::internal(anyhow::anyhow!(error))
            }
        }
    }

    fn provider(error: ProviderError) -> Self {
        warn!(error = ?error, "AI provider request failed");
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: error.user_message().to_owned(),
        }
    }

    fn image_provider(error: ImageGenerationError) -> Self {
        warn!(error = ?error, "image provider request failed");
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: error.user_message().to_owned(),
        }
    }

    fn creator_output(error: anyhow::Error) -> Self {
        warn!(error = ?error, "character creator output could not be validated");
        Self {
            status: StatusCode::BAD_GATEWAY,
            message: "The Character Creator returned an invalid draft. Try again or choose another local model.".to_owned(),
        }
    }

    fn internal(error: anyhow::Error) -> Self {
        error!(error = ?error, "API request failed");
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "NyxAI could not complete that request. Please try again.".to_owned(),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(error: anyhow::Error) -> Self {
        Self::internal(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[tokio::test]
    async fn environment_url_is_used_when_no_url_has_been_saved() {
        let settings = AppSettings::default();
        let state = AppState::new(
            sqlx::SqlitePool::connect_lazy("sqlite::memory:").expect("pool"),
            crate::providers::ProviderService::new(None).expect("provider service"),
            crate::providers::ImageProviderService::new().expect("image provider service"),
            "http://from-environment:11434".to_owned(),
            "http://from-environment:8080/v1".to_owned(),
            PathBuf::from("./avatars"),
            "http://from-environment:7860".to_owned(),
            PathBuf::from("./images"),
        );

        assert_eq!(
            effective_text_url(&settings, &state),
            "http://from-environment:11434"
        );
    }

    #[tokio::test]
    async fn health_check_is_a_liveness_only_response() {
        let Json(response) = health_check().await;
        assert_eq!(response.status, "ok");
    }

    #[tokio::test]
    async fn about_uses_the_package_version_source_of_truth() {
        let Json(response) = about().await;
        assert_eq!(response.name, "NyxAI");
        assert_eq!(response.version, APP_VERSION);
    }

    #[tokio::test]
    async fn saved_url_takes_precedence_over_environment_default() {
        let mut settings = AppSettings::default();
        settings.provider.ollama_base_url = Some("http://saved-server:11434".to_owned());
        let state = AppState::new(
            sqlx::SqlitePool::connect_lazy("sqlite::memory:").expect("pool"),
            crate::providers::ProviderService::new(None).expect("provider service"),
            crate::providers::ImageProviderService::new().expect("image provider service"),
            "http://from-environment:11434".to_owned(),
            "http://from-environment:8080/v1".to_owned(),
            PathBuf::from("./avatars"),
            "http://from-environment:7860".to_owned(),
            PathBuf::from("./images"),
        );

        assert_eq!(
            effective_text_url(&settings, &state),
            "http://saved-server:11434"
        );
    }

    #[tokio::test]
    async fn text_and_embedding_backends_can_use_independent_urls() {
        let mut settings = AppSettings::default();
        settings.provider.active_provider = ProviderKind::OpenaiCompatible;
        settings.provider.openai_compatible_base_url = Some("http://text.local:8080/v1".to_owned());
        settings.memory.embedding_provider = ProviderKind::Ollama;
        settings.provider.ollama_base_url = Some("http://embeddings.local:11434".to_owned());
        let state = AppState::new(
            sqlx::SqlitePool::connect_lazy("sqlite::memory:").expect("pool"),
            crate::providers::ProviderService::new(None).expect("provider service"),
            crate::providers::ImageProviderService::new().expect("image provider service"),
            "http://from-environment:11434".to_owned(),
            "http://from-environment:8080/v1".to_owned(),
            PathBuf::from("./avatars"),
            "http://from-environment:7860".to_owned(),
            PathBuf::from("./images"),
        );

        assert_eq!(
            effective_text_url(&settings, &state),
            "http://text.local:8080/v1"
        );
        assert_eq!(
            effective_embedding_url(&settings, &state),
            "http://embeddings.local:11434"
        );
    }

    #[tokio::test]
    async fn saved_image_url_takes_precedence_over_environment_default() {
        let mut settings = AppSettings::default();
        settings.image.a1111_base_url = Some("http://saved-image-server:7860".to_owned());
        let state = AppState::new(
            sqlx::SqlitePool::connect_lazy("sqlite::memory:").expect("pool"),
            crate::providers::ProviderService::new(None).expect("provider service"),
            crate::providers::ImageProviderService::new().expect("image provider service"),
            "http://from-environment:11434".to_owned(),
            "http://from-environment:8080/v1".to_owned(),
            PathBuf::from("./avatars"),
            "http://from-environment:7860".to_owned(),
            PathBuf::from("./images"),
        );
        assert_eq!(
            effective_a1111_url(&settings, &state),
            "http://saved-image-server:7860"
        );
    }
}
