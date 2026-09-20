const defaultAppearance = {
  app_background: "#08080B", secondary_background: "#101014", user_message_background: "#4A2C63",
  user_message_text: "#F2EEF5", character_message_background: "#21172D", character_message_text: "#F2EEF5",
  accent_color: "#C9A227", muted_text: "#9A94A3",
};
const defaultProvider = { active_provider: "ollama", ollama_base_url: null, openai_compatible_base_url: null, selected_model: null, character_creator_model: null };
const defaultGeneration = { temperature: 0.8, context_length: 4096, max_response_length: 512 };
const defaultPersonaSettings = { default_persona_id: null };
const defaultImageSettings = { enabled: false, a1111_base_url: null, selected_model: null, image_prompt_model: null, single_gpu_memory_mode: false, restore_text_model_after_generation: false, avatar_width: 512, avatar_height: 512, chat_width: 768, chat_height: 512, steps: 28, cfg_scale: 7, sampler_name: null, negative_prompt: "" };
const defaultMemorySettings = { enabled: false, automatic_extraction: true, embedding_model: null, embedding_provider: "ollama", extraction_model: null, extraction_interval: 8, retrieval_count: 4, similarity_threshold: .62, context_budget: 512 };

const chatScreen = document.querySelector("#chat-screen");
const settingsScreen = document.querySelector("#settings-screen");
const charactersScreen = document.querySelector("#characters-screen");
const characterEditorScreen = document.querySelector("#character-editor-screen");
const personasScreen = document.querySelector("#personas-screen");
const personaEditorScreen = document.querySelector("#persona-editor-screen");
const importScreen = document.querySelector("#import-screen");
const lorebooksScreen = document.querySelector("#lorebooks-screen");
const lorebookEditorScreen = document.querySelector("#lorebook-editor-screen");
const loreEntryEditorScreen = document.querySelector("#lore-entry-editor-screen");
const drawer = document.querySelector("#drawer");
const backdrop = document.querySelector("#drawer-backdrop");
const networkBanner = document.querySelector("#network-banner");
const toast = document.querySelector("#toast");
const conversation = document.querySelector("#conversation");
const composer = document.querySelector("#chat-composer");
const messageInput = document.querySelector("#message-input");
const sendButton = document.querySelector("#send-message");
const stopButton = document.querySelector("#stop-generation");
const chatStatus = document.querySelector("#chat-status");
const activeCharacterName = document.querySelector("#active-character-name");
const activeChatTitle = document.querySelector("#active-chat-title");
const activePersonaName = document.querySelector("#active-persona-name");
const connectionDot = document.querySelector("#connection-dot");
const connectionForm = document.querySelector("#connection-form");
const connectionStatus = document.querySelector("#connection-status");
const modelSelect = document.querySelector("#ollama-model");
const modelStatus = document.querySelector("#model-status");
const characterCreatorModelSettings = document.querySelector("#character-creator-model");
const characterCreatorModelStatus = document.querySelector("#character-creator-model-status");
const generationForm = document.querySelector("#generation-form");
const appearanceForm = document.querySelector("#appearance-form");
const appearanceStatus = document.querySelector("#form-status");
const sidebarCharacters = document.querySelector("#sidebar-characters");
const characterCards = document.querySelector("#character-cards");
const charactersStatus = document.querySelector("#characters-status");
const characterForm = document.querySelector("#character-form");
const editorStatus = document.querySelector("#character-editor-status");
const avatarInput = document.querySelector("#avatar-input");
const avatarPreview = document.querySelector("#avatar-preview");
const avatarFallback = document.querySelector("#avatar-fallback");
const tagInput = document.querySelector("#tag-input");
const tagChips = document.querySelector("#tag-chips");
const alternateGreetings = document.querySelector("#alternate-greetings");
const personaCards = document.querySelector("#persona-cards");
const personasStatus = document.querySelector("#personas-status");
const personaForm = document.querySelector("#persona-form");
const personaEditorStatus = document.querySelector("#persona-editor-status");
const personaAvatarInput = document.querySelector("#persona-avatar-input");
const personaAvatarPreview = document.querySelector("#persona-avatar-preview");
const personaAvatarFallback = document.querySelector("#persona-avatar-fallback");
const importForm = document.querySelector("#import-character-form");
const importCardFile = document.querySelector("#import-card-file");
const importStatus = document.querySelector("#import-character-status");
const actionsDialog = document.querySelector("#character-actions-dialog");
const deleteDialog = document.querySelector("#delete-character-dialog");
const chatActionsDialog = document.querySelector("#chat-actions-dialog");
const renameChatDialog = document.querySelector("#rename-chat-dialog");
const deleteChatDialog = document.querySelector("#delete-chat-dialog");
const newChatDialog = document.querySelector("#new-chat-dialog");
const chatPersonaDialog = document.querySelector("#chat-persona-dialog");
const personaActionsDialog = document.querySelector("#persona-actions-dialog");
const deletePersonaDialog = document.querySelector("#delete-persona-dialog");
const characterCreatorDialog = document.querySelector("#character-creator-dialog");
const fieldRegenerationDialog = document.querySelector("#field-regeneration-dialog");
const generatedAvatarDialog = document.querySelector("#generated-avatar-dialog");
const imageSettingsForm = document.querySelector("#image-settings-form");
const imageSettingsStatus = document.querySelector("#image-settings-status");
const lorebookCards = document.querySelector("#lorebook-cards");
const lorebooksStatus = document.querySelector("#lorebooks-status");
const lorebookForm = document.querySelector("#lorebook-form");
const loreEntryForm = document.querySelector("#lore-entry-form");
const lorebookEditorStatus = document.querySelector("#lorebook-editor-status");
const loreEntryEditorStatus = document.querySelector("#lore-entry-editor-status");
const lorebookAssociationDialog = document.querySelector("#lorebook-association-dialog");
const lorebookActionsDialog = document.querySelector("#lorebook-actions-dialog");
const memoryDialog = document.querySelector("#memory-dialog");
const memoryEditorDialog = document.querySelector("#memory-editor-dialog");
const memorySettingsForm = document.querySelector("#memory-settings-form");

let appSettings = { appearance: { ...defaultAppearance }, provider: { ...defaultProvider }, generation: { ...defaultGeneration }, persona: { ...defaultPersonaSettings }, image: { ...defaultImageSettings }, memory: { ...defaultMemorySettings } };
let characters = [];
let personas = [];
let activeCharacter = null;
let activeChat = null;
let activePersona = null;
let activeMessages = [];
let activeImages = [];
let activeController = null;
let isGenerating = false;
let chatLists = new Map();
let expandedCharacters = new Set();
let editorTags = [];
let editorAlternateGreetings = [];
let editorAvatarUrl = null;
let editorDirty = false;
let actionCharacterId = null;
let deleteCharacterId = null;
let chatActionTarget = null;
let personaActionId = null;
let deletePersonaId = null;
let personaEditorAvatarUrl = null;
let personaEditorDirty = false;
let newChatCharacter = null;
let toastTimer = null;
let drawerOpener = null;
let availableModels = [];
let availableEmbeddingModels = [];
let creatorOriginalPrompt = "";
let creatorAlternateGreetingCount = 3;
let creatorModelOverride = null;
let creatorFieldTarget = null;
let isCharacterCreatorGenerating = false;
let imageGenerationController = null;
let isImageGenerating = false;
let generatedAvatarCandidate = null;
let lorebooks = [];
let activeLorebook = null;
let lorebookAssociationTarget = null;
let lorebookActionBook = null;
let memories = [];
let editingMemory = null;

function showScreen(screen) { [chatScreen, settingsScreen, charactersScreen, characterEditorScreen, personasScreen, personaEditorScreen, importScreen, lorebooksScreen, lorebookEditorScreen, loreEntryEditorScreen].forEach((item) => { item.hidden = item !== screen; }); window.scrollTo({ top: 0, behavior: "auto" }); }
function openDrawer() { drawerOpener = document.activeElement; drawer.classList.add("is-open"); drawer.setAttribute("aria-hidden", "false"); backdrop.hidden = false; document.body.classList.add("drawer-open"); window.requestAnimationFrame(() => document.querySelector("#close-drawer").focus()); }
function closeDrawer({ restoreFocus = true } = {}) { drawer.classList.remove("is-open"); drawer.setAttribute("aria-hidden", "true"); backdrop.hidden = true; document.body.classList.remove("drawer-open"); if (restoreFocus && drawerOpener && drawerOpener.isConnected) drawerOpener.focus(); drawerOpener = null; }
function showToast(message) { toast.textContent = message; toast.hidden = false; clearTimeout(toastTimer); toastTimer = window.setTimeout(() => { toast.hidden = true; }, 2800); }
function showStatus(target, message = "", type = "") { target.textContent = message; target.className = "form-status" + (type ? " is-" + type : ""); }
function showChatStatus(message = "", type = "") { chatStatus.textContent = message; chatStatus.hidden = !message; chatStatus.className = "chat-status" + (type ? " is-" + type : ""); }
function closeDialog(dialog) { if (dialog.open) dialog.close(); }
function showChat() { cancelAppearancePreview(); closeDrawer({ restoreFocus: false }); showScreen(chatScreen); if (activeChat && !isGenerating) messageInput.focus(); }
function showSettings() { closeDrawer(); showScreen(settingsScreen); document.querySelector("#close-settings").focus(); void loadSettings().then(() => Promise.all([loadModels({ quiet: true }), loadEmbeddingModels({ quiet: true }), loadImageModels({ quiet: true })])); }
function showCharacters() { closeDrawer(); showScreen(charactersScreen); document.querySelector("#close-characters").focus(); void loadCharacters(); }
function showPersonas() { closeDrawer(); showScreen(personasScreen); document.querySelector("#close-personas").focus(); void Promise.all([loadSettings(), loadPersonas()]); }
function showLorebooks() { closeDrawer(); showScreen(lorebooksScreen); document.querySelector("#close-lorebooks").focus(); void loadLorebooks(); }
function showImportCharacter() { closeDrawer(); showScreen(importScreen); importForm.reset(); showStatus(importStatus); document.querySelector("#close-import").focus(); }

