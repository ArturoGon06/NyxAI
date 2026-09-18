# NyxAI – Codex Follow-Up Prompt: Milestone 1

Use the NyxAI master specification I provided as the project's long-term source of truth.

Do NOT attempt to implement the entire project at once.

Start with MILESTONE 1: Project Foundation only.

Before writing or modifying code:

1. Inspect the current repository and determine its existing state.
2. If files already exist, understand their purpose before changing them.
3. Briefly explain the architecture you recommend for NyxAI based on the master specification.
4. Propose a balanced project structure that follows these rules:
   - No giant thousand-line files.
   - No excessive file fragmentation.
   - Group related functionality together.
   - Keep UI, domain logic, provider communication, persistence, and application/server logic clearly separated.
   - Do not introduce abstractions that are not needed yet.
5. Explain the technology choices you intend to use for:
   - Rust backend/server
   - Web frontend
   - Styling/UI
   - SQLite persistence
   - Database migrations
   - Docker
   - Docker Compose
   - PWA support later
6. Identify any architectural decisions that should be made now versus decisions that should intentionally wait until later milestones.

After the plan is presented, implement ONLY the Milestone 1 requirements.

---

## MILESTONE 1 REQUIREMENTS

Create the NyxAI project foundation.

Implement:

- Rust project initialization if the repository is empty.
- A clean and balanced project structure.
- A web application/server architecture appropriate for a self-hosted Rust web app.
- The application listening on `0.0.0.0`.
- Default application port `8000`.
- Port configuration through the `NYXAI_PORT` environment variable.
- A basic responsive mobile-first UI shell.
- The initial NyxAI visual design foundation.
- Midnight black background.
- Dark haze purple secondary surfaces.
- Subtle gold accent color.
- A basic application layout designed primarily for portrait phones.
- A placeholder chat screen.
- A placeholder slide-out sidebar/drawer.
- A placeholder NyxAI header.
- A placeholder message composer.
- SQLite database initialization.
- Database migration support.
- Basic application settings persistence.
- Dockerfile.
- `docker-compose.yml`.
- Persistent Docker volume for application data.
- Environment variable configuration.
- Basic README documentation.

Do NOT implement yet:

- Ollama integration.
- Model listing.
- AI generation.
- Streaming responses.
- Character creation.
- Character card importing.
- Chat persistence beyond what is necessary to establish the database architecture.
- Advanced generation settings.
- Image generation.
- Voice features.
- Future AI providers.

The purpose of Milestone 1 is to establish a clean, working foundation before adding actual AI functionality.

---

## UI FOUNDATION

The initial UI should already establish the NyxAI identity.

### Visual direction

Application name: **NyxAI**

Primary aesthetic:

- Midnight black
- Dark haze purple
- Subtle gold accents
- Minimal
- Atmospheric
- Modern
- Premium
- Mobile-first

### Suggested palette

Background:

`#08080B`

Secondary Background:

`#101014`

Dark Purple:

`#21172D`

Purple Accent:

`#4A2C63`

Gold Accent:

`#C9A227`

Bright Gold:

`#E8C55A`

Primary Text:

`#F2EEF5`

Muted Text:

`#9A94A3`

Do not overuse gold.

Gold should be reserved primarily for small accents, active states, and important controls.

Avoid excessive gradients, glow effects, neon styling, and visual clutter.

---

## INITIAL MOBILE UI SHELL

Build a functional visual shell that demonstrates the intended mobile-first layout.

The initial layout should conceptually include:

```text
------------------------------------------------

Menu Button       NyxAI / Character Name       Actions

------------------------------------------------


                Placeholder Chat Area


        Placeholder Character Message


                   Placeholder User Message


------------------------------------------------


      Attach       Message Input       Send


------------------------------------------------
```

The UI does not need real chat functionality yet.

Use placeholder content and components where necessary.

The purpose is to establish:

- Responsive layout
- Mobile sizing
- Header behavior
- Sidebar behavior
- Message area layout
- Message composer layout
- Theme foundation

---

## SIDEBAR FOUNDATION

Implement the visual and interaction foundation for the future character/chat sidebar.

For Milestone 1, placeholder data is acceptable.

The sidebar should behave as a mobile slide-out drawer.

Requirements:

- Open from a menu button.
- Overlay the current page on mobile.
- Use a dark backdrop.
- Close when tapping outside the drawer.
- Close using an obvious close control where appropriate.
- Be touch-friendly.
- Not permanently consume the screen on mobile.

Include placeholder examples demonstrating the intended future hierarchy:

