# NyxAI

NyxAI is a self-hosted, mobile-first web interface for local AI backends and characters. It is designed for a private LAN or Tailscale network. Ollama is the first provider; NyxAI does not run models itself.

## Current status

Milestones 1–13 are implemented:

- Responsive Rust/Axum web application with SQLite settings, appearance preferences, Docker deployment, and a phone-first chat shell.
- Configurable Ollama connection, model discovery/selection, persisted generation controls, server-side streaming responses, and Stop generation.
- Persistent character CRUD: creation, editing, duplication, confirmed deletion, avatars, tags, and a real database-backed sidebar.
- Persistent character chats: multiple independent conversations per character, SQLite-backed messages, character-aware Ollama prompting, streaming response saves, cancellation-safe partial responses, chat rename, and confirmed chat deletion.
- Final phone-first character/chat navigation: independently expandable character groups, nested persisted chats, per-chat action menus, active-chat affordances, and an independently scrolling mobile drawer.
- Character-card import for JSON and supported PNG metadata, centralized normalization into NyxAI characters, editable creator notes, and selectable alternate greetings for new chats.
- Persisted, live-previewed chat appearance controls with the NyxAI default palette, plus safe render-time roleplay formatting for actions and dialogue.
- First-class personas, per-chat persona selection, default-persona support, and dynamic `{{char}}` / `{{user}}` resolution at prompt and greeting time.
- PWA metadata, a privacy-safe offline shell, a responsive multiline composer, server-unavailable feedback, health checks, and Docker production polish.
- A local AI Character Creator that turns a natural-language idea into a reviewable, editable character draft through the configured Ollama model—without saving anything until you explicitly use the normal character save flow.
- Local A1111-compatible text-to-image generation for reviewable character-avatar candidates and optional persisted in-chat scene images, with an independent image-model setting and safe local file storage.
- Local, deterministic Lorebooks / World Info: reusable trigger-key entries can be attached to characters or individual chats and are included in context only when relevant.
- Local Semantic Memory / RAG: concise durable facts are extracted with a local text model, embedded through Ollama, stored in SQLite, and retrieved with local cosine similarity for the active character and chat only.

Card export and search are intentionally not part of this release.

## Architecture

The browser communicates with NyxAI's HTTP routes. Ollama-specific behavior remains isolated:

```text
Browser UI -> NyxAI routes -> prompt builder -> ProviderService -> OllamaProvider -> Ollama API
```

Characters, personas, and chats follow the same separation: normalized domain models and prompt construction are in `src/domain`, SQLite records and avatar file handling are in `src/storage`, and `src/app/routes.rs` owns the HTTP boundary. The UI does not access SQLite or Ollama directly.

## Semantic Memory

Semantic Memory is a local, embedding-based complement to deterministic Lorebooks. Lorebooks inject explicit world facts when keys match; Semantic Memory recalls concise, durable facts from a specific conversation, such as a promise, a revealed preference, a relationship change, or an important event. It is not cloud RAG, a vector-database service, full transcript search, or a replacement for character context.

Open **Settings → Semantic Memory**, enable it, and select an installed Ollama **Embedding model**. This model is required because chat models are not silently used for embeddings. Choose an optional **Extraction model**; if it is empty, NyxAI uses the Character Creator Model and then the selected chat model as a local fallback. The settings also control automatic extraction (default: every 8 new messages), the number of retrieved memories, similarity threshold, and a conservative estimated-token budget.

After an assistant response finishes, NyxAI may asynchronously inspect a bounded batch of the newest unprocessed messages. A local text model returns JSON facts; NyxAI validates, de-duplicates, embeds, and saves only concise candidates. Extraction never delays streamed chat output and is skipped while another local AI task owns the text/image resource. Use **Chat actions → Extract memories now** to process a chat on demand. The **Manage memories** sheet supports creating, editing, deleting, and promoting a fact from **This chat only** to **All chats for this character**.