function setAppearance(appearance) {
  document.documentElement.style.setProperty("--background", appearance.app_background);
  document.documentElement.style.setProperty("--surface", appearance.secondary_background);
  document.documentElement.style.setProperty("--user-bubble", appearance.user_message_background);
  document.documentElement.style.setProperty("--user-text", appearance.user_message_text);
  document.documentElement.style.setProperty("--purple", appearance.character_message_background);
  document.documentElement.style.setProperty("--character-text", appearance.character_message_text);
  document.documentElement.style.setProperty("--gold", appearance.accent_color);
  document.documentElement.style.setProperty("--bright-gold", brighterAccent(appearance.accent_color));
  document.documentElement.style.setProperty("--muted", appearance.muted_text);
}
function normalizeSettings(settings) { return { appearance: { ...defaultAppearance, ...(settings.appearance || {}) }, provider: { ...defaultProvider, ...(settings.provider || {}) }, generation: { ...defaultGeneration, ...(settings.generation || {}) }, persona: { ...defaultPersonaSettings, ...(settings.persona || {}) }, image: { ...defaultImageSettings, ...(settings.image || {}) }, memory: { ...defaultMemorySettings, ...(settings.memory || {}) }, effective_ollama_base_url: settings.effective_ollama_base_url || "http://localhost:11434", effective_openai_compatible_base_url: settings.effective_openai_compatible_base_url || "http://localhost:8080/v1", effective_text_base_url: settings.effective_text_base_url || "http://localhost:11434", effective_a1111_base_url: settings.effective_a1111_base_url || "http://localhost:7860", capabilities: settings.capabilities || {} }; }
function normalizeHexColor(value) { const normalized = value.trim().toUpperCase(); return /^#[0-9A-F]{6}$/.test(normalized) ? normalized : null; }
function brighterAccent(hex) { const accent = normalizeHexColor(hex) || defaultAppearance.accent_color; if (accent === defaultAppearance.accent_color) return "#E8C55A"; return "#" + [1, 3, 5].map((index) => Math.round(parseInt(accent.slice(index, index + 2), 16) + (255 - parseInt(accent.slice(index, index + 2), 16)) * .3).toString(16).padStart(2, "0")).join("").toUpperCase(); }
function syncAppearanceControls(appearance) { appearanceForm.querySelectorAll(".hex-input").forEach((input) => { const value = appearance[input.dataset.appearanceField] || defaultAppearance[input.dataset.appearanceField]; input.value = value; input.setAttribute("aria-invalid", "false"); const picker = appearanceForm.querySelector('.color-picker[data-appearance-field="' + input.dataset.appearanceField + '"]'); if (picker) picker.value = value; }); }
function appearanceFromControls() { const appearance = {}; let invalid = false; appearanceForm.querySelectorAll(".hex-input").forEach((input) => { const value = normalizeHexColor(input.value); input.setAttribute("aria-invalid", String(!value)); if (value) appearance[input.dataset.appearanceField] = value; else invalid = true; }); return invalid ? null : appearance; }
function relativeLuminance(hex) { const channels = [1, 3, 5].map((index) => parseInt(hex.slice(index, index + 2), 16) / 255).map((value) => value <= .03928 ? value / 12.92 : ((value + .055) / 1.055) ** 2.4); return .2126 * channels[0] + .7152 * channels[1] + .0722 * channels[2]; }
function contrastRatio(first, second) { const [light, dark] = [relativeLuminance(first), relativeLuminance(second)].sort((a, b) => b - a); return (light + .05) / (dark + .05); }
function warnIfLowContrast(appearance) { const hardToRead = contrastRatio(appearance.user_message_text, appearance.user_message_background) < 3 || contrastRatio(appearance.character_message_text, appearance.character_message_background) < 3; if (hardToRead) showStatus(appearanceStatus, "Some message text may be hard to read with these colors."); }
function applyAppearancePreview() { const appearance = appearanceFromControls(); if (!appearance) { showStatus(appearanceStatus, "Use six-digit hex colors, such as #C9A227.", "error"); return false; } setAppearance(appearance); showStatus(appearanceStatus); warnIfLowContrast(appearance); return true; }
function cancelAppearancePreview() { setAppearance(appSettings.appearance); syncAppearanceControls(appSettings.appearance); }
function applySettings(settings) {
  appSettings = normalizeSettings(settings); setAppearance(appSettings.appearance);
  syncAppearanceControls(appSettings.appearance);
  renderPersonaCards();
  syncTextBackendControls();
  document.querySelector("#temperature").value = appSettings.generation.temperature;
  document.querySelector("#context-length").value = appSettings.generation.context_length;
  document.querySelector("#max-response-length").value = appSettings.generation.max_response_length;
  syncImageSettingsControls();
  populateCharacterCreatorModelChoices(availableModels);
  syncMemorySettingsControls();
}
function settingsPayload() { return { appearance: appSettings.appearance, provider: appSettings.provider, generation: appSettings.generation, persona: appSettings.persona, image: appSettings.image, memory: appSettings.memory }; }
function setServerAvailability(available) { networkBanner.hidden = available; }
function serverUnavailableMessage() { return "NyxAI server is unavailable. Check your connection or Tailscale status."; }
function userFacingNetworkError(error, fallbackMessage) { return !navigator.onLine || error instanceof TypeError ? serverUnavailableMessage() : (error.message || fallbackMessage); }
async function readJson(response, fallbackMessage) { setServerAvailability(true); const result = await response.json().catch(() => ({})); if (!response.ok) throw new Error(result.error || fallbackMessage); return result; }
async function persistSettings() { const result = await readJson(await fetch("/api/settings", { method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify(settingsPayload()) }), "Settings could not be saved."); applySettings(result); return result; }
async function checkServerHealth({ quiet = false } = {}) { try { await readJson(await fetch("/api/health", { cache: "no-store" }), "NyxAI server is unavailable."); setServerAvailability(true); return true; } catch { setServerAvailability(false); if (!quiet) showToast(serverUnavailableMessage()); return false; } }
async function loadAbout() { try { const about = await readJson(await fetch("/api/about", { cache: "no-store" }), "Could not load NyxAI version."); document.querySelector("#app-version").textContent = about.name + " v" + about.version; } catch { document.querySelector("#app-version").textContent = "NyxAI · self-hosted"; } }
async function loadSettings() { try { applySettings(await readJson(await fetch("/api/settings"), "Settings could not be loaded.")); } catch (error) { applySettings(appSettings); setServerAvailability(false); showToast(userFacingNetworkError(error, "Settings could not be loaded.")); } }

function setConnectionState(state) {
  connectionDot.className = "connection-dot"; if (state) connectionDot.classList.add("is-" + state);
  connectionDot.setAttribute("aria-label", state === "connected" ? "Connected to local inference backend" : state === "connecting" ? "Connecting to local inference backend" : state === "error" ? "Local inference backend connection failed" : "Connection not tested");
}
function backendLabel() { return appSettings.provider.active_provider === "openai_compatible" ? "OpenAI-Compatible local server" : "Ollama"; }
function backendUrl() { return appSettings.provider.active_provider === "openai_compatible" ? (appSettings.provider.openai_compatible_base_url || appSettings.effective_openai_compatible_base_url) : (appSettings.provider.ollama_base_url || appSettings.effective_ollama_base_url); }
function syncTextBackendControls() { const compatible = appSettings.provider.active_provider === "openai_compatible"; const manual = document.querySelector("#manual-model"); document.querySelector("#text-provider").value = appSettings.provider.active_provider; document.querySelector("#ollama-url").value = backendUrl(); document.querySelector("#ollama-url").placeholder = compatible ? "http://localhost:8080/v1" : "http://localhost:11434"; document.querySelector("#text-backend-url-label").textContent = compatible ? "OpenAI-compatible server URL" : "Ollama server URL"; document.querySelector("#text-backend-help").textContent = compatible ? "For a private llama.cpp, vLLM, or compatible local server. Optional credentials stay server-side in OPENAI_COMPATIBLE_API_KEY." : "Ollama's local API stays on your private network."; manual.value = appSettings.provider.selected_model || ""; manual.disabled = !compatible; manual.placeholder = compatible ? "Use when discovery is unavailable" : "Choose a discovered Ollama model"; }
async function saveConnection({ testAfterSave = false } = {}) { const provider = document.querySelector("#text-provider").value; const url = document.querySelector("#ollama-url").value.trim() || null; appSettings.provider.active_provider = provider; if (provider === "openai_compatible") appSettings.provider.openai_compatible_base_url = url; else appSettings.provider.ollama_base_url = url; showStatus(connectionStatus, "Saving backend…"); await persistSettings(); showStatus(connectionStatus, backendLabel() + " saved.", "success"); if (testAfterSave) await testConnection(); }
async function testConnection() { setConnectionState("connecting"); showStatus(connectionStatus, "Connecting to " + backendLabel() + "…"); try { const result = await readJson(await fetch("/api/providers/text/test", { method: "POST" }), "Could not connect to the local inference backend."); setConnectionState("connected"); showStatus(connectionStatus, result.message, "success"); await loadModels({ quiet: true }); } catch (error) { setConnectionState("error"); showStatus(connectionStatus, userFacingNetworkError(error, "Could not connect to the local inference backend."), "error"); } }
function formatBytes(bytes) { const units = ["B", "KB", "MB", "GB", "TB"]; const unit = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1); const value = bytes / (1024 ** unit); return (value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)) + " " + units[unit]; }
function formatModel(model) { const details = [model.parameter_size, model.size_bytes ? formatBytes(model.size_bytes) : null].filter(Boolean).join(" · "); return details ? model.name + " (" + details + ")" : model.name; }
function configuredCreatorModel() { return appSettings.provider.character_creator_model || appSettings.provider.selected_model || ""; }
function populateCharacterCreatorModelChoices(models = availableModels) {
  availableModels = models || [];
  const dialogModel = document.querySelector("#character-creator-dialog-model");
  const fallback = configuredCreatorModel();
  characterCreatorModelSettings.replaceChildren(new Option("Use the selected chat model", ""));
  dialogModel.replaceChildren(new Option(fallback ? "Use configured model: " + fallback : "Choose a local model", ""));
  availableModels.forEach((model) => {
    characterCreatorModelSettings.add(new Option(formatModel(model), model.name));
    dialogModel.add(new Option(formatModel(model), model.name));
  });
  const hasModels = availableModels.length > 0;
  characterCreatorModelSettings.disabled = !hasModels;
  document.querySelector("#save-character-creator-model").disabled = !hasModels;
  characterCreatorModelSettings.value = availableModels.some((model) => model.name === appSettings.provider.character_creator_model) ? appSettings.provider.character_creator_model : "";
  dialogModel.value = availableModels.some((model) => model.name === creatorModelOverride) ? creatorModelOverride : "";
  if (!hasModels) showStatus(characterCreatorModelStatus, "Refresh the selected local backend to choose a creator model, or use the selected chat model.");
  else if (appSettings.provider.character_creator_model && !availableModels.some((model) => model.name === appSettings.provider.character_creator_model)) showStatus(characterCreatorModelStatus, "Your saved Character Creator Model is unavailable. Choose another model or use the chat model.", "error");
  else if (appSettings.provider.character_creator_model) showStatus(characterCreatorModelStatus, "Using " + appSettings.provider.character_creator_model + " for Character Creator drafts.", "success");
  else showStatus(characterCreatorModelStatus, appSettings.provider.selected_model ? "Character Creator falls back to " + appSettings.provider.selected_model + "." : "Choose a local model for chat or Character Creator.");
  populateImagePromptModelChoices();
  populateMemoryModelChoices();
}
function populateModels(result) {
  modelSelect.replaceChildren(); modelSelect.add(new Option(result.models.length ? "Choose a discovered model" : "No models discovered", ""));
  if (!result.models.length) { modelSelect.disabled = true; document.querySelector("#save-model").disabled = false; populateCharacterCreatorModelChoices([]); showStatus(modelStatus, appSettings.provider.active_provider === "openai_compatible" ? "No models were discovered. Enter a model identifier manually." : "No Ollama models are installed yet."); return; }
  result.models.forEach((model) => modelSelect.add(new Option(formatModel(model), model.name, false, model.name === result.selected_model)));
  populateCharacterCreatorModelChoices(result.models);
  modelSelect.disabled = false; document.querySelector("#save-model").disabled = false;
  if (!result.selected_model_available) showStatus(modelStatus, "Your previously selected model is unavailable. Choose another one.", "error"); else if (result.selected_model) showStatus(modelStatus, "Using " + result.selected_model + ".", "success"); else showStatus(modelStatus, "Choose a model to start chatting.");
}
async function loadModels({ quiet = false } = {}) { if (!quiet) showStatus(modelStatus, "Refreshing local models…"); try { const result = await readJson(await fetch("/api/providers/text/models", { method: "POST" }), "Models could not be retrieved."); populateModels(result); if (!quiet) setConnectionState("connected"); } catch (error) { modelSelect.replaceChildren(new Option("Discovery unavailable", "")); modelSelect.disabled = true; document.querySelector("#save-model").disabled = false; populateCharacterCreatorModelChoices([]); showStatus(modelStatus, appSettings.provider.active_provider === "openai_compatible" ? "Model discovery is unavailable. Enter a model identifier manually." : "Model discovery is unavailable. Check the Ollama connection and try again.", "error"); if (!quiet) setConnectionState("error"); } }
async function selectModel() { const manual = document.querySelector("#manual-model"); const model = (!manual.disabled ? manual.value.trim() : "") || modelSelect.value; if (!model) { showStatus(modelStatus, manual.disabled ? "Choose a discovered Ollama model first." : "Choose or enter a model identifier first.", "error"); return; } document.querySelector("#save-model").disabled = true; showStatus(modelStatus, "Saving selected model…"); try { const result = await readJson(await fetch("/api/providers/text/model", { method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ model }) }), "The model could not be selected."); appSettings.provider.selected_model = result.selected_model; document.querySelector("#manual-model").value = result.selected_model || ""; populateModels(result); } catch (error) { showStatus(modelStatus, error.message, "error"); document.querySelector("#save-model").disabled = false; } }
async function saveCharacterCreatorModel() { const model = characterCreatorModelSettings.value || null; const previous = appSettings.provider.character_creator_model; appSettings.provider.character_creator_model = model; document.querySelector("#save-character-creator-model").disabled = true; showStatus(characterCreatorModelStatus, "Saving creator model…"); try { await persistSettings(); populateCharacterCreatorModelChoices(availableModels); showStatus(characterCreatorModelStatus, model ? "Character Creator Model saved." : "Character Creator now uses the selected chat model.", "success"); } catch (error) { appSettings.provider.character_creator_model = previous; populateCharacterCreatorModelChoices(availableModels); showStatus(characterCreatorModelStatus, error.message, "error"); } finally { document.querySelector("#save-character-creator-model").disabled = !availableModels.length; } }

function syncImageSettingsControls() {
  const image = appSettings.image;
  document.querySelector("#image-enabled").checked = image.enabled;
  document.querySelector("#a1111-url").value = image.a1111_base_url || appSettings.effective_a1111_base_url;
  document.querySelector("#image-avatar-width").value = image.avatar_width;
  document.querySelector("#image-avatar-height").value = image.avatar_height;
  document.querySelector("#image-chat-width").value = image.chat_width;
  document.querySelector("#image-chat-height").value = image.chat_height;
  document.querySelector("#image-steps").value = image.steps;
  document.querySelector("#image-cfg").value = image.cfg_scale;
  document.querySelector("#image-negative-prompt").value = image.negative_prompt || "";
  document.querySelector("#single-gpu-memory-mode").checked = image.single_gpu_memory_mode;
  document.querySelector("#restore-text-model").checked = image.restore_text_model_after_generation;
  populateImagePromptModelChoices();
}
function populateMemoryModelChoices() {
  const embedding = document.querySelector("#memory-embedding-model"); const manualEmbedding = document.querySelector("#memory-embedding-model-manual"); const extraction = document.querySelector("#memory-extraction-model");
  embedding.replaceChildren(new Option(availableEmbeddingModels.length ? "Choose an embedding model" : "Refresh embedding models first", ""));
  extraction.replaceChildren(new Option("Use Character Creator or chat model", ""));
  availableEmbeddingModels.forEach((model) => embedding.add(new Option(formatModel(model), model.name)));
  availableModels.forEach((model) => extraction.add(new Option(formatModel(model), model.name)));
  embedding.value = availableEmbeddingModels.some((model) => model.name === appSettings.memory.embedding_model) ? appSettings.memory.embedding_model : "";
  manualEmbedding.value = appSettings.memory.embedding_model || "";
  extraction.value = availableModels.some((model) => model.name === appSettings.memory.extraction_model) ? appSettings.memory.extraction_model : "";
  embedding.disabled = !availableEmbeddingModels.length; extraction.disabled = !availableModels.length;
}
async function loadEmbeddingModels({ quiet = false } = {}) { try { const result = await readJson(await fetch("/api/providers/embeddings/models", { method: "POST" }), "Embedding models could not be retrieved."); availableEmbeddingModels = result.models || []; populateMemoryModelChoices(); } catch (error) { availableEmbeddingModels = []; populateMemoryModelChoices(); if (!quiet) showStatus(document.querySelector("#memory-settings-status"), "Embedding model discovery is unavailable for this backend.", "error"); } }
function syncMemorySettingsControls() {
  const memory = appSettings.memory;
  document.querySelector("#memory-enabled").checked = memory.enabled;
  document.querySelector("#memory-automatic").checked = memory.automatic_extraction;
  document.querySelector("#memory-embedding-provider").value = memory.embedding_provider;
  document.querySelector("#memory-interval").value = memory.extraction_interval;
  document.querySelector("#memory-retrieval-count").value = memory.retrieval_count;
  document.querySelector("#memory-threshold").value = memory.similarity_threshold;
  document.querySelector("#memory-budget").value = memory.context_budget;
  populateMemoryModelChoices();
}
function memorySettingsFromControls() {
  return { enabled: document.querySelector("#memory-enabled").checked, automatic_extraction: document.querySelector("#memory-automatic").checked, embedding_model: document.querySelector("#memory-embedding-model-manual").value.trim() || document.querySelector("#memory-embedding-model").value || null, embedding_provider: document.querySelector("#memory-embedding-provider").value, extraction_model: document.querySelector("#memory-extraction-model").value || null, extraction_interval: Number(document.querySelector("#memory-interval").value), retrieval_count: Number(document.querySelector("#memory-retrieval-count").value), similarity_threshold: Number(document.querySelector("#memory-threshold").value), context_budget: Number(document.querySelector("#memory-budget").value) };
}
async function saveMemorySettings() {
  const previous = appSettings.memory; appSettings.memory = memorySettingsFromControls(); const status = document.querySelector("#memory-settings-status"); showStatus(status, "Saving memory settings…");
  try { await persistSettings(); if (previous.embedding_provider !== appSettings.memory.embedding_provider) void loadEmbeddingModels({ quiet: true }); showStatus(status, previous.embedding_model && previous.embedding_model !== appSettings.memory.embedding_model ? "Embedding model saved. Rebuild the memory index before retrieval resumes." : "Semantic memory settings saved.", "success"); }
  catch (error) { appSettings.memory = previous; syncMemorySettingsControls(); showStatus(status, error.message, "error"); }
}
async function rebuildMemoryIndex() {
  const button = document.querySelector("#rebuild-memory-index"); const status = document.querySelector("#memory-settings-status"); button.disabled = true; showStatus(status, "Rebuilding local memory index…");
  try { const result = await readJson(await fetch("/api/memories/rebuild", { method: "POST" }), "Memory index could not be rebuilt."); showStatus(status, result.failed ? ("Reindexed " + result.reindexed + " memories; " + result.failed + " failed.") : ("Reindexed " + result.reindexed + " memories."), result.failed ? "error" : "success"); }
  catch (error) { showStatus(status, error.message, "error"); } finally { button.disabled = false; }
}
function populateImagePromptModelChoices() {
  const select = document.querySelector("#image-prompt-model");
  select.replaceChildren(new Option("Use Character Creator or chat model", ""));
  availableModels.forEach((model) => select.add(new Option(formatModel(model), model.name)));
  select.value = availableModels.some((model) => model.name === appSettings.image.image_prompt_model) ? appSettings.image.image_prompt_model : "";
}
function populateImageModels(result) {
  const select = document.querySelector("#image-model");
  select.replaceChildren(new Option("Use backend default", ""));
  (result.models || []).forEach((model) => select.add(new Option(model, model)));
  select.value = (result.models || []).includes(appSettings.image.selected_model) ? appSettings.image.selected_model : "";
  if (appSettings.image.selected_model && !result.selected_model_available) showStatus(imageSettingsStatus, "Your saved image model is unavailable. The backend default will be used until you select another.", "error");
}
async function loadImageModels({ quiet = false } = {}) {
  if (!quiet) showStatus(imageSettingsStatus, "Refreshing image models…");
  try { populateImageModels(await readJson(await fetch("/api/providers/a1111/models", { method: "POST" }), "Image models could not be retrieved.")); }
  catch (error) { if (!quiet) showStatus(imageSettingsStatus, userFacingNetworkError(error, "Image models could not be retrieved."), "error"); }
}
function imageSettingsFromControls() {
  return {
    enabled: document.querySelector("#image-enabled").checked,
    a1111_base_url: document.querySelector("#a1111-url").value.trim() || null,
    selected_model: document.querySelector("#image-model").value || null,
    image_prompt_model: document.querySelector("#image-prompt-model").value || null,
    single_gpu_memory_mode: document.querySelector("#single-gpu-memory-mode").checked,
    restore_text_model_after_generation: document.querySelector("#restore-text-model").checked,
    avatar_width: Number(document.querySelector("#image-avatar-width").value), avatar_height: Number(document.querySelector("#image-avatar-height").value),
    chat_width: Number(document.querySelector("#image-chat-width").value), chat_height: Number(document.querySelector("#image-chat-height").value),
    steps: Number(document.querySelector("#image-steps").value), cfg_scale: Number(document.querySelector("#image-cfg").value),
    sampler_name: null, negative_prompt: document.querySelector("#image-negative-prompt").value,
  };
}
async function testImageConnection() {
  showStatus(imageSettingsStatus, "Connecting to image backend…");
  try { const result = await readJson(await fetch("/api/providers/a1111/test", { method: "POST" }), "Could not connect to the image backend."); showStatus(imageSettingsStatus, result.message, "success"); await loadImageModels({ quiet: true }); }
  catch (error) { showStatus(imageSettingsStatus, userFacingNetworkError(error, "Could not connect to the image backend."), "error"); }
}
async function saveImageSettings({ testAfterSave = false } = {}) {
  const previous = appSettings.image;
  appSettings.image = imageSettingsFromControls();
  showStatus(imageSettingsStatus, "Saving image settings…");
  try {
    await persistSettings();
    await loadImageModels({ quiet: true });
    showStatus(imageSettingsStatus, "Image settings saved.", "success");
    if (testAfterSave) await testImageConnection();
  } catch (error) {
    appSettings.image = previous;
    syncImageSettingsControls();
    showStatus(imageSettingsStatus, error.message, "error");
  }
}