### Expanded Character

```text
v Nyx

    Main Chat
    Adventure

    + New Chat
```

### Collapsed Character

```text
> Luna
```

### Another Expanded Character

```text
v Aria

    Testing
    Story

    + New Chat
```

IMPORTANT:

This is only placeholder UI for Milestone 1.

Do not implement the full character/chat database system yet.

However, structure the UI so the placeholder sidebar can later be connected cleanly to real character and chat data.

---

## SETTINGS FOUNDATION

Implement only the basic settings architecture and persistence needed for the project foundation.

Create a simple settings area or placeholder settings screen that demonstrates where future settings will live.

Do not implement full model/provider settings yet.

At minimum, establish a persistent application settings structure that can later support:

- Appearance settings
- Provider settings
- Model settings
- User preferences

Only implement what is genuinely needed now.

Do not create empty complex abstractions for every future settings category.

---

## DATABASE FOUNDATION

Set up SQLite correctly.

Requirements:

- Database initialization.
- Database migrations.
- Persistent database location configurable through environment variables.
- Reasonable error handling.
- A clean separation between database code and UI code.

For Docker, default persistent data should live under a path such as:

`/data`

Example environment variable:

`NYXAI_DATABASE_URL=/data/nyxai.db`

Do not fully implement all future tables unless they are needed for Milestone 1.

It is acceptable to establish only:

- Application metadata
- Basic settings persistence
- Migration infrastructure

Future character, chat, and message schema can be added in later milestones.

---

## DOCKER REQUIREMENTS

Create a working `Dockerfile` and `docker-compose.yml`.

Requirements:

- Build and run NyxAI in Docker.
- Expose port `8000`.
- Bind application listening address to `0.0.0.0`.
- Persist application data through a Docker volume.
- Support environment variables.
- Do not bundle Ollama into this container.
- Do not require Ollama for Milestone 1 to run.

The Milestone 1 Docker deployment should allow the user to run NyxAI with a command equivalent to:

```bash
docker compose up --build
```

After startup, NyxAI should be accessible at:

`http://SERVER_IP:8000`

and, when applicable:

`http://TAILSCALE_HOSTNAME:8000`

---

## README REQUIREMENTS

Create or update the README with:

- What NyxAI is.
- Current project status.
- Milestone 1 functionality.
- Linux requirements.
- Development setup.
- How to run locally.
- How to run with Docker.
- How to run with Docker Compose.
- Port configuration.
- Environment variables currently supported.
- Data persistence location.
- A brief note that Ollama integration will be implemented in Milestone 2.

Do not document unimplemented features as if they already work.

Clearly distinguish:

- Implemented now
- Planned later

---

## CODE QUALITY RULES

Maintain the architectural standards from the master specification.

Specifically:

- Keep files reasonably sized and focused.
- Do not create giant modules containing unrelated responsibilities.
- Do not create a file for every tiny component.
- Prefer logical grouping of related functionality.
- Use descriptive names.
- Keep server logic separate from UI logic.
- Keep persistence separate from application/domain logic.
- Avoid unnecessary abstractions.
- Avoid unnecessary dependencies.
- Avoid premature optimization.
- Use comments only for non-obvious logic or important architectural decisions.

The code should be easy for a developer to:

- Read.
- Modify.
- Debug.
- Extend in future milestones.

---

## VALIDATION

Before considering Milestone 1 complete:

1. Verify the project builds successfully.
2. Verify the application starts successfully.
3. Verify the application listens on port `8000` by default.
4. Verify `NYXAI_PORT` can override the default port.
5. Verify the application binds to `0.0.0.0`.
6. Verify the basic UI loads.
7. Verify the mobile layout is usable.
8. Verify the sidebar opens and closes.
9. Verify the Docker image builds.
10. Verify Docker Compose starts successfully.
11. Verify persistent application data survives container recreation.
12. Verify SQLite initialization works.
13. Verify migrations run successfully.
14. Fix any errors discovered during validation.

Do not mark Milestone 1 complete unless the implemented functionality actually works.

---

## FINAL OUTPUT

When finished:

1. Summarize what was implemented.
2. List the important files created or modified.
3. Explain how to run NyxAI locally.
4. Explain how to run NyxAI with Docker Compose.
5. List the currently supported environment variables.
6. List anything intentionally deferred to Milestone 2.
7. Identify any important technical decisions that were made.

Do not begin Milestone 2 until explicitly instructed.

The goal of this task is a clean, functional, visually recognizable NyxAI foundation, not a partially implemented version of the entire project.