Each entry has a `chat` or `character` scope. Chat memories are retrieved only for that exact chat; character memories can be shared by the character's chats. Retrieval embeds a bounded recent conversation query, compares it against only matching stored vectors with Rust cosine similarity, applies the selected threshold plus a small importance tie-breaker, and keeps the top matches within the configured budget. The resulting text is placed in a separate `[Relevant Semantic Memory]` prompt section after character/persona and lore context. It never mutates stored character, lorebook, or message text. `{{char}}` and `{{user}}` remain raw in memory storage and resolve only for the active prompt.

Changing the embedding model does not discard existing memory text, but vectors from a different model are not used. Select the new model and choose **Rebuild index** to re-embed saved entries locally. Assistant messages record the exact memory IDs used for that generation; their **Context** action shows both lore and semantic memories. SQLite migration `0008_semantic_memory.sql` adds `memory_entries`, binary `memory_vectors`, extraction progress, and message-context join tables. Deleting a chat or character cascades its scoped memories safely. Existing installs simply start with no memories and remain fully functional when Semantic Memory is disabled.

## Lorebooks / World Info

Open **Settings → Manage lorebooks** to create a named collection of reusable facts. Each entry has a name, content, enabled state, numeric priority, one or more trigger keys, **Any key** or **All keys** matching, and optional **Case-sensitive** or **Always Active** behavior. For example, an entry named *Blackwood Village* with the keys `blackwood` and `blackwood village` can explain the village whenever recent chat text mentions either phrase. Matching is case-insensitive by default and uses word/phrase boundaries, so `vale` does not activate inside `available`.

Attach enabled lorebooks from a character’s action sheet to share them across that character’s conversations, or from a chat’s action sheet for a single conversation. A lorebook may be attached in both places but its entries are deduplicated. Lore is never global by default. Existing characters and chats remain unchanged until you explicitly attach a lorebook.

During generation, NyxAI inspects only the newest six messages, selects enabled Always Active and triggered entries deterministically, sorts them by priority, and reserves at most one third of the available input context (capped at 1,024 estimated tokens). Higher-priority entries survive first; older conversation is still trimmed before the character/persona system context. Selected entries appear in a separate `[Relevant World Information]` prompt section. `{{char}}` and `{{user}}` stay raw in stored lore content and resolve only for the active prompt. This is deterministic keyword context, not embeddings, semantic search, RAG, or long-term memory.

Assistant messages with recorded lore have a **Context** action that lists the entry names used for that response. Export a lorebook as the versioned `nyxai-lorebook` JSON format from its editor; use **Import** in the lorebook list to bring it into another NyxAI installation. Imports validate every entry before writing, preserve metadata and matching settings, and choose a duplicate-safe name instead of overwriting an existing lorebook. Character-card imports without recognized lore metadata continue to work normally; third-party world-info schemas are intentionally not converted in this release.

## Character system

Open the navigation drawer and select **Manage**. Create a character with basic information, then fill the independent prompt fields as needed:

- Name and optional avatar
- Description
- Personality, scenario, and first message
- Advanced example dialogue, system prompt, and tags

The editor is organized into small phone-friendly sections. Advanced fields are collapsible. Character rows have a touch-friendly action sheet for creating a chat, editing, duplicating, or deleting. Deletion uses an explicit confirmation dialog.

Character fields are normalized into NyxAI's own model rather than any external character-card format. Each character has an ID, timestamps, avatar reference, independent prompt fields, creator notes, tags, and zero or more alternate greetings. The card importer maps external data into that model without changing chat or provider code.

### Database schema

Migration `0002_character_system.sql` adds a relational `characters` table. It stores each primary field in its own column: ID, name, avatar path, description, personality, scenario, first message, example dialogue, system prompt, timestamps, and `tags_json`. Migration `0004_character_card_compatibility.sql` adds `creator_notes` and a related `character_greetings` table. Each alternate greeting has its own ID, character foreign key, position, and content; deleting a character cascades safely to its greetings. Tags remain the sole compact JSON field.