function initials(name) { return (name || "?").trim().split(/\s+/).slice(0, 2).map((word) => word[0]).join("").toUpperCase() || "?"; }
function avatarElement(character, extraClass = "") { const fallback = document.createElement("span"); fallback.className = "avatar avatar-fallback " + extraClass; fallback.textContent = initials(character.name); fallback.setAttribute("aria-hidden", "true"); if (character.avatar_url) { const image = document.createElement("img"); image.className = "avatar avatar-file " + extraClass; image.src = character.avatar_url; image.alt = character.name + " avatar"; image.addEventListener("error", () => image.replaceWith(fallback), { once: true }); return image; } return fallback; }
function tagChip(tag, removable = false) { const chip = document.createElement("span"); chip.className = "tag-chip"; const text = document.createElement("span"); text.textContent = tag; chip.append(text); if (removable) { const button = document.createElement("button"); button.type = "button"; button.setAttribute("aria-label", "Remove " + tag); button.textContent = "×"; button.addEventListener("click", () => { editorTags = editorTags.filter((item) => item !== tag); renderEditorTags(); markEditorDirty(); }); chip.append(button); } return chip; }
function renderRoleplayContent(container, rawContent) {
  const parser = window.NyxRoleplay && window.NyxRoleplay.parseRoleplaySegments;
  const segments = parser ? parser(rawContent) : [{ kind: "plain", text: rawContent }];
  const fragment = document.createDocumentFragment();
  segments.forEach((segment) => { const span = document.createElement("span"); span.className = "roleplay-" + segment.kind; span.textContent = segment.text; fragment.append(span); });
  container.replaceChildren(fragment);
}

function isNearConversationBottom() { return conversation.scrollHeight - conversation.scrollTop - conversation.clientHeight < 96; }
function scrollConversationToBottom() { window.requestAnimationFrame(() => { conversation.scrollTop = conversation.scrollHeight; }); }
function appendMessageElement(message, { streaming = false, forceScroll = false } = {}) {
  const shouldStick = forceScroll || isNearConversationBottom(); const article = document.createElement("article"); article.className = "message " + (message.role === "user" ? "message-user" : "message-character"); if (streaming) article.classList.add("is-streaming");
  if (message.role === "assistant") { const sender = document.createElement("div"); sender.className = "message-sender"; if (activeCharacter) sender.append(avatarElement(activeCharacter), document.createTextNode(" " + activeCharacter.name)); else sender.textContent = "NyxAI"; article.append(sender); }
  const content = document.createElement("div"); content.className = "message-content"; renderRoleplayContent(content, message.content); article.append(content);
  if (!streaming && (message.role === "user" || message.role === "assistant")) { const actions = document.createElement("div"); actions.className = "message-actions"; const copy = document.createElement("button"); copy.className = "message-copy"; copy.type = "button"; copy.textContent = "Copy"; copy.addEventListener("click", () => { void copyMessage(message.content); }); actions.append(copy); if (message.role === "assistant" && message.id && !message.id.startsWith("pending-")) { const lore = document.createElement("button"); lore.className = "message-copy"; lore.type = "button"; lore.textContent = "Context"; lore.setAttribute("aria-label", "Show lore and semantic memory context used for this response"); lore.addEventListener("click", () => { void showLoreForMessage(message.id); }); actions.append(lore); } article.append(actions); }
  conversation.append(article); if (shouldStick) scrollConversationToBottom(); return { article, content };
}
async function copyMessage(content) { try { await navigator.clipboard.writeText(content); showToast("Message copied."); } catch { showToast("Could not copy this message."); } }
function renderConversation() {
  conversation.replaceChildren();
  if (!activeChat || !activeCharacter) { const empty = document.createElement("div"); empty.className = "conversation-empty"; empty.innerHTML = "<strong>Choose a conversation</strong>Create a character, then use <em>New chat</em> from its menu to begin."; conversation.append(empty); return; }
  if (!activeMessages.length && !activeImages.length) { const empty = document.createElement("div"); empty.className = "conversation-empty"; const title = document.createElement("strong"); title.textContent = "Start the conversation"; const text = document.createElement("span"); text.textContent = "Send the first message to " + activeCharacter.name + "."; empty.append(title, text); conversation.append(empty); return; }
  const divider = document.createElement("div"); divider.className = "date-divider"; divider.textContent = "Conversation"; conversation.append(divider); activeMessages.filter((message) => message.role !== "system").forEach((message) => appendMessageElement(message)); activeImages.forEach(appendSceneImageElement); scrollConversationToBottom();
}
function appendSceneImageElement(image) { const card = document.createElement("figure"); card.className = "scene-image-card"; const img = document.createElement("img"); img.src = image.image_url; img.alt = "Generated scene image"; img.loading = "lazy"; img.addEventListener("error", () => { card.classList.add("is-unavailable"); }); const caption = document.createElement("figcaption"); caption.textContent = "Generated scene"; const remove = document.createElement("button"); remove.type = "button"; remove.className = "scene-image-remove"; remove.textContent = "Remove"; remove.setAttribute("aria-label", "Delete generated scene image"); remove.addEventListener("click", () => { void deleteSceneImage(image.id); }); caption.append(remove); card.append(img, caption); conversation.append(card); }
function updateChatHeader() { activeCharacterName.textContent = activeCharacter ? activeCharacter.name : "NyxAI"; activeChatTitle.textContent = activeChat ? activeChat.title : "Choose a chat"; activePersonaName.textContent = activeChat ? "Persona: " + (activePersona ? activePersona.name : "User") : ""; }
function resizeComposerInput() { messageInput.style.height = "auto"; messageInput.style.height = Math.min(messageInput.scrollHeight, 140) + "px"; }
function setComposerState() { const canCompose = Boolean(activeChat && activeCharacter) && !isGenerating; messageInput.disabled = !canCompose; sendButton.disabled = !canCompose; stopButton.hidden = !isGenerating; composer.classList.toggle("is-generating", isGenerating); messageInput.placeholder = activeChat ? "Message " + activeCharacter.name + "…" : "Choose a chat in the menu…"; resizeComposerInput(); }
function setGenerating(active) { isGenerating = active; if (!active) activeController = null; setComposerState(); }

function chatPath(characterId, chatId = "") { return "/api/characters/" + encodeURIComponent(characterId) + "/chats" + (chatId ? "/" + encodeURIComponent(chatId) : ""); }
function applyChatDetail(detail) { activeCharacter = detail.character; activeChat = { ...detail.chat, lorebooks: detail.lorebooks || [] }; activePersona = detail.persona || null; activeMessages = detail.messages || []; activeImages = detail.images || []; expandedCharacters.add(activeCharacter.id); const current = chatLists.get(activeCharacter.id) || []; const index = current.findIndex((chat) => chat.id === activeChat.id); if (index >= 0) current[index] = activeChat; else current.unshift(activeChat); chatLists.set(activeCharacter.id, current); updateChatHeader(); renderConversation(); renderSidebarCharacters(); setComposerState(); }
async function loadChatsForCharacter(characterId, { quiet = false } = {}) { try { const chats = await readJson(await fetch(chatPath(characterId)), "Chats could not be loaded."); chatLists.set(characterId, chats); renderSidebarCharacters(); return chats; } catch (error) { if (!quiet) showToast(error.message); throw error; } }
async function toggleCharacterChats(characterId) { if (!expandedCharacters.has(characterId)) { expandedCharacters.add(characterId); renderSidebarCharacters(); try { await loadChatsForCharacter(characterId, { quiet: true }); } catch { showToast("Chats could not be loaded."); } } else { expandedCharacters.delete(characterId); renderSidebarCharacters(); } }
async function openChat(characterId, chatId) { if (isGenerating && (!activeChat || activeChat.id !== chatId)) { showToast("Stop the current response before opening another chat."); return; } try { applyChatDetail(await readJson(await fetch(chatPath(characterId, chatId)), "Chat could not be loaded.")); showChat(); } catch (error) { showToast(userFacingNetworkError(error, "Chat could not be loaded.")); } }
async function createChatWithSetup(characterId, greetingId = null, personaId = undefined) { const body = {}; if (greetingId) body.greeting_id = greetingId; if (personaId !== undefined) body.persona_id = personaId; try { applyChatDetail(await readJson(await fetch(chatPath(characterId), { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }), "Chat could not be created.")); closeDialog(newChatDialog); showChat(); showToast("New chat created."); } catch (error) { showToast(userFacingNetworkError(error, "Chat could not be created.")); } }
function selectedNewChatPersonaId() { const selected = document.querySelector("#new-chat-persona").value; return selected || null; }
function updateNewChatGreetingPreview() { const greetingSelect = document.querySelector("#new-chat-greeting"); const option = greetingSelect.selectedOptions[0]; document.querySelector("#new-chat-greeting-preview").textContent = option && option.dataset.preview ? option.dataset.preview : "This chat starts without an opening message."; }
function openNewChatChooser(character) { const personaSelect = document.querySelector("#new-chat-persona"); const greetingSelect = document.querySelector("#new-chat-greeting"); personaSelect.replaceChildren(new Option("No persona — User", "")); personas.forEach((persona) => personaSelect.add(new Option(persona.name, persona.id, false, persona.id === preferredPersonaId()))); if (personas.length === 1) personaSelect.value = personas[0].id; const defaultGreeting = new Option("Default greeting", ""); defaultGreeting.dataset.preview = character.first_message || ""; greetingSelect.replaceChildren(defaultGreeting); (character.alternate_greetings || []).forEach((greeting, index) => { const option = new Option("Alternate greeting " + (index + 1), greeting.id); option.dataset.preview = greeting.content || ""; greetingSelect.add(option); }); newChatCharacter = character; document.querySelector("#new-chat-dialog-copy").textContent = "Choose how " + character.name + " opens this conversation."; updateNewChatGreetingPreview(); newChatDialog.showModal(); }
function preferredPersonaId() { const defaultId = appSettings.persona.default_persona_id; if (defaultId && personas.some((persona) => persona.id === defaultId)) return defaultId; return personas.length === 1 ? personas[0].id : null; }
async function createChatForCharacter(characterId) { if (isGenerating) { showToast("Stop the current response before creating another chat."); return; } try { const [character] = await Promise.all([readJson(await fetch("/api/characters/" + encodeURIComponent(characterId)), "Character could not be loaded."), loadPersonas({ quiet: true })]); const needsSetup = (character.alternate_greetings || []).length > 0 || personas.length > 1; if (needsSetup) { openNewChatChooser(character); return; } await createChatWithSetup(characterId, null, personas.length === 1 ? personas[0].id : undefined); } catch (error) { showToast(error.message); } }

function renderSidebarCharacters() {
  sidebarCharacters.replaceChildren();
  if (!characters.length) {
    const empty = document.createElement("div"); empty.className = "sidebar-empty";
    const title = document.createElement("strong"); title.textContent = "No characters yet.";
    const copy = document.createElement("span"); copy.textContent = "Create a character to begin.";
    const create = document.createElement("button"); create.className = "secondary-button sidebar-create-character"; create.type = "button"; create.textContent = "Create character"; create.addEventListener("click", () => { void openCharacterEditor(); });
    empty.append(title, copy, create); sidebarCharacters.append(empty); return;
  }
  characters.forEach((character) => {
    const open = expandedCharacters.has(character.id); const group = document.createElement("section"); group.className = "character-group" + (open ? " is-open" : ""); const header = document.createElement("div"); header.className = "character-group-header";
    const chatListId = "character-chats-" + character.id;
    const toggle = document.createElement("button"); toggle.className = "character-row"; toggle.type = "button"; toggle.setAttribute("aria-expanded", String(open)); toggle.setAttribute("aria-controls", chatListId); const chevron = document.createElement("span"); chevron.className = "chevron"; chevron.setAttribute("aria-hidden", "true"); const name = document.createElement("span"); name.className = "character-name"; name.textContent = character.name; name.title = character.name; toggle.append(chevron, avatarElement(character), name); toggle.addEventListener("click", () => { void toggleCharacterChats(character.id); });
    const actions = document.createElement("button"); actions.className = "sidebar-action"; actions.type = "button"; actions.textContent = "•••"; actions.setAttribute("aria-label", "Actions for " + character.name); actions.addEventListener("click", () => openCharacterActions(character.id));
    const chatList = document.createElement("div"); chatList.className = "chat-list"; chatList.id = chatListId; chatList.hidden = !open; const chats = chatLists.get(character.id);
    if (open && !chats) { const loading = document.createElement("p"); loading.className = "sidebar-chat-empty"; loading.textContent = "Loading chats…"; chatList.append(loading); }
    else if (open && !chats.length) { const empty = document.createElement("p"); empty.className = "sidebar-chat-empty"; empty.textContent = "No chats yet"; chatList.append(empty); }
    else if (chats) chats.forEach((chat) => {
      const isActive = Boolean(activeChat && activeChat.id === chat.id); const entry = document.createElement("div"); entry.className = "chat-entry";
      const button = document.createElement("button"); button.className = "chat-row" + (isActive ? " active" : ""); button.type = "button"; button.title = chat.title; if (isActive) button.setAttribute("aria-current", "page");
      const label = document.createElement("span"); label.className = "chat-row-label"; label.textContent = chat.title; button.append(label);
      if (isActive) { const marker = document.createElement("span"); marker.className = "active-chat-marker"; marker.setAttribute("aria-hidden", "true"); marker.textContent = "●"; button.append(marker); }
      button.addEventListener("click", () => { void openChat(character.id, chat.id); });
      const chatActions = document.createElement("button"); chatActions.className = "chat-row-action"; chatActions.type = "button"; chatActions.textContent = "•••"; chatActions.setAttribute("aria-label", "Actions for chat " + chat.title); chatActions.addEventListener("click", () => openChatActionsFor(character.id, chat));
      entry.append(button, chatActions); chatList.append(entry);
    });
    const newChat = document.createElement("button"); newChat.className = "new-chat"; newChat.type = "button"; newChat.textContent = "+ New Chat"; newChat.disabled = isGenerating; newChat.addEventListener("click", () => { void createChatForCharacter(character.id); }); chatList.append(newChat);
    header.append(toggle, actions); group.append(header, chatList); sidebarCharacters.append(group);
  });
}
function renderCharacterCards() {
  characterCards.replaceChildren();
  if (!characters.length) { const empty = document.createElement("div"); empty.className = "character-empty"; empty.innerHTML = "<strong>Your cast is empty.</strong>Create your first character to define their personality, scenario, greeting, and prompt details."; characterCards.append(empty); return; }
  characters.forEach((character) => {
    const card = document.createElement("article"); card.className = "character-card"; card.append(avatarElement(character, "character-card-avatar")); const main = document.createElement("button"); main.className = "character-card-main"; main.type = "button"; const name = document.createElement("h3"); name.textContent = character.name; const description = document.createElement("p"); description.textContent = character.description || "No description yet."; main.append(name, description); if (character.tags.length) { const tags = document.createElement("div"); tags.className = "tag-chips"; character.tags.slice(0, 3).forEach((tag) => tags.append(tagChip(tag))); main.append(tags); } main.addEventListener("click", () => { void openCharacterEditor(character.id); });
    const actions = document.createElement("button"); actions.className = "icon-button"; actions.type = "button"; actions.setAttribute("aria-label", "Actions for " + character.name); actions.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1.25"/><circle cx="12" cy="12" r="1.25"/><circle cx="19" cy="12" r="1.25"/></svg>'; actions.addEventListener("click", () => openCharacterActions(character.id)); card.append(main, actions); characterCards.append(card);
  });
}
async function loadCharacters() {
  try { characters = await readJson(await fetch("/api/characters"), "Characters could not be loaded."); const ids = new Set(characters.map((character) => character.id)); chatLists.forEach((_, id) => { if (!ids.has(id)) chatLists.delete(id); }); if (activeCharacter && !ids.has(activeCharacter.id)) { activeCharacter = null; activeChat = null; activePersona = null; activeMessages = []; activeImages = []; updateChatHeader(); renderConversation(); setComposerState(); } renderSidebarCharacters(); renderCharacterCards(); showStatus(charactersStatus); }
  catch (error) { characters = []; setServerAvailability(false); renderSidebarCharacters(); renderCharacterCards(); showStatus(charactersStatus, userFacingNetworkError(error, "Characters could not be loaded."), "error"); }
}

function renderPersonaCards() {
  personaCards.replaceChildren();
  if (!personas.length) { const empty = document.createElement("div"); empty.className = "character-empty"; empty.innerHTML = "<strong>No personas yet.</strong>Create one to personalize <code>{{user}}</code> in roleplay prompts."; personaCards.append(empty); return; }
  personas.forEach((persona) => {
    const card = document.createElement("article"); card.className = "character-card"; card.append(avatarElement(persona, "character-card-avatar")); const main = document.createElement("button"); main.className = "character-card-main"; main.type = "button"; const name = document.createElement("h3"); name.textContent = persona.name; const description = document.createElement("p"); description.textContent = persona.description || "No description yet."; main.append(name, description); if (persona.id === appSettings.persona.default_persona_id) { const badge = document.createElement("span"); badge.className = "persona-default-badge"; badge.textContent = "Default"; main.append(badge); } main.addEventListener("click", () => { void openPersonaEditor(persona.id); }); const actions = document.createElement("button"); actions.className = "icon-button"; actions.type = "button"; actions.setAttribute("aria-label", "Actions for " + persona.name); actions.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="5" cy="12" r="1.25"/><circle cx="12" cy="12" r="1.25"/><circle cx="19" cy="12" r="1.25"/></svg>'; actions.addEventListener("click", () => openPersonaActions(persona.id)); card.append(main, actions); personaCards.append(card);
  });
}
async function loadPersonas({ quiet = false } = {}) {
  try { personas = await readJson(await fetch("/api/personas"), "Personas could not be loaded."); renderPersonaCards(); showStatus(personasStatus); return personas; }
  catch (error) { personas = []; setServerAvailability(false); renderPersonaCards(); if (!quiet) showStatus(personasStatus, userFacingNetworkError(error, "Personas could not be loaded."), "error"); return personas; }
}
function emptyPersonaDraft() { return { id: "", name: "", avatar_path: null, avatar_url: null, description: "" }; }
function setPersonaEditorAvatar(persona) { personaEditorAvatarUrl = persona.avatar_url || null; document.querySelector("#persona-avatar-path").value = persona.avatar_path || ""; personaAvatarPreview.hidden = !personaEditorAvatarUrl; personaAvatarFallback.hidden = Boolean(personaEditorAvatarUrl); document.querySelector("#remove-persona-avatar").hidden = !persona.avatar_path; if (personaEditorAvatarUrl) personaAvatarPreview.src = personaEditorAvatarUrl; personaAvatarFallback.textContent = initials(persona.name); }
function setPersonaEditorFields(persona) { document.querySelector("#persona-id").value = persona.id || ""; document.querySelector("#persona-name").value = persona.name || ""; document.querySelector("#persona-description").value = persona.description || ""; setPersonaEditorAvatar(persona); }
async function openPersonaEditor(id = null) { let persona = emptyPersonaDraft(); if (id) { try { persona = await readJson(await fetch("/api/personas/" + encodeURIComponent(id)), "Persona could not be loaded."); } catch (error) { showToast(error.message); return; } } closeDrawer(); showScreen(personaEditorScreen); setPersonaEditorFields(persona); personaEditorDirty = false; showStatus(personaEditorStatus); document.querySelector("#persona-editor-title").textContent = id ? "Edit persona" : "New persona"; document.querySelector("#persona-editor-subtitle").textContent = id ? persona.name : "Your roleplay identity"; document.querySelector("#persona-name").focus(); }
function personaDraftFromEditor() { return { name: document.querySelector("#persona-name").value, avatar_path: document.querySelector("#persona-avatar-path").value || null, description: document.querySelector("#persona-description").value }; }
function leavePersonaEditor() { if (personaEditorDirty && !window.confirm("Discard unsaved persona changes?")) return; personaEditorDirty = false; showPersonas(); }
async function savePersona(event) { event.preventDefault(); const id = document.querySelector("#persona-id").value; const draft = personaDraftFromEditor(); if (!draft.name.trim()) { showStatus(personaEditorStatus, "Give the persona a name.", "error"); document.querySelector("#persona-name").focus(); return; } const saveButton = document.querySelector("#save-persona"); saveButton.disabled = true; showStatus(personaEditorStatus, "Saving persona…"); try { const persona = await readJson(await fetch(id ? "/api/personas/" + encodeURIComponent(id) : "/api/personas", { method: id ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(draft) }), "Persona could not be saved."); if (activePersona && activePersona.id === persona.id) { activePersona = persona; updateChatHeader(); } personaEditorDirty = false; await loadPersonas(); showPersonas(); showToast(persona.name + (id ? " updated." : " created.")); } catch (error) { showStatus(personaEditorStatus, error.message, "error"); } finally { saveButton.disabled = false; } }
async function uploadPersonaAvatar() { const file = personaAvatarInput.files && personaAvatarInput.files[0]; if (!file) return; if (file.size > 4 * 1024 * 1024) { showStatus(personaEditorStatus, "Choose an image smaller than 4 MB.", "error"); personaAvatarInput.value = ""; return; } const form = new FormData(); form.append("avatar", file); showStatus(personaEditorStatus, "Uploading avatar…"); try { const result = await readJson(await fetch("/api/avatars", { method: "POST", body: form }), "Avatar upload failed."); document.querySelector("#persona-avatar-path").value = result.avatar_path; personaEditorAvatarUrl = result.avatar_url; personaAvatarPreview.src = result.avatar_url; personaAvatarPreview.hidden = false; personaAvatarFallback.hidden = true; document.querySelector("#remove-persona-avatar").hidden = false; personaEditorDirty = true; showStatus(personaEditorStatus, "Avatar ready. Save the persona to keep it.", "success"); } catch (error) { showStatus(personaEditorStatus, error.message, "error"); } finally { personaAvatarInput.value = ""; } }
function removePersonaEditorAvatar() { document.querySelector("#persona-avatar-path").value = ""; personaEditorAvatarUrl = null; personaAvatarPreview.src = ""; personaAvatarPreview.hidden = true; personaAvatarFallback.hidden = false; personaAvatarFallback.textContent = initials(document.querySelector("#persona-name").value); document.querySelector("#remove-persona-avatar").hidden = true; personaEditorDirty = true; showStatus(personaEditorStatus, "Avatar will be removed when you save."); }
function selectedPersona() { return personas.find((persona) => persona.id === personaActionId); }
function openPersonaActions(id) { personaActionId = id; const persona = selectedPersona(); if (!persona) return; document.querySelector("#action-persona-name").textContent = persona.name; document.querySelector("#action-set-default-persona").textContent = persona.id === appSettings.persona.default_persona_id ? "Default persona" : "Set as default"; document.querySelector("#action-set-default-persona").disabled = persona.id === appSettings.persona.default_persona_id; personaActionsDialog.showModal(); }
async function setDefaultPersona() { const persona = selectedPersona(); if (!persona) return; closeDialog(personaActionsDialog); const previous = appSettings.persona.default_persona_id; appSettings.persona.default_persona_id = persona.id; try { await persistSettings(); renderPersonaCards(); showToast(persona.name + " is now the default persona."); } catch (error) { appSettings.persona.default_persona_id = previous; renderPersonaCards(); showToast(error.message); } }
async function duplicateSelectedPersona() { const persona = selectedPersona(); if (!persona) return; closeDialog(personaActionsDialog); showToast("Duplicating " + persona.name + "…"); try { const duplicate = await readJson(await fetch("/api/personas/" + encodeURIComponent(persona.id) + "/duplicate", { method: "POST" }), "Persona could not be duplicated."); await loadPersonas(); showToast(duplicate.name + " created."); } catch (error) { showToast(error.message); } }
function requestDeleteSelectedPersona() { const persona = selectedPersona(); if (!persona) return; closeDialog(personaActionsDialog); deletePersonaId = persona.id; document.querySelector("#delete-persona-copy").textContent = "Delete " + persona.name + "? Chats using it will safely fall back to User for future prompts. Existing messages and greetings are unchanged."; deletePersonaDialog.showModal(); }
async function confirmDeletePersona() { const persona = personas.find((item) => item.id === deletePersonaId); if (!persona) { closeDialog(deletePersonaDialog); return; } const button = document.querySelector("#confirm-delete-persona"); button.disabled = true; try { const result = await readJson(await fetch("/api/personas/" + encodeURIComponent(persona.id), { method: "DELETE" }), "Persona could not be deleted."); closeDialog(deletePersonaDialog); if (activePersona && activePersona.id === persona.id) { activePersona = null; if (activeChat) activeChat.persona_id = null; updateChatHeader(); } await Promise.all([loadPersonas(), loadSettings()]); showToast(persona.name + " deleted." + (result.chats_reassigned ? " " + result.chats_reassigned + " chat(s) now use User." : "")); } catch (error) { showToast(error.message); } finally { button.disabled = false; deletePersonaId = null; } }