The sidebar is character-backed. Tapping a character expands or collapses only that character's persisted chats; it never opens a conversation itself. Multiple character groups can stay expanded. **New Chat** creates and opens a chat, while tapping a chat opens that exact conversation. Each chat has its own compact overflow menu for rename and confirmed deletion.

The selected chat has a dark-purple surface, gold edge accent, stronger type, a visible current-chat marker, and `aria-current="page"`; active state does not rely on color alone. The drawer closes after opening or creating a chat, locks background scrolling while open, and lets long character/chat lists scroll inside the drawer.

### Avatar storage

Avatar bytes are not stored in SQLite. NyxAI validates PNG, JPEG, GIF, and WebP signatures, limits uploads to 4 MB, creates UUID-based filenames, and stores only that controlled filename in SQLite. The public image URL is served from `/avatars/<generated-name>`; user filenames and paths are never trusted.

When a character is duplicated, its avatar file is copied to a new generated filename, so either character can later be deleted or changed independently. Replacing, removing, or deleting a character removes the formerly associated avatar file after the database update succeeds.

`NYXAI_ASSET_DIR` controls the persistent avatar directory. If omitted in a normal file-based local setup, it defaults to an `avatars` directory beside the database. Docker uses `/data/avatars`, which is inside the existing persistent `nyxai_data` volume.

### Character-card import and greetings

Use **Import** from Character management or the import icon in the navigation drawer. NyxAI accepts JSON card files and PNG cards with a supported `chara` metadata chunk. Importing creates a separate character rather than overwriting an existing same-named character; a duplicate-safe suffix such as `Nyx (2)` is used when needed. Imported characters appear in the sidebar immediately and can be edited, duplicated, deleted, prompted, and chatted with like manually created characters.

The importer is deliberately isolated from the character domain: file format parser → normalized import draft → normal character persistence. Currently supported common fields include `name`/`char_name`, `description`, `personality`/`char_persona`, `scenario`/`world_scenario`, `first_mes`/`greeting`/`first_message`, `alternate_greetings`, `mes_example`/`example_dialogue`, `system_prompt`, `creator_notes`, and `tags`. Character Card V2-shaped JSON with its values in `data` is supported. Unknown fields are ignored safely.

Imports are deliberately permissive. A readable card with a recognizable character field imports even if all other optional fields are missing, `null`, or malformed. Missing text becomes an empty string and missing collections become empty lists. If no usable name is available, NyxAI uses **Imported Character**. Single-string tags and alternate greetings are normalized; invalid collection entries are skipped. The import result opens the editor with non-blocking warnings explaining missing, shortened, or ignored fields. Only unreadable/corrupt files or JSON that is unrelated to a recognizable character-card structure are rejected.

PNG support reads a base64 (or raw JSON) `chara` value from standard PNG `tEXt` chunks or uncompressed `iTXt` chunks, then stores the PNG itself through the normal generated-filename avatar system. Compressed `zTXt`/compressed `iTXt` metadata, unsupported PNG metadata encodings, raw metadata dumps, and card export are not supported in this milestone. NyxAI does not claim full compatibility with every third-party card dialect.

The editor’s collapsed **Greetings** section preserves, adds, edits, and removes alternate greetings. A character with no alternate greetings retains the one-tap new-chat flow. If alternates exist, NyxAI shows a compact greeting chooser. The chosen default or alternate greeting is stored once as that chat’s first assistant message; the other greetings never enter visible history or later prompts.

### AI Character Creator

From **Manage characters**, choose the moon-shaped **Generate Character with AI** control. Describe the character in ordinary language, choose an optional local creator-model override, set the requested alternate-greeting count (0–10; 3 by default), and select **Generate draft**. NyxAI asks the configured local provider for a structured `CharacterDraft`, then opens the normal character editor with the result. Nothing is written to SQLite during this step: review or edit every field and use the editor's usual **Save character** action to persist it.

Set a dedicated **Character Creator Model** in **Settings** when you want character drafting to use a different locally installed model. If it is unset, NyxAI falls back to the globally selected chat model. The generator dialog can also use a one-time available-model override without changing the saved setting. Model discovery and all generation continue through NyxAI's provider service, so no extra Ollama client or cloud service is introduced.

Generated drafts keep the original creation request only while the unsaved editor session is open. Small moon controls on generated fields can regenerate Description, Personality, Scenario, First Message, Alternate Greetings, Example Dialogue, System Prompt, or Tags. The request includes the current draft, the original idea, and an optional direction, but the server applies only the requested returned field—so manual edits elsewhere are retained. Generated and regenerated values remain normal character data: `{{char}}`, `{{user}}`, and unknown placeholders are stored literally and are resolved later by the existing chat template system.

NyxAI requests JSON-only draft output, tolerates fenced JSON and harmless surrounding text, and validates it using the same normalized character draft rules as the editor. If a full draft has invalid structure, NyxAI makes one bounded repair request. A second invalid response produces a clear error; it never creates a partial character, overwrites an existing character, or discards the typed idea. Some smaller local models may still struggle to follow JSON instructions; choose a stronger instruction-following local model if that happens repeatedly.

## Personas and template variables

Open **Settings** → **Manage personas** to create an in-story user identity with a name, optional avatar, and optional description. Personas use the same generated-filename avatar storage as characters. You can edit, duplicate, delete, and set one as the default. Deleting a persona never deletes chats or messages: its references are cleared and those chats use the safe **User** fallback for future prompt construction.

Each chat stores its own persona reference. A default persona is applied to a new chat unless you explicitly choose another identity or **No persona — User** in the compact new-chat sheet. Existing chats never change merely because the global default changes. Changing a persona on an existing chat is explicit, affects only future generations, and leaves existing messages and its already-resolved opening greeting unchanged. The active chat header identifies its current persona.

NyxAI supports these plain-text template variables in character fields used for greetings or prompts:

- `{{char}}` and `{{Char}}` resolve to the active character's name.
- `{{user}}` and `{{User}}` resolve to the active chat persona's name, or **User** if no persona is selected.

Resolution happens only when creating a visible greeting or building a provider request. It applies to description, personality, scenario, first/alternate greetings, example dialogue, and system prompt. Unknown variables such as `{{world}}` remain untouched. This is deliberately simple text replacement, not an expression language. Imported and manually created character fields—including placeholders—remain stored exactly as authored, so cards stay portable and the same character can be reused with different personas.

## Persistent chats and messages

Create a character, open its entry in the navigation drawer, and choose **New Chat**. A character can own any number of chats. Each one has its own title, timestamps, and message history; a chat can be renamed or deleted from the chat action menu. Deleting a chat removes only its messages, not its character or sibling chats.

Migration `0003_persistent_chats.sql` adds two relational tables:

- `chats`: ID, owning `character_id`, title, and timestamps. The character foreign key uses `ON DELETE CASCADE`.
- `messages`: ID, owning `chat_id`, role, content, optional generation ID, and timestamp. The chat foreign key uses `ON DELETE CASCADE`.

Messages load in chronological insertion order. The database never accepts an orphaned chat or message. Deleting a character is confirmed in the UI and cascades to its chats and messages, so there are no dangling conversations.

Migration `0005_personas_and_chat_personas.sql` adds a relational `personas` table (ID, name, avatar reference, description, timestamps) and a nullable `chats.persona_id` foreign key. Its `ON DELETE SET NULL` rule preserves the chat and its history if a persona is deleted. The global default persona ID lives in the existing persisted application settings record. Migration `0007_lorebooks.sql` adds relational lorebooks, lore entries, character/chat attachment tables, and recorded assistant-message lore context. Its foreign keys cascade safely when an owning lorebook, chat, or character is deleted.