function emptyDraft() { return { id: "", name: "", avatar_path: null, avatar_url: null, description: "", personality: "", scenario: "", first_message: "", example_dialogue: "", system_prompt: "", creator_notes: "", alternate_greetings: [], tags: [] }; }
function setCharacterCreatorDraftState({ prompt = "", alternateGreetingCount = 3, model = null } = {}) { creatorOriginalPrompt = prompt; creatorAlternateGreetingCount = alternateGreetingCount; creatorModelOverride = model; const hasDraft = Boolean(prompt); document.querySelectorAll(".field-generate").forEach((button) => { button.hidden = !hasDraft; }); document.querySelector("#regenerate-alternate-greetings").hidden = !hasDraft; }
function setCharacterCreatorBusy(busy) { isCharacterCreatorGenerating = busy; document.querySelector("#confirm-character-creator").disabled = busy; document.querySelector("#cancel-character-creator").disabled = busy; document.querySelector("#confirm-field-regeneration").disabled = busy; document.querySelector("#cancel-field-regeneration").disabled = busy; document.querySelectorAll(".field-generate").forEach((button) => { button.disabled = busy; }); document.querySelector("#regenerate-alternate-greetings").disabled = busy; }
function creatorFieldLabel(field) { return ({ description: "Description", personality: "Personality", scenario: "Scenario", first_message: "First message", alternate_greetings: "Alternate greetings", example_dialogue: "Example dialogue", system_prompt: "System prompt", tags: "Tags" })[field] || "field"; }
async function openCharacterCreator() { if (isCharacterCreatorGenerating) return; const dialogModel = document.querySelector("#character-creator-dialog-model"); const status = document.querySelector("#character-creator-status"); populateCharacterCreatorModelChoices(availableModels); document.querySelector("#character-creator-prompt").value = creatorOriginalPrompt || ""; document.querySelector("#character-creator-greeting-count").value = creatorAlternateGreetingCount; dialogModel.value = availableModels.some((model) => model.name === creatorModelOverride) ? creatorModelOverride : ""; showStatus(status); characterCreatorDialog.showModal(); document.querySelector("#character-creator-prompt").focus(); if (!availableModels.length) { showStatus(status, "Loading local models…"); await loadModels({ quiet: true }); if (availableModels.length || appSettings.provider.selected_model) showStatus(status); else showStatus(status, "No local model is selected. Configure a local backend and choose or enter a model in Settings.", "error"); } }
function openGeneratedCharacterEditor(draft, prompt, alternateGreetingCount, model) { closeDialog(characterCreatorDialog); closeDrawer({ restoreFocus: false }); showScreen(characterEditorScreen); setEditorFields({ ...emptyDraft(), ...draft }); setCharacterCreatorDraftState({ prompt, alternateGreetingCount, model }); editorDirty = true; showStatus(editorStatus, "Character draft generated locally. Review it and Save character when you are ready.", "success"); document.querySelector("#editor-title").textContent = "Review generated character"; document.querySelector("#editor-subtitle").textContent = draft.name || "AI Character Creator"; document.querySelector("#character-name").focus(); }
async function generateCharacterDraft() { if (isCharacterCreatorGenerating) return; const prompt = document.querySelector("#character-creator-prompt").value; const alternateGreetingCount = Number(document.querySelector("#character-creator-greeting-count").value); const model = document.querySelector("#character-creator-dialog-model").value || null; if (!prompt.trim()) { showStatus(document.querySelector("#character-creator-status"), "Describe the character you want to create.", "error"); return; } if (!Number.isInteger(alternateGreetingCount) || alternateGreetingCount < 0 || alternateGreetingCount > 10) { showStatus(document.querySelector("#character-creator-status"), "Choose between 0 and 10 alternate greetings.", "error"); return; } setCharacterCreatorBusy(true); showStatus(document.querySelector("#character-creator-status"), "Generating character draft locally…"); try { const result = await readJson(await fetch("/api/character-creator/generate", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ prompt, alternate_greeting_count: alternateGreetingCount, model }) }), "Character draft could not be generated."); openGeneratedCharacterEditor(result.draft, prompt, alternateGreetingCount, model); showToast("Character draft ready to review."); } catch (error) { showStatus(document.querySelector("#character-creator-status"), userFacingNetworkError(error, "Character draft could not be generated."), "error"); } finally { setCharacterCreatorBusy(false); } }
function requestFieldRegeneration(field) { if (!creatorOriginalPrompt) { showToast("Generate a character draft first to regenerate individual fields."); return; } if (isCharacterCreatorGenerating) return; creatorFieldTarget = field; document.querySelector("#field-regeneration-title").textContent = "Regenerate " + creatorFieldLabel(field); document.querySelector("#field-regeneration-copy").textContent = "Only " + creatorFieldLabel(field).toLowerCase() + " will change. Your other edits stay intact."; document.querySelector("#field-regeneration-instruction").value = ""; showStatus(document.querySelector("#field-regeneration-status")); fieldRegenerationDialog.showModal(); document.querySelector("#field-regeneration-instruction").focus(); }
async function regenerateCharacterField() { if (!creatorFieldTarget || isCharacterCreatorGenerating) return; const instruction = document.querySelector("#field-regeneration-instruction").value; const currentAvatarUrl = editorAvatarUrl; setCharacterCreatorBusy(true); showStatus(document.querySelector("#field-regeneration-status"), "Generating " + creatorFieldLabel(creatorFieldTarget).toLowerCase() + " locally…"); try { const result = await readJson(await fetch("/api/character-creator/regenerate", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ field: creatorFieldTarget, draft: characterDraftFromEditor(), original_prompt: creatorOriginalPrompt, instruction, alternate_greeting_count: creatorAlternateGreetingCount, model: creatorModelOverride }) }), "Character field could not be regenerated."); setEditorFields({ ...emptyDraft(), ...result.draft, avatar_url: currentAvatarUrl }); editorDirty = true; closeDialog(fieldRegenerationDialog); showStatus(editorStatus, creatorFieldLabel(creatorFieldTarget) + " regenerated. Review it before saving.", "success"); showToast(creatorFieldLabel(creatorFieldTarget) + " regenerated."); creatorFieldTarget = null; } catch (error) { showStatus(document.querySelector("#field-regeneration-status"), userFacingNetworkError(error, "Character field could not be regenerated."), "error"); } finally { setCharacterCreatorBusy(false); } }
function setEditorAvatar(character) { editorAvatarUrl = character.avatar_url || null; document.querySelector("#avatar-path").value = character.avatar_path || ""; avatarPreview.hidden = !editorAvatarUrl; avatarFallback.hidden = Boolean(editorAvatarUrl); document.querySelector("#remove-avatar").hidden = !character.avatar_path; if (editorAvatarUrl) avatarPreview.src = editorAvatarUrl; avatarFallback.textContent = initials(character.name); }
function setEditorFields(character) {
  document.querySelector("#character-id").value = character.id || ""; document.querySelector("#character-name").value = character.name || ""; document.querySelector("#character-description").value = character.description || ""; document.querySelector("#character-personality").value = character.personality || ""; document.querySelector("#character-scenario").value = character.scenario || ""; document.querySelector("#character-first-message").value = character.first_message || ""; document.querySelector("#character-example-dialogue").value = character.example_dialogue || ""; document.querySelector("#character-system-prompt").value = character.system_prompt || ""; document.querySelector("#character-creator-notes").value = character.creator_notes || "";
  editorTags = [...(character.tags || [])]; editorAlternateGreetings = (character.alternate_greetings || []).map((greeting) => typeof greeting === "string" ? greeting : greeting.content); tagInput.value = ""; renderEditorTags(); renderAlternateGreetings(); setEditorAvatar(character);
}
function renderEditorTags() { tagChips.replaceChildren(...editorTags.map((tag) => tagChip(tag, true))); }
function renderAlternateGreetings() { alternateGreetings.replaceChildren(); editorAlternateGreetings.forEach((greeting, index) => { const field = document.createElement("div"); field.className = "alternate-greeting-field"; const label = document.createElement("label"); label.className = "editor-field"; const heading = document.createElement("span"); heading.textContent = "Alternate greeting " + (index + 1); const textarea = document.createElement("textarea"); textarea.rows = 4; textarea.maxLength = 32000; textarea.value = greeting; textarea.placeholder = "A different opening message for a new chat."; textarea.addEventListener("input", () => { editorAlternateGreetings[index] = textarea.value; markEditorDirty(); }); label.append(heading, textarea); const remove = document.createElement("button"); remove.className = "text-button danger-text remove-greeting"; remove.type = "button"; remove.textContent = "Remove"; remove.addEventListener("click", () => { editorAlternateGreetings.splice(index, 1); renderAlternateGreetings(); markEditorDirty(); }); field.append(label, remove); alternateGreetings.append(field); }); }
function addAlternateGreeting() { if (editorAlternateGreetings.length >= 20) { showStatus(editorStatus, "Use at most 20 alternate greetings.", "error"); return; } editorAlternateGreetings.push(""); renderAlternateGreetings(); markEditorDirty(); const last = alternateGreetings.querySelector("textarea:last-of-type"); if (last) last.focus(); }
function markEditorDirty() { editorDirty = true; }
async function openCharacterEditor(id = null, importWarnings = []) {
  let character = emptyDraft();
  if (id) { try { character = await readJson(await fetch("/api/characters/" + encodeURIComponent(id)), "Character could not be loaded."); } catch (error) { showToast(error.message); return; } }
  closeDrawer(); showScreen(characterEditorScreen); setEditorFields(character); setCharacterCreatorDraftState(); editorDirty = false; showStatus(editorStatus, importWarnings.length ? "Imported successfully. " + importWarnings.join(" ") : "", importWarnings.length ? "success" : ""); document.querySelector("#editor-title").textContent = id ? "Edit character" : "New character"; document.querySelector("#editor-subtitle").textContent = id ? character.name : "Build someone memorable"; document.querySelector("#character-name").focus();
}
function characterDraftFromEditor() { return { name: document.querySelector("#character-name").value, avatar_path: document.querySelector("#avatar-path").value || null, description: document.querySelector("#character-description").value, personality: document.querySelector("#character-personality").value, scenario: document.querySelector("#character-scenario").value, first_message: document.querySelector("#character-first-message").value, example_dialogue: document.querySelector("#character-example-dialogue").value, system_prompt: document.querySelector("#character-system-prompt").value, creator_notes: document.querySelector("#character-creator-notes").value, alternate_greetings: editorAlternateGreetings, tags: editorTags }; }
function leaveCharacterEditor() { if (editorDirty && !window.confirm("Discard unsaved character changes?")) return; void discardGeneratedAvatarCandidate(); editorDirty = false; showCharacters(); }
async function saveCharacter(event) {
  event.preventDefault(); const id = document.querySelector("#character-id").value; const draft = characterDraftFromEditor();
  if (!draft.name.trim()) { showStatus(editorStatus, "Give the character a name.", "error"); document.querySelector("#character-name").focus(); return; }
  const saveButton = document.querySelector("#save-character"); saveButton.disabled = true; showStatus(editorStatus, "Saving character…");
  try {
    const character = await readJson(await fetch(id ? "/api/characters/" + encodeURIComponent(id) : "/api/characters", { method: id ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(draft) }), "Character could not be saved.");
    if (activeCharacter && activeCharacter.id === character.id) { activeCharacter = character; updateChatHeader(); renderConversation(); }
    editorDirty = false; setCharacterCreatorDraftState(); await loadCharacters(); showCharacters(); showToast(character.name + (id ? " updated." : " created."));
  } catch (error) { showStatus(editorStatus, error.message, "error"); }
  finally { saveButton.disabled = false; }
}
async function importCharacterCard(event) {
  event.preventDefault(); const file = importCardFile.files && importCardFile.files[0];
  if (!file) { showStatus(importStatus, "Choose a JSON or PNG character card.", "error"); return; }
  if (file.size > 4 * 1024 * 1024) { showStatus(importStatus, "Character cards must be smaller than 4 MB.", "error"); return; }
  const button = document.querySelector("#confirm-import"); button.disabled = true; showStatus(importStatus, "Importing character…");
  try { const body = new FormData(); body.append("card", file); const result = await readJson(await fetch("/api/characters/import", { method: "POST", body }), "Character import failed."); await loadCharacters(); showToast(result.character.name + " imported."); await openCharacterEditor(result.character.id, result.warnings || []); }
  catch (error) { showStatus(importStatus, error.message, "error"); }
  finally { button.disabled = false; }
}
async function uploadSelectedAvatar() {
  const file = avatarInput.files && avatarInput.files[0]; if (!file) return;
  if (file.size > 4 * 1024 * 1024) { showStatus(editorStatus, "Choose an image smaller than 4 MB.", "error"); avatarInput.value = ""; return; }
  await discardGeneratedAvatarCandidate(); const form = new FormData(); form.append("avatar", file); showStatus(editorStatus, "Uploading avatar…");
  try { const result = await readJson(await fetch("/api/avatars", { method: "POST", body: form }), "Avatar upload failed."); document.querySelector("#avatar-path").value = result.avatar_path; editorAvatarUrl = result.avatar_url; avatarPreview.src = result.avatar_url; avatarPreview.hidden = false; avatarFallback.hidden = true; document.querySelector("#remove-avatar").hidden = false; markEditorDirty(); showStatus(editorStatus, "Avatar ready. Save the character to keep it.", "success"); }
  catch (error) { showStatus(editorStatus, error.message, "error"); }
  finally { avatarInput.value = ""; }
}
async function discardGeneratedAvatarCandidate() {
  const candidate = generatedAvatarCandidate; generatedAvatarCandidate = null;
  if (!candidate) return;
  try { await fetch("/api/generated-avatars/" + encodeURIComponent(candidate.avatar_path), { method: "DELETE" }); } catch {}
}
async function generateAvatarCandidate() {
  if (isImageGenerating) return;
  const draft = characterDraftFromEditor();
  if (!draft.name.trim()) { showStatus(editorStatus, "Give the character a name before generating an avatar.", "error"); document.querySelector("#character-name").focus(); return; }
  isImageGenerating = true; const button = document.querySelector("#generate-avatar"); button.disabled = true; showStatus(editorStatus, "Generating avatar locally…");
  try { await discardGeneratedAvatarCandidate(); const candidate = await readJson(await fetch("/api/character-avatar/generate", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(draft) }), "Avatar could not be generated."); generatedAvatarCandidate = candidate; document.querySelector("#generated-avatar-preview").src = candidate.avatar_url; document.querySelector("#generated-avatar-prompt").textContent = candidate.prompt; showStatus(document.querySelector("#generated-avatar-status")); generatedAvatarDialog.showModal(); }
  catch (error) { showStatus(editorStatus, userFacingNetworkError(error, "Avatar could not be generated."), "error"); }
  finally { isImageGenerating = false; button.disabled = false; }
}
function useGeneratedAvatarCandidate() {
  if (!generatedAvatarCandidate) return; const candidate = generatedAvatarCandidate; generatedAvatarCandidate = null;
  document.querySelector("#avatar-path").value = candidate.avatar_path; editorAvatarUrl = candidate.avatar_url; avatarPreview.src = candidate.avatar_url; avatarPreview.hidden = false; avatarFallback.hidden = true; document.querySelector("#remove-avatar").hidden = false; markEditorDirty(); closeDialog(generatedAvatarDialog); showStatus(editorStatus, "Generated avatar ready. Save the character to keep it.", "success");
}
function removeEditorAvatar() { document.querySelector("#avatar-path").value = ""; editorAvatarUrl = null; avatarPreview.src = ""; avatarPreview.hidden = true; avatarFallback.hidden = false; avatarFallback.textContent = initials(document.querySelector("#character-name").value); document.querySelector("#remove-avatar").hidden = true; markEditorDirty(); showStatus(editorStatus, "Avatar will be removed when you save."); }
function addEditorTag() { const tag = tagInput.value.trim(); if (!tag) return; if (tag.length > 40) { showStatus(editorStatus, "Tags can be at most 40 characters.", "error"); return; } if (editorTags.length >= 20) { showStatus(editorStatus, "Use at most 20 tags.", "error"); return; } if (!editorTags.some((existing) => existing.toLowerCase() === tag.toLowerCase())) editorTags.push(tag); tagInput.value = ""; renderEditorTags(); markEditorDirty(); }
function selectedCharacter() { return characters.find((character) => character.id === actionCharacterId); }
function openCharacterActions(id) { actionCharacterId = id; const character = selectedCharacter(); if (!character) return; document.querySelector("#action-character-name").textContent = character.name; actionsDialog.showModal(); }
function createChatForSelectedCharacter() { const character = selectedCharacter(); if (!character) return; closeDialog(actionsDialog); void createChatForCharacter(character.id); }
async function duplicateSelectedCharacter() { const character = selectedCharacter(); if (!character) return; closeDialog(actionsDialog); showToast("Duplicating " + character.name + "…"); try { const duplicate = await readJson(await fetch("/api/characters/" + encodeURIComponent(character.id) + "/duplicate", { method: "POST" }), "Character could not be duplicated."); await loadCharacters(); showToast(duplicate.name + " created."); } catch (error) { showToast(error.message); } }
function requestDeleteSelectedCharacter() { const character = selectedCharacter(); if (!character) return; closeDialog(actionsDialog); deleteCharacterId = character.id; document.querySelector("#delete-character-copy").textContent = "Delete " + character.name + "? This will permanently remove this character, all chats belonging to it, and their messages."; deleteDialog.showModal(); }
async function confirmDeleteCharacter() { const character = characters.find((item) => item.id === deleteCharacterId); if (!character) { closeDialog(deleteDialog); return; } document.querySelector("#confirm-delete-character").disabled = true; try { await readJson(await fetch("/api/characters/" + encodeURIComponent(character.id), { method: "DELETE" }), "Character could not be deleted."); closeDialog(deleteDialog); await loadCharacters(); showToast(character.name + " deleted."); } catch (error) { showToast(error.message); } finally { document.querySelector("#confirm-delete-character").disabled = false; deleteCharacterId = null; } }