When a chat is first created, a non-empty character **First message** is written once as its first assistant message. It is not recreated when the chat is reopened, after restart, or for every generation. It is safe to leave that field empty.

### Character-aware generation

Every generation reads the active character, the active chat's selected persona, and only that chat's persisted messages. A centralized prompt builder creates a non-visible system context with minimal roleplay framing, then the character's custom system prompt, description, personality, scenario, selected persona identity/description, example dialogue, and tags—blank sections are omitted. Example dialogue is explicitly framed as a style reference rather than visible conversation history. Template values are resolved once while this static context is built, never into the database.

NyxAI then appends only the selected chat's recent messages in chronological order. It never uses a global message array, sidebar placeholders, another chat, or another character's history. This keeps conversations isolated even when the same character has several chats.

Context length reserves the configured maximum response length first. Without adding a heavyweight tokenizer dependency, NyxAI uses a documented conservative approximation of roughly four characters per token plus small message overhead. It preserves character context and newest messages first, then drops older history when necessary. If the newest user message cannot fit, NyxAI explains that the context settings need adjustment instead of silently losing it.

Streaming output remains in browser memory while it arrives. NyxAI writes one final assistant message at completion rather than one SQLite row per token. Stop aborts the request, keeps useful partial text visible, and asks the server to persist that partial message once using a generation ID; a stream/completion race cannot duplicate it. Message bubbles provide a simple Copy action.

## Appearance and roleplay rendering

Open **Settings** → **Appearance** to customize the application background, secondary surface, user and character message backgrounds/text, accent, and muted text. Each control has a synchronized native color picker and six-digit hex input. Changes preview in the settings sample and across the open interface immediately; **Save Settings** persists them in the existing SQLite-backed application settings record. Leaving Settings without saving restores the last persisted appearance, while **Reset to NyxAI default** previews the original palette until it is saved.

The default palette is midnight black `#08080B`, secondary background `#101014`, dark purple `#21172D`, purple accent `#4A2C63`, gold accent `#C9A227`, bright gold `#E8C55A`, primary text `#F2EEF5`, and muted text `#9A94A3`. Invalid hex entries cannot be saved. If an older or manually edited settings record contains a bad color, NyxAI falls back to the default for that individual field instead of allowing the UI to break.

Messages retain their exact raw content in SQLite. At render time only, NyxAI recognizes `*action*` and `**action**` as restrained italic narration, including short segments such as `*word*` and `**word**`, and straight or curly quoted spans such as `"dialogue"` and `“dialogue”` as spoken dialogue. Mixed prose stays in its original order and spacing. Ambiguous or unclosed markers remain ordinary text. The parser emits text segments rather than HTML, so imported or generated message content is assigned as text and cannot execute markup; Copy always uses the original stored content.

## Local image generation

NyxAI v1.2 can generate images through an [AUTOMATIC1111-compatible API](https://github.com/AUTOMATIC1111/stable-diffusion-webui/wiki/API). Start the image backend with its API enabled (for AUTOMATIC1111 this is normally `--api`), then open **Settings → Image Generation**, enable it, save the private A1111 URL, test the connection, refresh checkpoint names, and optionally choose a checkpoint. A saved Settings URL takes priority over `A1111_BASE_URL`; the environment value is only the startup fallback.

NyxAI first asks the local text provider for a concise visual prompt. It uses **Image Prompt Model**, then **Character Creator Model**, then the selected chat model. Avatar generation creates a reviewable candidate in the character editor; it is not associated with a character until the normal character save flow is used. From a chat action sheet, **Generate scene image** creates one optional visual from the active character, persona, scenario, and a small recent-history window. Scene-image metadata is stored in SQLite and generated image files are stored under the persistent image directory. Deleting a chat or character removes associated image metadata and files; deleting a single scene image is also available in the conversation.

Image prompts and generated pixels stay on the configured private services. NyxAI does not add cloud image providers, image-to-image workflows, ControlNet, queues, galleries, or automatic message-image insertion. The optional **Single-GPU memory mode** makes best-effort requests to unload the active Ollama model before image generation, unload the A1111 checkpoint afterward, and restore the selected chat model when enabled. It serializes text and image generation in this NyxAI process; it cannot control other applications using the same GPU.

## Ollama configuration

In **Settings**, enter an Ollama server URL, save it, test the connection, refresh available models, and select one. The saved Settings URL takes precedence over `OLLAMA_BASE_URL`; when nothing is saved, the environment value is used, falling back to `http://localhost:11434` for local development.

Ollama's documented [chat API](https://docs.ollama.com/api/chat), [model listing](https://docs.ollama.com/api/tags), and [streaming format](https://docs.ollama.com/api/streaming) are used server-side. NyxAI translates streaming chunks to server-sent events, incrementally updates the active assistant message, and aborts the upstream response when Stop is used.

| Variable | Default | Purpose |
| --- | --- | --- |
| `NYXAI_PORT` | `8000` | HTTP port NyxAI listens on. |
| `NYXAI_DATABASE_URL` | `./data/nyxai.db` locally; `/data/nyxai.db` in Docker | SQLite file path or SQLx SQLite URL. |
| `NYXAI_ASSET_DIR` | `./data/avatars` beside the local database; `/data/avatars` in Docker | Persistent character-avatar directory. |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Fallback Ollama URL until a URL is saved in Settings. |
| `A1111_BASE_URL` | `http://localhost:7860` | Fallback A1111-compatible image API URL until a URL is saved in Settings. |
| `NYXAI_IMAGE_DIR` | `./data/images` beside the local database; `/data/images` in Docker | Persistent generated scene-image directory. |

The saved Ollama and A1111 URLs in **Settings** have priority over their corresponding environment fallbacks. The other variables are read at NyxAI startup. Use a persistent `/data` location for the database, avatars, and generated images; changing database locations does not migrate existing data automatically.

## PWA and mobile use

NyxAI includes a web manifest, midnight/purple/gold icon, standalone display metadata, Apple home-screen metadata, and a small service worker. The service worker caches only the public application shell (HTML, CSS, JavaScript, manifest, and icon). It never caches `/api` responses, Ollama traffic, avatars, chats, settings, or other private data. When the shell is available but the NyxAI server cannot be reached, the app clearly reports that the server is unavailable rather than pretending chat is usable.

On supported browsers, use the browser's **Install app** or **Add to Home Screen** action. Service workers and Chromium-style PWA installation require a secure context: `https://` or browser-recognized localhost. Private Tailscale/LAN HTTP remains supported for normal web use, but may not offer the install prompt until you provide HTTPS through your own private setup. NyxAI does not manage certificates.

The chat composer grows for multiline messages up to a comfortable limit. On a desktop keyboard, **Enter** sends and **Shift+Enter** adds a newline. On touch-first devices, Enter remains available for multiline composition. The layout uses dynamic viewport sizing and safe-area insets so the composer stays visible when mobile browser chrome or the on-screen keyboard changes size.

## Run locally

Requirements: Rust 1.82 or newer, plus an Ollama server and downloaded model when using generation.

```bash
cargo run --release
```

Open `http://localhost:8000`. The server listens on `0.0.0.0`, so a phone on the same private network can use `http://SERVER_IP:8000` when the firewall permits it.

On PowerShell, for a remote Ollama server:

```powershell
$env:OLLAMA_BASE_URL = "http://192.168.1.100:11434"
cargo run --release
```

## Docker Compose

NyxAI deliberately does not bundle Ollama:

```bash
docker compose up --build
```

Copy `.env.example` to `.env` to configure the port, database, avatar/image directories, Ollama URL, and A1111 URL. SQLite, avatars, and generated images persist in the `nyxai_data` named volume. The runtime image is multi-stage, runs as a non-root `nyxai` user, uses a small init process in Compose, and reports healthy when its own `GET /api/health` endpoint returns `{ "status": "ok" }`. Health does not depend on Ollama or A1111: NyxAI is alive even if either provider is temporarily offline.

For networking:

1. **NyxAI and Ollama on the same Linux host:** use `http://127.0.0.1:11434` when NyxAI runs directly on that host.
2. **NyxAI in Docker, Ollama on LAN/Tailscale:** use the remote private address, such as `http://100.x.y.z:11434`.
3. **Separate Docker containers:** connect both to the same user-defined network and use the Ollama service name, such as `http://ollama:11434`.
4. **NyxAI in Docker, Ollama on the Linux host:** container `localhost` is not the host. Explicitly map `host.docker.internal:host-gateway`, use that address, and ensure Ollama is reachable from Docker.

Use the same private-network patterns for A1111, substituting port `7860`. Do not expose Ollama or A1111 directly to the public internet.

Do not expose Ollama directly to the public internet.

For a Docker-managed named volume, the image prepares `/data` for the non-root runtime user. If you replace it with a host bind mount, ensure that the host directory is writable by the container user or use your deployment's normal ownership policy.

## Tailscale and private access

NyxAI has no Tailscale dependency. Run it on a machine already connected to your Tailnet, then browse to `http://TAILSCALE_HOSTNAME:8000` or the machine's Tailscale IP. No public port forwarding is required. Keep Ollama on the host, LAN, or Tailnet and configure its private URL in NyxAI; do not publish Ollama to the internet.

## Troubleshooting

- **NyxAI cannot connect to Ollama:** confirm the URL in Settings, verify Ollama is running, then use **Test connection**. In Docker, `localhost` means the NyxAI container—not the host.
- **No models appear:** NyxAI reached Ollama, but Ollama has no installed models or the selected model was removed. Pull a model with Ollama, then use **Refresh models**.
- **NyxAI cannot connect to A1111:** verify the image server URL and ensure its API mode is enabled (AUTOMATIC1111 normally needs `--api`). In Docker, `localhost` means the NyxAI container; use a reachable LAN/Tailscale address or explicit host gateway mapping.
- **No image checkpoints appear:** NyxAI may still be able to use the backend default checkpoint. Refresh models after the backend has finished loading, or choose no explicit Image Model.
- **Docker cannot reach host Ollama on Linux:** configure a reachable host/LAN/Tailscale address, or explicitly map `host.docker.internal:host-gateway` and use that hostname. Docker does not add that mapping automatically on every Linux installation.
- **A PNG card has no metadata:** only supported PNG `chara` text metadata is accepted. A normal PNG can still be used as a character avatar after you create or import the character.
- **PWA install is unavailable:** use a supported browser in HTTPS or localhost context. Private HTTP can serve NyxAI but browsers may disable service workers and installation there.
- **A Tailscale hostname is unreachable:** verify both devices are connected to the same Tailnet, use the correct NyxAI port, and check the host firewall.
- **A migration fails at startup:** stop NyxAI, back up the SQLite database and `avatars` directory together, review the startup log, then correct the filesystem/permissions issue before restarting. NyxAI migrations run automatically and do not rebuild user databases.

## Test

```bash
cargo test
cargo build --release
node --test tests/roleplay.test.cjs
node --test tests/pwa.test.cjs
```

The renderer test only requires Node 18 or newer; no browser test framework is required.

Manual character check: open **Manage**, create a character, choose an avatar, add tags, save, reopen it, edit a field, duplicate it, then delete each copy through the confirmation dialog. The duplicate should keep its own avatar after the original is deleted.

Import check: choose **Import**, first try a JSON card containing only `name`, then one with `description` but no name, and then one with `null` optional fields, a string tag, and mixed alternate-greeting values. Each should import and show non-blocking warnings in the editor. Reopen an imported character, change an alternate greeting, add and remove another, and save. Create one chat with the default and another with an alternate; each must show only the greeting selected for that chat after restart. Import a supported PNG card to verify its image becomes the avatar; a normal PNG without `chara` metadata, malformed JSON, and unrelated JSON should show a concise error instead of creating a character.

For a sidebar check: create three characters and multiple chats beneath two of them. Open the drawer, expand several character rows independently, and confirm that tapping a character only toggles its nested list. Tap a chat to open it and verify the drawer closes and its Current marker appears. Use **New Chat**, per-chat overflow actions, character actions, and the empty-state Create Character control. Verify long names truncate instead of moving overflow controls off-screen.

For Ollama: configure Settings, test the server, refresh/select a model, send a message in an open character chat, and use Stop during a longer response. The selected model and generation settings are global settings that persist independently of chats.

Persona check: create **Arturo** and **Aldric** in Settings → Manage personas, make Arturo the default, and create two chats for a character whose greeting or prompt contains `{{char}}` and `{{user}}`. Start the first with Arturo and choose Aldric in the second new-chat sheet. Each chat should preserve its own persona after restart and resolve the placeholders differently, while the character editor continues to show the original placeholders. Change one chat's persona from its chat action menu; only later generations should use the new persona. Delete a persona and confirm its chats remain while their headers show **Persona: User**.

Appearance check: open **Settings** → **Appearance**, change a picker and a valid hex field, and verify the preview, sidebar accents, composer, dialogs, and message colors update immediately. Save, restart NyxAI, and verify the colors persist; choose **Reset to NyxAI default**, save, and verify the original palette returns. Send or open messages containing `*She looks away.*`, `**She folds her arms.**`, `"Welcome back."`, and mixed prose. Confirm the visual segments are readable, malformed markers remain visible as normal text, and Copy returns the original raw message.

Production polish check: open `/api/health` and confirm `{ "status": "ok" }`; open Settings and confirm the About card reports the package version. Test `**word**`, `**two words**`, `*word*`, dialogue, and mixed roleplay formatting. On a narrow phone viewport, open the drawer, focus the multiline composer, type several lines, and confirm the composer remains above the keyboard. Disconnect the NyxAI server or network temporarily to confirm the server-unavailable banner and Retry action appear. On a secure origin or localhost, inspect the manifest and install NyxAI from the browser if supported.

AI Character Creator check: configure Ollama and either select a chat model or save a dedicated **Character Creator Model**. From **Manage characters**, open the moon button and describe a character containing `{{user}}`; request several alternate greetings. Confirm the generated draft opens in the existing editor, no character appears in the sidebar until **Save character** is pressed, and the raw placeholders are still present in the editor. Change one generated field manually, regenerate a different field with an optional direction, and confirm the manual change remains. Save the final character, create a chat, and verify the existing greeting/template flow resolves the placeholders normally. Try a model that emits malformed output to confirm the prompt remains available and no character is saved after the bounded repair fails.

Lorebook check: create **Elderglen** in **Settings → Manage lorebooks**, then add an Always Active *World rules* entry and a high-priority *Blackwood Village* entry with `blackwood, blackwood village` keys. Attach it to a character, open two chats for that character, and mention Blackwood in only one. Generate a response in each: the response after the mention should expose both entries through **Context**, while the unrelated chat should expose only *World rules*. Attach a second lorebook to just one chat to confirm attachments and histories stay isolated. Export Elderglen, import the JSON, and verify a duplicate-safe copy retains its entries and configuration.

## Deferred work

Future work may add richer persona metadata, sidebar search, character-card export, broader card-format compatibility, semantic memory/RAG, branching, image/voice features, and additional AI providers. NyxAI deliberately does not yet implement any of those features.