function parseSseBlock(block) { const fields = { event: "message", data: "" }; const data = []; block.split("\n").forEach((line) => { if (line.startsWith("event:")) fields.event = line.slice(6).trim(); if (line.startsWith("data:")) data.push(line.slice(5).trimStart()); }); fields.data = data.join("\n"); return fields; }
async function consumeSse(response, onEvent) { if (!response.body) throw new Error("NyxAI could not read the streaming response."); const reader = response.body.getReader(); const decoder = new TextDecoder(); let buffered = ""; while (true) { const { value, done } = await reader.read(); buffered += decoder.decode(value || new Uint8Array(), { stream: !done }); const blocks = buffered.split(/\n\n/); buffered = blocks.pop(); blocks.forEach((block) => { if (block.trim()) onEvent(parseSseBlock(block)); }); if (done) break; } if (buffered.trim()) onEvent(parseSseBlock(buffered)); }
function newGenerationId() { return crypto.randomUUID ? crypto.randomUUID() : "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (character) => { const random = Math.random() * 16 | 0; return (character === "x" ? random : random & 3 | 8).toString(16); }); }
async function persistPartialAssistant(characterId, chatId, generationId, content) { if (!content.trim()) return; try { await readJson(await fetch(chatPath(characterId, chatId) + "/partial-assistant", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ generation_id: generationId, content }) }), "Partial response could not be saved."); } catch (error) { console.warn("Could not save partial response", error); } }
async function refreshActiveChat(characterId, chatId) { if (!activeChat || activeChat.id !== chatId || !activeCharacter || activeCharacter.id !== characterId) return; try { applyChatDetail(await readJson(await fetch(chatPath(characterId, chatId)), "Chat could not be refreshed.")); } catch (error) { showChatStatus(userFacingNetworkError(error, "Chat could not be refreshed."), "error"); } }
async function sendMessage(message) {
  if (isGenerating) return;
  if (!activeChat || !activeCharacter) { showChatStatus("Create or open a chat before sending a message.", "error"); return; }
  if (!appSettings.provider.selected_model) { showChatStatus("Choose a local text model in Settings before sending a message.", "error"); showSettings(); return; }
  const characterId = activeCharacter.id; const chatId = activeChat.id; const generationId = newGenerationId(); const userMessage = { id: "pending-user", role: "user", content: message };
  activeMessages.push(userMessage);
  if (!conversation.querySelector(".date-divider")) { conversation.replaceChildren(); const divider = document.createElement("div"); divider.className = "date-divider"; divider.textContent = "Conversation"; conversation.append(divider); }
  appendMessageElement(userMessage, { forceScroll: true }); const assistantTarget = appendMessageElement({ id: "pending-assistant", role: "assistant", content: "" }, { streaming: true, forceScroll: true });
  let receivedContent = ""; let streamError = ""; activeController = new AbortController(); setGenerating(true); showChatStatus("");
  try {
    const response = await fetch(chatPath(characterId, chatId) + "/generate", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ content: message, generation_id: generationId }), signal: activeController.signal });
    if (!response.ok) { const error = await response.json().catch(() => ({})); throw new Error(error.error || "Generation could not start."); }
    await consumeSse(response, (event) => { if (event.event === "delta") { const chunk = JSON.parse(event.data); receivedContent += chunk.content; renderRoleplayContent(assistantTarget.content, receivedContent); if (isNearConversationBottom()) scrollConversationToBottom(); } else if (event.event === "context") { const context = JSON.parse(event.data); const labels = [...(context.lore_used || []), ...(context.memories_used || [])]; if (labels.length) showChatStatus("Using context: " + labels.join(" · ") + "."); } else if (event.event === "error") streamError = JSON.parse(event.data).error || "Generation failed."; });
    if (streamError) throw new Error(streamError);
    if (!receivedContent) { renderRoleplayContent(assistantTarget.content, "No response was generated."); showChatStatus("The local inference backend completed without returning text.", "error"); } else showChatStatus("");
  } catch (error) {
    const cancelled = error.name === "AbortError"; if (receivedContent) await persistPartialAssistant(characterId, chatId, generationId, receivedContent);
    if (cancelled) { if (!receivedContent) assistantTarget.article.remove(); showChatStatus("Generation stopped."); } else { if (!receivedContent) renderRoleplayContent(assistantTarget.content, "Generation could not be completed."); showChatStatus(userFacingNetworkError(error, "Generation failed."), "error"); }
  } finally { assistantTarget.article.classList.remove("is-streaming"); await refreshActiveChat(characterId, chatId); setGenerating(false); }
}

function openChatActions() { if (!activeChat || !activeCharacter) { showToast("Open a chat to manage it."); return; } openChatActionsFor(activeCharacter.id, activeChat); }
function openChatActionsFor(characterId, chat) { if (isGenerating) { showToast("Stop the current response before changing chats."); return; } chatActionTarget = { characterId, chatId: chat.id, title: chat.title, personaId: chat.persona_id || null }; document.querySelector("#action-chat-name").textContent = chat.title; document.querySelector("#action-generate-scene").hidden = isImageGenerating; document.querySelector("#action-stop-scene").hidden = !isImageGenerating; chatActionsDialog.showModal(); }
function selectedChatAction() { return chatActionTarget; }
async function generateSceneImage() {
  const target = selectedChatAction(); if (!target || isImageGenerating) return;
  isImageGenerating = true; imageGenerationController = new AbortController(); document.querySelector("#action-generate-scene").hidden = true; document.querySelector("#action-stop-scene").hidden = false; showChatStatus("Generating a scene image locally…");
  try { const image = await readJson(await fetch(chatPath(target.characterId, target.chatId) + "/images", { method: "POST", signal: imageGenerationController.signal }), "Scene image could not be generated."); if (activeChat && activeChat.id === target.chatId) { activeImages.push(image); renderConversation(); } closeDialog(chatActionsDialog); showToast("Scene image generated."); }
  catch (error) { if (error.name === "AbortError") showChatStatus("Image generation stopped."); else showChatStatus(userFacingNetworkError(error, "Scene image could not be generated."), "error"); }
  finally { isImageGenerating = false; imageGenerationController = null; document.querySelector("#action-generate-scene").hidden = false; document.querySelector("#action-stop-scene").hidden = true; }
}
async function stopSceneImage() { const target = selectedChatAction(); if (!target) return; showChatStatus("Stopping image generation…"); if (imageGenerationController) imageGenerationController.abort(); try { await fetch(chatPath(target.characterId, target.chatId) + "/images/interrupt", { method: "POST" }); } catch {} }
async function deleteSceneImage(imageId) { if (!activeChat || !activeCharacter) return; try { const response = await fetch(chatPath(activeCharacter.id, activeChat.id) + "/images/" + encodeURIComponent(imageId), { method: "DELETE" }); if (!response.ok) throw new Error("Scene image could not be deleted."); activeImages = activeImages.filter((image) => image.id !== imageId); renderConversation(); showToast("Scene image deleted."); } catch (error) { showToast(error.message); } }
function requestRenameChat() { const target = selectedChatAction(); if (!target) return; closeDialog(chatActionsDialog); document.querySelector("#rename-chat-input").value = target.title; showStatus(document.querySelector("#rename-chat-status")); renameChatDialog.showModal(); document.querySelector("#rename-chat-input").focus(); }
async function confirmRenameChat() {
  const target = selectedChatAction(); if (!target) return; const title = document.querySelector("#rename-chat-input").value.trim(); if (!title) { showStatus(document.querySelector("#rename-chat-status"), "Give this chat a name.", "error"); return; }
  const button = document.querySelector("#confirm-rename-chat"); button.disabled = true;
  try { const chat = await readJson(await fetch(chatPath(target.characterId, target.chatId), { method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ title }) }), "Chat could not be renamed."); const chats = chatLists.get(target.characterId) || []; const index = chats.findIndex((item) => item.id === chat.id); if (index >= 0) chats[index] = chat; chatLists.set(target.characterId, chats); if (activeChat && activeChat.id === chat.id) { activeChat = chat; updateChatHeader(); } renderSidebarCharacters(); closeDialog(renameChatDialog); chatActionTarget = null; showToast("Chat renamed."); } catch (error) { showStatus(document.querySelector("#rename-chat-status"), error.message, "error"); } finally { button.disabled = false; }
}
function requestDeleteChat() { const target = selectedChatAction(); if (!target) return; closeDialog(chatActionsDialog); document.querySelector("#delete-chat-copy").textContent = "Delete \"" + target.title + "\"? This will permanently remove this conversation and its messages."; deleteChatDialog.showModal(); }
async function confirmDeleteChat() {
  const target = selectedChatAction(); if (!target) return; const button = document.querySelector("#confirm-delete-chat"); button.disabled = true;
  try { const response = await fetch(chatPath(target.characterId, target.chatId), { method: "DELETE" }); if (!response.ok) { const result = await response.json().catch(() => ({})); throw new Error(result.error || "Chat could not be deleted."); } chatLists.set(target.characterId, (chatLists.get(target.characterId) || []).filter((chat) => chat.id !== target.chatId)); if (activeChat && activeChat.id === target.chatId) { activeChat = null; activeCharacter = null; activePersona = null; activeMessages = []; activeImages = []; updateChatHeader(); renderConversation(); setComposerState(); } renderSidebarCharacters(); closeDialog(deleteChatDialog); chatActionTarget = null; showToast(target.title + " deleted."); } catch (error) { showToast(error.message); } finally { button.disabled = false; }
}

function lorebookPath(id = "") { return "/api/lorebooks" + (id ? "/" + encodeURIComponent(id) : ""); }
async function loadLorebooks({ quiet = false } = {}) { try { lorebooks = await readJson(await fetch(lorebookPath()), "Lorebooks could not be loaded."); renderLorebookCards(); return lorebooks; } catch (error) { if (!quiet) showToast(error.message); throw error; } }
function renderLorebookCards() { lorebookCards.replaceChildren(); if (!lorebooks.length) { const empty = document.createElement("div"); empty.className = "character-empty"; empty.innerHTML = "<strong>No lorebooks yet.</strong>Create a collection of reusable world information."; lorebookCards.append(empty); return; } lorebooks.forEach((book) => { const card = document.createElement("article"); card.className = "character-card"; const main = document.createElement("button"); main.className = "character-card-main"; main.type = "button"; const name = document.createElement("h3"); name.textContent = book.name; const description = document.createElement("p"); description.textContent = book.description || (book.enabled ? "Enabled for attached chats." : "Disabled."); main.append(name, description); main.addEventListener("click", () => { void openLorebookEditor(book.id); }); const actions = document.createElement("button"); actions.className = "icon-button"; actions.type = "button"; actions.setAttribute("aria-label", "Actions for " + book.name); actions.textContent = "•••"; actions.addEventListener("click", () => openLorebookActions(book)); card.append(main, actions); lorebookCards.append(card); }); }
function openLorebookActions(book) { lorebookActionBook = book; document.querySelector("#action-lorebook-name").textContent = book.name; lorebookActionsDialog.showModal(); }
function editSelectedLorebook() { if (!lorebookActionBook) return; const id = lorebookActionBook.id; closeDialog(lorebookActionsDialog); void openLorebookEditor(id); }
async function duplicateSelectedLorebook() { if (!lorebookActionBook) return; const book = lorebookActionBook; closeDialog(lorebookActionsDialog); try { const copy = await readJson(await fetch(lorebookPath(book.id) + "/duplicate", { method: "POST" }), "Lorebook could not be duplicated."); await loadLorebooks({ quiet: true }); showToast(copy.name + " created."); } catch (error) { showToast(error.message); } }
async function deleteSelectedLorebook() { if (!lorebookActionBook) return; const book = lorebookActionBook; if (!window.confirm("Delete " + book.name + " and all its entries?")) return; closeDialog(lorebookActionsDialog); try { await readJson(await fetch(lorebookPath(book.id), { method: "DELETE" }), "Lorebook could not be deleted."); await loadLorebooks({ quiet: true }); showToast(book.name + " deleted."); } catch (error) { showToast(error.message); } }
async function openLorebookEditor(id = null) { let book = { id: "", name: "", description: "", enabled: true }; let entries = []; if (id) { try { [book, entries] = await Promise.all([readJson(await fetch(lorebookPath(id)), "Lorebook could not be loaded."), readJson(await fetch(lorebookPath(id) + "/entries"), "Lore entries could not be loaded.")]); } catch (error) { showToast(error.message); return; } } activeLorebook = book; document.querySelector("#lorebook-id").value = book.id || ""; document.querySelector("#lorebook-name").value = book.name || ""; document.querySelector("#lorebook-description").value = book.description || ""; document.querySelector("#lorebook-enabled").checked = book.enabled !== false; document.querySelector("#lorebook-editor-title").textContent = id ? "Edit lorebook" : "New lorebook"; document.querySelector("#lorebook-editor-subtitle").textContent = id ? book.name : "World information"; document.querySelector("#export-lorebook").hidden = !id; renderLoreEntries(entries); showStatus(lorebookEditorStatus); showScreen(lorebookEditorScreen); document.querySelector("#lorebook-name").focus(); }
function renderLoreEntries(entries) { const container = document.querySelector("#lore-entry-list"); container.replaceChildren(); if (!activeLorebook || !activeLorebook.id) { const copy = document.createElement("p"); copy.className = "field-copy"; copy.textContent = "Save this lorebook before adding entries."; container.append(copy); document.querySelector("#new-lore-entry-inline").disabled = true; return; } document.querySelector("#new-lore-entry-inline").disabled = false; if (!entries.length) { const empty = document.createElement("p"); empty.className = "field-copy"; empty.textContent = "No entries yet. Add world rules or facts with trigger keys."; container.append(empty); return; } entries.forEach((entry) => { const card = document.createElement("article"); card.className = "lore-entry-card"; const title = document.createElement("strong"); title.textContent = entry.name + (entry.enabled ? "" : " · Disabled"); const details = document.createElement("span"); details.textContent = (entry.always_active ? "Always Active" : (entry.keys || []).join(", ") || "No keys") + " · Priority " + entry.priority; const edit = document.createElement("button"); edit.className = "text-button"; edit.type = "button"; edit.textContent = "Edit"; edit.addEventListener("click", () => { void openLoreEntryEditor(activeLorebook.id, entry.id); }); const remove = document.createElement("button"); remove.className = "text-button danger-text"; remove.type = "button"; remove.textContent = "Delete"; remove.addEventListener("click", () => { void deleteLoreEntry(activeLorebook.id, entry); }); card.append(title, details, edit, remove); container.append(card); }); }
async function saveLorebook(event) { event.preventDefault(); const id = document.querySelector("#lorebook-id").value; const draft = { name: document.querySelector("#lorebook-name").value, description: document.querySelector("#lorebook-description").value, enabled: document.querySelector("#lorebook-enabled").checked }; showStatus(lorebookEditorStatus, "Saving lorebook…"); try { const book = await readJson(await fetch(id ? lorebookPath(id) : lorebookPath(), { method: id ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(draft) }), "Lorebook could not be saved."); await loadLorebooks({ quiet: true }); showToast(book.name + (id ? " updated." : " created.")); await openLorebookEditor(book.id); } catch (error) { showStatus(lorebookEditorStatus, error.message, "error"); } }
async function openLoreEntryEditor(bookId, entryId = null) { let entry = { id: "", name: "", keys: [], content: "", enabled: true, priority: 50, case_sensitive: false, match_mode: "any_key", always_active: false }; if (entryId) { try { entry = await readJson(await fetch(lorebookPath(bookId) + "/entries/" + encodeURIComponent(entryId)), "Lore entry could not be loaded."); } catch (error) { showToast(error.message); return; } } document.querySelector("#lore-entry-id").value = entry.id || ""; document.querySelector("#lore-entry-book-id").value = bookId; document.querySelector("#lore-entry-name").value = entry.name || ""; document.querySelector("#lore-entry-keys").value = (entry.keys || []).join(", "); document.querySelector("#lore-entry-content").value = entry.content || ""; document.querySelector("#lore-entry-enabled").checked = entry.enabled !== false; document.querySelector("#lore-entry-always-active").checked = Boolean(entry.always_active); document.querySelector("#lore-entry-case-sensitive").checked = Boolean(entry.case_sensitive); document.querySelector("#lore-entry-priority").value = entry.priority ?? 50; document.querySelector("#lore-entry-match-mode").value = entry.match_mode || "any_key"; document.querySelector("#lore-entry-editor-title").textContent = entryId ? "Edit entry" : "New entry"; showStatus(loreEntryEditorStatus); showScreen(loreEntryEditorScreen); document.querySelector("#lore-entry-name").focus(); }
async function saveLoreEntry(event) { event.preventDefault(); const bookId = document.querySelector("#lore-entry-book-id").value; const id = document.querySelector("#lore-entry-id").value; const draft = { name: document.querySelector("#lore-entry-name").value, keys: document.querySelector("#lore-entry-keys").value.split(",").map((key) => key.trim()).filter(Boolean), content: document.querySelector("#lore-entry-content").value, enabled: document.querySelector("#lore-entry-enabled").checked, always_active: document.querySelector("#lore-entry-always-active").checked, case_sensitive: document.querySelector("#lore-entry-case-sensitive").checked, priority: Number(document.querySelector("#lore-entry-priority").value), match_mode: document.querySelector("#lore-entry-match-mode").value }; showStatus(loreEntryEditorStatus, "Saving entry…"); try { await readJson(await fetch(lorebookPath(bookId) + "/entries" + (id ? "/" + encodeURIComponent(id) : ""), { method: id ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(draft) }), "Lore entry could not be saved."); showToast("Lore entry saved."); await openLorebookEditor(bookId); } catch (error) { showStatus(loreEntryEditorStatus, error.message, "error"); } }
async function deleteLoreEntry(bookId, entry) { if (!window.confirm("Delete " + entry.name + "?")) return; try { await readJson(await fetch(lorebookPath(bookId) + "/entries/" + encodeURIComponent(entry.id), { method: "DELETE" }), "Lore entry could not be deleted."); await openLorebookEditor(bookId); showToast("Lore entry deleted."); } catch (error) { showToast(error.message); } }
async function importLorebook() { const input = document.querySelector("#import-lorebook-file"); const file = input.files && input.files[0]; if (!file) return; if (file.size > 2 * 1024 * 1024) { showStatus(lorebooksStatus, "Lorebook imports must be smaller than 2 MB.", "error"); input.value = ""; return; } showStatus(lorebooksStatus, "Importing lorebook…"); try { const parsed = JSON.parse(await file.text()); const book = await readJson(await fetch(lorebookPath() + "/import", { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(parsed) }), "Lorebook import failed."); input.value = ""; await loadLorebooks({ quiet: true }); showToast(book.name + " imported."); await openLorebookEditor(book.id); } catch (error) { input.value = ""; showStatus(lorebooksStatus, error instanceof SyntaxError ? "That file is not valid JSON." : error.message, "error"); } }
async function exportLorebook() { if (!activeLorebook?.id) return; try { const payload = await readJson(await fetch(lorebookPath(activeLorebook.id) + "/export"), "Lorebook could not be exported."); const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" }); const link = document.createElement("a"); link.href = URL.createObjectURL(blob); link.download = (activeLorebook.name || "lorebook").replace(/[^a-z0-9-_]+/gi, "-") + ".nyxai-lorebook.json"; document.body.append(link); link.click(); link.remove(); URL.revokeObjectURL(link.href); showToast("Lorebook exported."); } catch (error) { showStatus(lorebookEditorStatus, error.message, "error"); } }
async function openLorebookAssociation(target) { try { await loadLorebooks({ quiet: true }); const current = await readJson(await fetch(target.path), "Lorebook attachments could not be loaded."); lorebookAssociationTarget = target; const selected = new Set(current.map((book) => book.id)); const options = document.querySelector("#lorebook-association-options"); options.replaceChildren(); if (!lorebooks.length) { const copy = document.createElement("p"); copy.className = "field-copy"; copy.textContent = "No lorebooks yet. Create one from Settings first."; options.append(copy); } lorebooks.forEach((book) => { const label = document.createElement("label"); label.className = "check-field"; const checkbox = document.createElement("input"); checkbox.type = "checkbox"; checkbox.value = book.id; checkbox.checked = selected.has(book.id); checkbox.disabled = !book.enabled; label.append(checkbox, document.createTextNode(" " + book.name + (book.enabled ? "" : " (disabled)"))); options.append(label); }); document.querySelector("#lorebook-association-title").textContent = target.label; showStatus(document.querySelector("#lorebook-association-status")); lorebookAssociationDialog.showModal(); } catch (error) { showToast(error.message); } }
async function saveLorebookAssociation() { if (!lorebookAssociationTarget) return; const ids = [...document.querySelectorAll("#lorebook-association-options input:checked")].map((input) => input.value); const status = document.querySelector("#lorebook-association-status"); showStatus(status, "Saving lorebooks…"); try { const attached = await readJson(await fetch(lorebookAssociationTarget.path, { method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ lorebook_ids: ids }) }), "Lorebook attachments could not be saved."); if (lorebookAssociationTarget.kind === "chat" && activeChat && activeChat.id === lorebookAssociationTarget.chatId) activeChat.lorebooks = attached; closeDialog(lorebookAssociationDialog); showToast("Lorebooks updated."); } catch (error) { showStatus(status, error.message, "error"); } }
function requestCharacterLorebooks() { const character = selectedCharacter(); if (!character) return; closeDialog(actionsDialog); void openLorebookAssociation({ kind: "character", label: "Character lorebooks", path: "/api/characters/" + encodeURIComponent(character.id) + "/lorebooks" }); }
function requestChatLorebooks() { const target = selectedChatAction(); if (!target) return; closeDialog(chatActionsDialog); void openLorebookAssociation({ kind: "chat", label: "Chat lorebooks", chatId: target.chatId, path: chatPath(target.characterId, target.chatId) + "/lorebooks" }); }
async function showLoreForMessage(messageId) { if (!activeCharacter || !activeChat) return; try { const context = await readJson(await fetch(chatPath(activeCharacter.id, activeChat.id) + "/messages/" + encodeURIComponent(messageId) + "/context"), "Context could not be loaded."); const lore = (context.lore || []).map((entry) => entry.name); const memory = (context.memories || []).map((entry) => entry.content); const labels = [...lore, ...memory]; showToast(labels.length ? "Context used: " + labels.join(" · ") : "No lore or memory was used for this response."); } catch (error) { showToast(error.message); } }

async function loadMemories({ quiet = false } = {}) { try { memories = await readJson(await fetch("/api/memories"), "Memories could not be loaded."); renderMemoryList(); return memories; } catch (error) { if (!quiet) showToast(error.message); throw error; } }
function characterNameForMemory(id) { return characters.find((character) => character.id === id)?.name || "Deleted character"; }
function renderMemoryList() { const list = document.querySelector("#memory-list"); list.replaceChildren(); if (!memories.length) { const empty = document.createElement("p"); empty.className = "field-copy"; empty.textContent = "No memories yet. Extract them from a chat or add one manually."; list.append(empty); return; } memories.forEach((memory) => { const card = document.createElement("article"); card.className = "lore-entry-card"; const content = document.createElement("strong"); content.textContent = memory.content; const details = document.createElement("span"); details.textContent = characterNameForMemory(memory.character_id) + " · " + (memory.scope === "character" ? "Character" : "Chat") + " · Importance " + memory.importance; const edit = document.createElement("button"); edit.className = "text-button"; edit.type = "button"; edit.textContent = "Edit"; edit.addEventListener("click", () => { void openMemoryEditor(memory); }); card.append(content, details, edit); list.append(card); }); }
async function openMemoryManager() { try { await loadCharacters({ quiet: true }); await loadMemories({ quiet: true }); memoryDialog.showModal(); } catch (error) { showToast(error.message); } }
async function refreshMemoryChatChoices(characterId, selectedChatId = "") { const select = document.querySelector("#memory-chat"); select.replaceChildren(new Option("Choose a chat", "")); if (!characterId) return; try { const chats = await loadChatsForCharacter(characterId, { quiet: true }); chats.forEach((chat) => select.add(new Option(chat.title, chat.id))); select.value = chats.some((chat) => chat.id === selectedChatId) ? selectedChatId : ""; } catch { showStatus(document.querySelector("#memory-editor-status"), "Chats could not be loaded for this character.", "error"); } }
async function openMemoryEditor(memory = null) {
  editingMemory = memory; const characterSelect = document.querySelector("#memory-character"); characterSelect.replaceChildren(new Option("Choose a character", "")); characters.forEach((character) => characterSelect.add(new Option(character.name, character.id)));
  characterSelect.value = memory?.character_id || activeCharacter?.id || ""; document.querySelector("#memory-scope").value = memory?.scope || (activeChat ? "chat" : "character"); document.querySelector("#memory-content").value = memory?.content || ""; document.querySelector("#memory-importance").value = memory?.importance || 60; document.querySelector("#memory-editor-title").textContent = memory ? "Edit memory" : "New memory"; document.querySelector("#delete-memory").hidden = !memory; showStatus(document.querySelector("#memory-editor-status")); await refreshMemoryChatChoices(characterSelect.value, memory?.chat_id || activeChat?.id || ""); updateMemoryScopeControls(); closeDialog(memoryDialog); memoryEditorDialog.showModal(); document.querySelector("#memory-content").focus();
}
function updateMemoryScopeControls() { const character = document.querySelector("#memory-character").value; const chat = document.querySelector("#memory-chat"); const chatScoped = document.querySelector("#memory-scope").value === "chat"; chat.disabled = !chatScoped; if (!character) chat.value = ""; }
async function saveMemory(event) { event.preventDefault(); const status = document.querySelector("#memory-editor-status"); const characterId = document.querySelector("#memory-character").value; const draft = { scope: document.querySelector("#memory-scope").value, chat_id: document.querySelector("#memory-scope").value === "chat" ? document.querySelector("#memory-chat").value || null : null, content: document.querySelector("#memory-content").value, importance: Number(document.querySelector("#memory-importance").value) }; if (!characterId) { showStatus(status, "Choose a character for this memory.", "error"); return; } if (draft.scope === "chat" && !draft.chat_id) { showStatus(status, "Choose a chat for this chat-scoped memory.", "error"); return; } showStatus(status, "Saving memory…"); try { const path = editingMemory ? "/api/memories/" + encodeURIComponent(editingMemory.id) : "/api/memories"; const body = editingMemory ? draft : { character_id: characterId, ...draft }; await readJson(await fetch(path, { method: editingMemory ? "PUT" : "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) }), "Memory could not be saved."); closeDialog(memoryEditorDialog); await loadMemories({ quiet: true }); memoryDialog.showModal(); showToast("Memory saved."); } catch (error) { showStatus(status, error.message, "error"); } }
async function deleteEditingMemory() { if (!editingMemory || !window.confirm("Delete this memory?")) return; try { await readJson(await fetch("/api/memories/" + encodeURIComponent(editingMemory.id), { method: "DELETE" }), "Memory could not be deleted."); closeDialog(memoryEditorDialog); await loadMemories({ quiet: true }); memoryDialog.showModal(); showToast("Memory deleted."); } catch (error) { showStatus(document.querySelector("#memory-editor-status"), error.message, "error"); } }
async function extractMemoriesFromActiveChat() { const target = selectedChatAction(); if (!target) return; closeDialog(chatActionsDialog); showChatStatus("Extracting durable memories locally…"); try { const result = await readJson(await fetch(chatPath(target.characterId, target.chatId) + "/memories/extract", { method: "POST" }), "Memories could not be extracted."); showChatStatus(result.created ? ("Created " + result.created + " memory" + (result.created === 1 ? "." : " entries.")) : "No new durable memories found."); } catch (error) { showChatStatus(error.message, "error"); } }

function requestChangeChatPersona() { const target = selectedChatAction(); if (!target) return; closeDialog(chatActionsDialog); const select = document.querySelector("#chat-persona-select"); select.replaceChildren(new Option("No persona — User", "")); personas.forEach((persona) => select.add(new Option(persona.name, persona.id, false, target.personaId === persona.id))); if (!target.personaId) select.value = ""; chatPersonaDialog.dataset.characterId = target.characterId; chatPersonaDialog.dataset.chatId = target.chatId; showStatus(document.querySelector("#chat-persona-status")); chatPersonaDialog.showModal(); }
async function confirmChatPersona() { const characterId = chatPersonaDialog.dataset.characterId; const chatId = chatPersonaDialog.dataset.chatId; if (!characterId || !chatId) return; const button = document.querySelector("#confirm-chat-persona"); button.disabled = true; showStatus(document.querySelector("#chat-persona-status"), "Updating persona…"); try { const personaId = document.querySelector("#chat-persona-select").value || null; const detail = await readJson(await fetch(chatPath(characterId, chatId) + "/persona", { method: "PUT", headers: { "Content-Type": "application/json" }, body: JSON.stringify({ persona_id: personaId }) }), "Chat persona could not be changed."); applyChatDetail(detail); closeDialog(chatPersonaDialog); chatActionTarget = null; showToast("Future responses now use " + (detail.persona ? detail.persona.name : "User") + "."); } catch (error) { showStatus(document.querySelector("#chat-persona-status"), error.message, "error"); } finally { button.disabled = false; } }

document.querySelector("#open-drawer").addEventListener("click", openDrawer);
document.querySelector("#close-drawer").addEventListener("click", closeDrawer);
backdrop.addEventListener("click", closeDrawer);
document.querySelector("#open-settings").addEventListener("click", showSettings);
document.querySelector("#close-settings").addEventListener("click", showChat);
document.querySelector("#open-personas-settings").addEventListener("click", showPersonas);
document.querySelector("#open-lorebooks-settings").addEventListener("click", showLorebooks);
document.querySelector("#open-memory-manager").addEventListener("click", () => { void openMemoryManager(); });
document.querySelector("#close-memory-manager").addEventListener("click", () => closeDialog(memoryDialog));
document.querySelector("#new-memory").addEventListener("click", () => { void openMemoryEditor(); });
document.querySelector("#cancel-memory-editor").addEventListener("click", () => { closeDialog(memoryEditorDialog); memoryDialog.showModal(); });
document.querySelector("#delete-memory").addEventListener("click", () => { void deleteEditingMemory(); });
document.querySelector("#memory-character").addEventListener("change", (event) => { void refreshMemoryChatChoices(event.target.value); });
document.querySelector("#memory-scope").addEventListener("change", updateMemoryScopeControls);
memoryEditorDialog.querySelector("form").addEventListener("submit", (event) => { void saveMemory(event); });
memorySettingsForm.addEventListener("submit", (event) => { event.preventDefault(); void saveMemorySettings(); });
document.querySelector("#rebuild-memory-index").addEventListener("click", () => { void rebuildMemoryIndex(); });
document.querySelector("#close-lorebooks").addEventListener("click", showSettings);
document.querySelector("#new-lorebook").addEventListener("click", () => { void openLorebookEditor(); });
document.querySelector("#new-lorebook-inline").addEventListener("click", () => { void openLorebookEditor(); });
document.querySelector("#import-lorebook").addEventListener("click", () => { document.querySelector("#import-lorebook-file").click(); });
document.querySelector("#import-lorebook-file").addEventListener("change", () => { void importLorebook(); });
document.querySelector("#close-lorebook-editor").addEventListener("click", showLorebooks);
document.querySelector("#cancel-lorebook").addEventListener("click", showLorebooks);
document.querySelector("#export-lorebook").addEventListener("click", () => { void exportLorebook(); });
document.querySelector("#new-lore-entry-inline").addEventListener("click", () => { if (activeLorebook?.id) void openLoreEntryEditor(activeLorebook.id); });
document.querySelector("#close-lore-entry-editor").addEventListener("click", () => { if (activeLorebook?.id) void openLorebookEditor(activeLorebook.id); else showLorebooks(); });
document.querySelector("#cancel-lore-entry").addEventListener("click", () => { if (activeLorebook?.id) void openLorebookEditor(activeLorebook.id); else showLorebooks(); });
document.querySelector("#action-manage-character-lore").addEventListener("click", requestCharacterLorebooks);
document.querySelector("#action-manage-chat-lore").addEventListener("click", requestChatLorebooks);
document.querySelector("#action-extract-memories").addEventListener("click", () => { void extractMemoriesFromActiveChat(); });
document.querySelector("#cancel-lorebook-association").addEventListener("click", () => closeDialog(lorebookAssociationDialog));
document.querySelector("#confirm-lorebook-association").addEventListener("click", () => { void saveLorebookAssociation(); });
document.querySelector("#close-lorebook-actions").addEventListener("click", () => closeDialog(lorebookActionsDialog));
document.querySelector("#action-edit-lorebook").addEventListener("click", editSelectedLorebook);
document.querySelector("#action-duplicate-lorebook").addEventListener("click", () => { void duplicateSelectedLorebook(); });
document.querySelector("#action-delete-lorebook").addEventListener("click", () => { void deleteSelectedLorebook(); });
document.querySelector("#open-characters").addEventListener("click", showCharacters);
document.querySelector("#new-character-sidebar").addEventListener("click", () => { void openCharacterEditor(); });
document.querySelector("#import-character-sidebar").addEventListener("click", showImportCharacter);
document.querySelector("#new-character-empty").addEventListener("click", () => { void openCharacterEditor(); });
document.querySelector("#close-characters").addEventListener("click", showChat);
document.querySelector("#generate-character").addEventListener("click", () => { void openCharacterCreator(); });
document.querySelector("#new-character-inline").addEventListener("click", () => { void openCharacterEditor(); });
document.querySelector("#import-character").addEventListener("click", showImportCharacter);
document.querySelector("#close-import").addEventListener("click", showCharacters);
document.querySelector("#cancel-import").addEventListener("click", showCharacters);
document.querySelector("#close-character-editor").addEventListener("click", leaveCharacterEditor);
document.querySelector("#cancel-character").addEventListener("click", leaveCharacterEditor);
document.querySelector("#close-personas").addEventListener("click", showSettings);
document.querySelector("#new-persona").addEventListener("click", () => { void openPersonaEditor(); });
document.querySelector("#new-persona-inline").addEventListener("click", () => { void openPersonaEditor(); });
document.querySelector("#close-persona-editor").addEventListener("click", leavePersonaEditor);
document.querySelector("#cancel-persona").addEventListener("click", leavePersonaEditor);
document.querySelector("#chat-actions").addEventListener("click", openChatActions);
document.querySelector("#close-character-actions").addEventListener("click", () => closeDialog(actionsDialog));
document.querySelector("#action-new-chat").addEventListener("click", createChatForSelectedCharacter);
document.querySelector("#action-edit").addEventListener("click", () => { const id = actionCharacterId; closeDialog(actionsDialog); if (id) void openCharacterEditor(id); });
document.querySelector("#action-duplicate").addEventListener("click", () => { void duplicateSelectedCharacter(); });
document.querySelector("#action-delete").addEventListener("click", requestDeleteSelectedCharacter);
document.querySelector("#cancel-delete-character").addEventListener("click", () => closeDialog(deleteDialog));
document.querySelector("#confirm-delete-character").addEventListener("click", () => { void confirmDeleteCharacter(); });
document.querySelector("#close-chat-actions").addEventListener("click", () => closeDialog(chatActionsDialog));
document.querySelector("#action-rename-chat").addEventListener("click", requestRenameChat);
document.querySelector("#action-generate-scene").addEventListener("click", () => { void generateSceneImage(); });
document.querySelector("#action-stop-scene").addEventListener("click", () => { void stopSceneImage(); });
document.querySelector("#action-change-chat-persona").addEventListener("click", requestChangeChatPersona);
document.querySelector("#action-delete-chat").addEventListener("click", requestDeleteChat);
document.querySelector("#cancel-rename-chat").addEventListener("click", () => closeDialog(renameChatDialog));
document.querySelector("#confirm-rename-chat").addEventListener("click", () => { void confirmRenameChat(); });
document.querySelector("#cancel-delete-chat").addEventListener("click", () => closeDialog(deleteChatDialog));
document.querySelector("#confirm-delete-chat").addEventListener("click", () => { void confirmDeleteChat(); });
document.querySelector("#cancel-new-chat").addEventListener("click", () => closeDialog(newChatDialog));
document.querySelector("#new-chat-greeting").addEventListener("change", updateNewChatGreetingPreview);
document.querySelector("#confirm-new-chat").addEventListener("click", () => { if (newChatCharacter) void createChatWithSetup(newChatCharacter.id, document.querySelector("#new-chat-greeting").value || null, selectedNewChatPersonaId()); });
document.querySelector("#cancel-chat-persona").addEventListener("click", () => closeDialog(chatPersonaDialog));
document.querySelector("#confirm-chat-persona").addEventListener("click", () => { void confirmChatPersona(); });
document.querySelector("#close-persona-actions").addEventListener("click", () => closeDialog(personaActionsDialog));
document.querySelector("#action-set-default-persona").addEventListener("click", () => { void setDefaultPersona(); });
document.querySelector("#action-edit-persona").addEventListener("click", () => { const id = personaActionId; closeDialog(personaActionsDialog); if (id) void openPersonaEditor(id); });
document.querySelector("#action-duplicate-persona").addEventListener("click", () => { void duplicateSelectedPersona(); });
document.querySelector("#action-delete-persona").addEventListener("click", requestDeleteSelectedPersona);
document.querySelector("#cancel-delete-persona").addEventListener("click", () => closeDialog(deletePersonaDialog));
document.querySelector("#confirm-delete-persona").addEventListener("click", () => { void confirmDeletePersona(); });

appearanceForm.querySelectorAll(".color-picker").forEach((picker) => picker.addEventListener("input", () => { const hex = appearanceForm.querySelector('.hex-input[data-appearance-field="' + picker.dataset.appearanceField + '"]'); if (hex) hex.value = picker.value.toUpperCase(); applyAppearancePreview(); }));
appearanceForm.querySelectorAll(".hex-input").forEach((input) => { input.addEventListener("input", () => { const value = normalizeHexColor(input.value); input.setAttribute("aria-invalid", String(!value)); if (value) { const picker = appearanceForm.querySelector('.color-picker[data-appearance-field="' + input.dataset.appearanceField + '"]'); if (picker) picker.value = value; applyAppearancePreview(); } }); input.addEventListener("blur", () => { if (!normalizeHexColor(input.value)) showStatus(appearanceStatus, "Use six-digit hex colors, such as #C9A227.", "error"); }); });
document.querySelector("#reset-theme").addEventListener("click", () => { syncAppearanceControls(defaultAppearance); setAppearance(defaultAppearance); showStatus(appearanceStatus, "NyxAI default colors restored. Save to keep them.", "success"); });
appearanceForm.addEventListener("submit", async (event) => { event.preventDefault(); const appearance = appearanceFromControls(); if (!appearance) { showStatus(appearanceStatus, "Use six-digit hex colors, such as #C9A227.", "error"); return; } const persistedAppearance = appSettings.appearance; appSettings.appearance = appearance; showStatus(appearanceStatus, "Saving…"); try { await persistSettings(); showStatus(appearanceStatus, "Appearance saved locally.", "success"); } catch (error) { appSettings.appearance = persistedAppearance; showStatus(appearanceStatus, error.message, "error"); } });
connectionForm.addEventListener("submit", async (event) => { event.preventDefault(); try { await saveConnection(); } catch (error) { setConnectionState("error"); showStatus(connectionStatus, error.message, "error"); } });
document.querySelector("#test-connection").addEventListener("click", async () => { try { await saveConnection({ testAfterSave: true }); } catch (error) { setConnectionState("error"); showStatus(connectionStatus, error.message, "error"); } });
document.querySelector("#text-provider").addEventListener("change", () => { appSettings.provider.active_provider = document.querySelector("#text-provider").value; syncTextBackendControls(); showStatus(connectionStatus, "Save this backend before testing or refreshing models."); });
document.querySelector("#refresh-models").addEventListener("click", () => { void loadModels(); });
modelSelect.addEventListener("change", () => { if (modelSelect.value) { document.querySelector("#manual-model").value = modelSelect.value; showStatus(modelStatus, "Save this selection to use it."); } });
document.querySelector("#save-model").addEventListener("click", () => { void selectModel(); });
document.querySelector("#memory-embedding-provider").addEventListener("change", () => { availableEmbeddingModels = []; populateMemoryModelChoices(); showStatus(document.querySelector("#memory-settings-status"), "Save the embedding backend before refreshing its models."); });
document.querySelector("#memory-embedding-model").addEventListener("change", () => { if (document.querySelector("#memory-embedding-model").value) document.querySelector("#memory-embedding-model-manual").value = document.querySelector("#memory-embedding-model").value; });
characterCreatorModelSettings.addEventListener("change", () => { document.querySelector("#save-character-creator-model").disabled = !availableModels.length; if (characterCreatorModelSettings.value) showStatus(characterCreatorModelStatus, "Save this selection to use it for character drafts."); });
document.querySelector("#save-character-creator-model").addEventListener("click", () => { void saveCharacterCreatorModel(); });
document.querySelector("#test-image-connection").addEventListener("click", () => { void saveImageSettings({ testAfterSave: true }); });
document.querySelector("#refresh-image-models").addEventListener("click", () => { void loadImageModels(); });
imageSettingsForm.addEventListener("submit", (event) => { event.preventDefault(); void saveImageSettings(); });
generationForm.addEventListener("submit", async (event) => { event.preventDefault(); appSettings.generation = { temperature: Number(document.querySelector("#temperature").value), context_length: Number(document.querySelector("#context-length").value), max_response_length: Number(document.querySelector("#max-response-length").value) }; showStatus(document.querySelector("#generation-status"), "Saving…"); try { await persistSettings(); showStatus(document.querySelector("#generation-status"), "Generation settings saved.", "success"); } catch (error) { showStatus(document.querySelector("#generation-status"), error.message, "error"); } });
composer.addEventListener("submit", (event) => { event.preventDefault(); const message = messageInput.value.trim(); if (!message) return; messageInput.value = ""; resizeComposerInput(); void sendMessage(message); });
messageInput.addEventListener("input", resizeComposerInput);
messageInput.addEventListener("keydown", (event) => {
  // Desktop gets quick Enter-to-send while phones retain Enter for natural
  // multiline composition. Shift+Enter always inserts a newline.
  if (event.key === "Enter" && !event.shiftKey && window.matchMedia("(pointer: fine)").matches) {
    event.preventDefault();
    composer.requestSubmit();
  }
});
stopButton.addEventListener("click", () => { if (activeController) { showChatStatus(appSettings.capabilities.generation_cancellation === false ? "Stopping NyxAI's stream; this backend may continue work server-side." : "Stopping generation…"); activeController.abort(); } });
characterForm.addEventListener("submit", (event) => { void saveCharacter(event); });
 lorebookForm.addEventListener("submit", (event) => { void saveLorebook(event); });
 loreEntryForm.addEventListener("submit", (event) => { void saveLoreEntry(event); });
personaForm.addEventListener("submit", (event) => { void savePersona(event); });
importForm.addEventListener("submit", (event) => { void importCharacterCard(event); });
characterForm.querySelectorAll("input:not([type=hidden]), textarea").forEach((field) => field.addEventListener("input", () => { if (field.id === "character-name" && !editorAvatarUrl) avatarFallback.textContent = initials(field.value); markEditorDirty(); }));
personaForm.querySelectorAll("input:not([type=hidden]), textarea").forEach((field) => field.addEventListener("input", () => { if (field.id === "persona-name" && !personaEditorAvatarUrl) personaAvatarFallback.textContent = initials(field.value); personaEditorDirty = true; }));
avatarInput.addEventListener("change", () => { void uploadSelectedAvatar(); });
document.querySelector("#generate-avatar").addEventListener("click", () => { void generateAvatarCandidate(); });
document.querySelector("#discard-generated-avatar").addEventListener("click", () => { void discardGeneratedAvatarCandidate(); closeDialog(generatedAvatarDialog); });
document.querySelector("#regenerate-generated-avatar").addEventListener("click", () => { closeDialog(generatedAvatarDialog); void generateAvatarCandidate(); });
document.querySelector("#use-generated-avatar").addEventListener("click", useGeneratedAvatarCandidate);
personaAvatarInput.addEventListener("change", () => { void uploadPersonaAvatar(); });
document.querySelector("#remove-avatar").addEventListener("click", removeEditorAvatar);
document.querySelector("#remove-persona-avatar").addEventListener("click", removePersonaEditorAvatar);
document.querySelector("#add-tag").addEventListener("click", addEditorTag);
document.querySelector("#add-alternate-greeting").addEventListener("click", addAlternateGreeting);
document.querySelector("#cancel-character-creator").addEventListener("click", () => { if (!isCharacterCreatorGenerating) closeDialog(characterCreatorDialog); });
document.querySelector("#confirm-character-creator").addEventListener("click", () => { void generateCharacterDraft(); });
document.querySelector("#cancel-field-regeneration").addEventListener("click", () => { if (!isCharacterCreatorGenerating) closeDialog(fieldRegenerationDialog); });
document.querySelector("#confirm-field-regeneration").addEventListener("click", () => { void regenerateCharacterField(); });
document.querySelector("#regenerate-alternate-greetings").addEventListener("click", () => requestFieldRegeneration("alternate_greetings"));
document.querySelectorAll(".field-generate").forEach((button) => button.addEventListener("click", () => requestFieldRegeneration(button.dataset.creatorField)));
tagInput.addEventListener("keydown", (event) => { if (event.key === "Enter") { event.preventDefault(); addEditorTag(); } });
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape") return;
  if (!isCharacterCreatorGenerating && fieldRegenerationDialog.open) closeDialog(fieldRegenerationDialog); else if (!isCharacterCreatorGenerating && characterCreatorDialog.open) closeDialog(characterCreatorDialog); else if (newChatDialog.open) closeDialog(newChatDialog); else if (chatPersonaDialog.open) closeDialog(chatPersonaDialog); else if (deletePersonaDialog.open) closeDialog(deletePersonaDialog); else if (personaActionsDialog.open) closeDialog(personaActionsDialog); else if (deleteChatDialog.open) closeDialog(deleteChatDialog); else if (renameChatDialog.open) closeDialog(renameChatDialog); else if (chatActionsDialog.open) closeDialog(chatActionsDialog); else if (deleteDialog.open) closeDialog(deleteDialog); else if (actionsDialog.open) closeDialog(actionsDialog); else if (!backdrop.hidden) closeDrawer();
});

document.querySelector("#retry-server").addEventListener("click", () => { void checkServerHealth().then((available) => { if (available) { void Promise.all([loadSettings(), loadCharacters(), loadPersonas({ quiet: true })]); showToast("NyxAI server is available again."); } }); });
window.addEventListener("offline", () => setServerAvailability(false));
window.addEventListener("online", () => { void checkServerHealth({ quiet: true }); });
function updateViewportHeight() { document.documentElement.style.setProperty("--nyx-app-height", ((window.visualViewport && window.visualViewport.height) || window.innerHeight) + "px"); }
updateViewportHeight();
if (window.visualViewport) { window.visualViewport.addEventListener("resize", updateViewportHeight); window.visualViewport.addEventListener("scroll", updateViewportHeight); }
window.addEventListener("resize", updateViewportHeight);
if ("serviceWorker" in navigator && (window.isSecureContext || location.hostname === "localhost" || location.hostname === "127.0.0.1")) {
  window.addEventListener("load", () => { navigator.serviceWorker.register("/sw.js").catch(() => {}); });
}

setConnectionState(); updateChatHeader(); renderConversation(); setComposerState();
void checkServerHealth({ quiet: true });
void loadAbout();
void loadSettings();
void loadCharacters();
void loadPersonas();
