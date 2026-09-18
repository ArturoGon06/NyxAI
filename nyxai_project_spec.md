# NyxAI Master Project Specification

## Project Name

**NyxAI**

---

# Project Vision

NyxAI is a self-hosted, mobile-first web frontend for interacting with locally hosted AI models and AI characters.

The primary use case is:

A user has a Linux machine running local AI models through Ollama.

NyxAI provides a clean, modern, phone-focused interface for interacting with those models.

NyxAI manages:

- AI character cards
- Characters
- Multiple chats per character
- Conversation history
- Model selection
- Context configuration
- Generation settings
- Themes
- Chat appearance customization
- Connections to local AI providers

NyxAI does NOT run AI models itself.

The AI inference backend is separate.

The initial primary backend is Ollama.

The architecture must allow future support for additional local AI providers.

NyxAI should feel intentionally designed for phones.

It must NOT feel like:

- A desktop application squeezed onto a phone
- A complicated developer dashboard
- A cluttered AI control panel
- A clone of an existing character chat application's UI
- A generic admin dashboard
- A giant page full of advanced settings

The primary design principle is:

> Simple, powerful, mobile-first local AI chat.

---

# Application Type

NyxAI is a self-hosted web application.

It is NOT:

- A native iOS application
- A native Android application
- A Windows application
- A macOS application

The application runs on a Linux machine or Linux server.

Users access NyxAI through a web browser.

Primary clients:

- iPhone
- Android phones
- Tablets

Desktop browsers are supported as a secondary use case.

The UI must be designed primarily for portrait mobile screens.

---

# Primary Network Architecture

The intended architecture is:

```text
Phone / Browser
        |
        v
Tailscale or Local Network
        |
        v
NyxAI Web Application
Port 8000
        |
        v
AI Provider Adapter
        |
        v
Ollama
Port 11434 by default
```

Conceptually:

```text
📱 Phone
    |
    | Tailscale / LAN
    |
    v
🖥 Linux AI Machine
    |
    ├── NyxAI
    │       |
    │       └── Port 8000
    |
    └── Ollama
            |
            └── Port 11434
```

NyxAI should listen on:

`0.0.0.0:8000`

The application port should be configurable.

Default environment variable:

`NYXAI_PORT=8000`

Users should be able to access NyxAI using:

- Local LAN IP addresses
- Tailscale IP addresses
- Tailscale MagicDNS hostnames

Example:

`http://TAILSCALE_HOSTNAME:8000`

NyxAI is primarily intended to be accessed through Tailscale for remote access.

Do not require public internet exposure.

Do not require port forwarding.

Do not expose Ollama publicly.

The Ollama backend should remain configurable and independent from the public-facing NyxAI web interface.

---

# Primary Platform

The application will primarily run on:

- Linux
- Docker
- Docker Compose

Linux is the primary target platform.

Windows and macOS do not need to be primary development or deployment targets.

---

# Technology Requirements

Use Rust as the primary application language.

Use a modern Rust web architecture suitable for:

- HTTP APIs
- Async networking
- Streaming AI responses
- Web applications
- Responsive UI

A suitable backend framework such as Axum is recommended.

Use a modern frontend approach compatible with Rust and mobile-first responsive design.

The implementation should prioritize:

- Maintainability
- Performance
- Readability
- Long-term extensibility

Use SQLite for persistent local data.

Use asynchronous Rust where appropriate.

Provide Docker support.

Provide Docker Compose support.

The architecture should support:

- Streaming AI responses
- Persistent local data
- LAN access
- Tailscale access
- Configurable AI backend URLs
- Multiple future AI providers

Do not use a database server requiring an additional external service for the MVP.

SQLite is sufficient.

---

# Architectural Philosophy

NyxAI should have a balanced modular architecture.

The code should be:

- Efficient
- Readable
- Easily modifiable
- Modular
- Easy to debug

Avoid:

- Massive thousand-line files containing unrelated functionality
- One giant application module
- Excessively fragmented projects
- One file per tiny function
- Unnecessary abstraction layers
- Premature optimization
- Overly clever architecture

The goal is:

> Logical modules with clear responsibilities.

NOT:

> One giant file.

And NOT:

> Hundreds of tiny files.

Prefer:

> One meaningful module or file per logical responsibility.

Split modules when they become difficult to navigate or contain multiple unrelated responsibilities.

Do not split files merely to artificially reduce line count.

Do not create separate files for trivial components that naturally belong together.

Do not create factories, repositories, services, traits, or abstraction layers unless they provide a genuine architectural benefit.

Before creating a major abstraction, ask:

> Does this solve a real problem now?

If not, keep the design simpler.

---

# High Level Application Architecture

Use clear separation between:

```text
UI
    |
Application State
    |
Domain Logic
    |
Services
    |
Provider Layer
    |
Persistence
```

Conceptually:

```text
Application
|
+-- UI
|
+-- Application State
|
+-- Domain
|   |
|   +-- Characters
|   +-- Chats
|   +-- Messages
|   +-- Settings
|
+-- Services
|   |
|   +-- Chat Service
|   +-- Character Service
|   +-- Provider Service
|
+-- Providers
|   |
|   +-- Generic Provider Interface
|   +-- Ollama Provider
|   +-- Future Providers
|
+-- Storage
|   |
|   +-- SQLite
|   +-- Migrations
|
+-- HTTP / API Layer
|
+-- Theme System
```

This is a conceptual architecture.

Adapt it appropriately to the selected framework.

Do not follow the example mechanically if a better architecture naturally fits the implementation.

---

# AI Provider Architecture

NyxAI must not be tightly coupled to Ollama.

Ollama is the first provider implementation.

Create a provider abstraction layer.

The UI and chat system should communicate with a generic provider interface or capability system.

Provider-specific implementation details must remain isolated.

Future providers may include:

- OpenAI-compatible APIs
- llama.cpp server
- KoboldCPP
- LM Studio
- Other compatible local AI APIs

The provider architecture should support capabilities such as:

- Test connection
- List models
- Refresh models
- Select models
- Stream generation
- Stop generation where supported
- Context configuration where supported
- Generation parameters where supported

Do not force every provider to pretend it supports identical features.

Expose provider capabilities.

The UI should gracefully adapt based on what the connected provider supports.

---

# Ollama Support

The first implemented AI provider must be Ollama.

Required functionality:

- Configure Ollama URL
- Test connection
- List available models
- Refresh model list
- Select active model
- Send chat generation requests
- Stream generated responses
- Stop generation if supported
- Handle provider errors cleanly
- Support common generation settings
- Support configurable context where supported

The Ollama URL must be configurable.

Example environment variable:

`OLLAMA_BASE_URL`

Examples:

```text
http://192.168.1.100:11434
http://some-hostname:11434
http://host.docker.internal:11434
```

Do NOT hardcode `localhost:11434` as the only possible backend.

NyxAI may run:

- Directly on the Linux host
- Inside Docker
- On a different machine than Ollama
- On the same LAN as Ollama
- With Ollama running in another Docker container

Therefore the backend URL must always be configurable.

Prefer server-side communication between NyxAI and the AI provider.

Do not unnecessarily expose direct provider communication to the browser UI.

---

# Docker Requirements

NyxAI must support Docker deployment.

Provide:

- Dockerfile
- docker-compose.yml
- Persistent data volume
- Environment variable configuration
- README documentation

The default exposed application port is:

`8000`

Container behavior:

NyxAI listens internally on:

`0.0.0.0:8000`

The Docker container should expose:

`8000:8000`

The container must NOT require Ollama to run inside the same container.

Ollama should be treated as an external configurable backend.

Example environment variables:

```text
NYXAI_PORT=8000
NYXAI_DATABASE_URL=/data/nyxai.db
OLLAMA_BASE_URL=http://configured-ollama-address:11434
```

Provide persistent storage for:

- SQLite database
- User assets where appropriate
- Character avatars
- Imported files where appropriate

Document Linux Docker networking clearly.

Do not assume `host.docker.internal` automatically works on every Linux Docker installation.

Document appropriate options for connecting a Dockerized NyxAI instance to Ollama running:

- On the Linux host
- On another LAN machine
- In another Docker container

---

# PWA Support

NyxAI should support Progressive Web App functionality where practical.

Include:

- Web app manifest
- Application icons
- Mobile theme color
- Add to Home Screen compatibility
- Responsive full-screen layout
- App-like mobile experience

The application should work well when added to a phone's home screen.

Do not require:

- App Store distribution
- Native iOS development
- Native Android development

---

# Character System

NyxAI must support AI characters.

Users must be able to:

- Create characters
- Edit characters
- Duplicate characters
- Delete characters
- Import characters
- Export characters

Characters should have a normalized internal data model.

At minimum support:

- ID
- Name
- Avatar
- Description
- Personality
- Scenario
- First Message
- Example Dialogue
- System Prompt
- Tags

Optional support:

- Alternate Greetings
- Creator Notes
- Metadata
- Additional character card fields where useful

Character data should be stored locally.

---

# Character Cards

NyxAI must support importing common character card formats where practical.

Support:

- JSON character cards
- PNG character cards containing embedded character metadata
- Character Card V2-compatible fields where possible

External character card formats must be parsed and normalized.

Use a flow conceptually like:

```text
Character Card File
        |
        v
Importer
        |
        v
Format Parser
        |
        v
Normalized NyxAI Character Model
        |
        v
Database
```

Do not allow external card formats to spread throughout the entire application architecture.

NyxAI should internally operate on its own normalized character model.

Provide clear user-facing errors for invalid character cards.

Do not expose confusing raw parser errors to normal users.

---

# Character Editor

The character editor must be mobile-first.

Users should be able to comfortably create and edit characters from a phone.

Do not display every possible field simultaneously.

Use logical sections.

Use collapsible advanced sections where appropriate.

Example concept:

```text
Character

[ Avatar ]

Name
[________________]

Description
[________________]

Personality
[________________]

Scenario
[________________]

First Message
[________________]

Advanced
v
```

The editor should be:

- Simple
- Touch-friendly
- Easy to understand
- Pleasant to use on a phone

---

# Chat Domain Model

Chats belong to characters.

The relationship is:

```text
Character
    |
    +-- Chat
    +-- Chat
    +-- Chat
```

A character may have multiple independent chats.

A chat should contain:

- ID
- Character ID
- Title
- Creation timestamp
- Last updated timestamp

Messages should contain:

- ID
- Chat ID
- Role
- Content
- Timestamp
- Optional generation metadata

At minimum support roles:

- System
- User
- Assistant

Do not store unrelated chat state inside one giant serialized application object.

Use a reasonable relational SQLite schema.

Include database migrations.

---

# Hierarchical Character and Chat Sidebar

This is a core NyxAI feature.

The application must use a hierarchical character and chat navigation system.

Do NOT display every chat as an independent top-level item.

Instead:

Characters are top-level navigation groups.

Chats belonging to that character appear underneath the character when expanded.

Conceptually:

```text
v Nyx
    Main Chat
    Adventure
    Testing

    + New Chat

> Luna

> Aria

v Cyber Assistant
    Development
    Random Conversation

    + New Chat
```

## Important Interaction Requirement

Tapping a character must expand or collapse that character's chat list.

Tapping a character must NOT automatically open a chat.

The character is a collapsible parent.

The chats are selectable child items.

Only tapping an individual chat should open that conversation.

Each character row should support:

- Expand/collapse control
- Character avatar
- Character name
- Optional action menu

Example:

```text
v [Avatar] Nyx                         ...

        Main Chat
        Late Night Conversation
        Testing

        + New Chat
```

Character actions may include:

- Edit Character
- Duplicate Character
- New Chat
- Export Character
- Delete Character

Chat actions may include:

- Rename Chat
- Duplicate Chat
- Delete Chat

---

# Sidebar Mobile Behavior

The sidebar should be designed primarily for phones.

On portrait mobile screens:

- The sidebar should behave as a slide-out drawer or overlay.
- The sidebar should not permanently consume most of the screen.
- Opening the sidebar should overlay the active chat.
- A dark backdrop should appear behind the sidebar.
- Closing the sidebar should immediately return the user to the active chat.

The sidebar should include:

- NyxAI branding area
- Character list
- Collapsible character groups
- Nested chats
- New character action
- Settings access

The sidebar should be touch-friendly.

The sidebar should avoid:

- Tiny controls
- Deep nested menus
- Desktop-style navigation complexity

The MVP should prioritize a simple mobile drawer.

Do not build an unnecessarily complicated desktop navigation system first.

Desktop layouts may adapt gracefully later.

---

# Chat Experience

The chat screen should feel immersive, clean, and focused.

Required functionality:

- Send messages
- Stream AI responses
- Stop generation
- Regenerate messages
- Continue generation
- Edit messages
- Delete messages
- Copy messages
- Persistent conversation history

Message interactions must work well on touchscreens.

Use:

- Long press menus
- Touch-friendly action sheets
- Contextual actions

Do not clutter every message with permanently visible buttons.

---

# Chat Screen Structure

Conceptually:

```text
------------------------------------------------

Menu        Character Name        Actions

------------------------------------------------


              Messages


      Character Message


                   User Message


      Character Message


------------------------------------------------


    Attach      Message Input       Send


------------------------------------------------
```

The exact implementation may vary.

The important design principles are:

- Comfortable on phones
- Large touch targets
- Immersive reading space
- Easy access to the sidebar
- Easy message composition
- Clean streaming behavior

---

# Streaming Behavior

AI responses must stream smoothly.

Streaming should:

- Update the active assistant message incrementally
- Avoid excessive full-page redraws
- Avoid scroll jitter
- Avoid blocking the interface
- Preserve a good mobile experience

The application should handle:

- Connection failures
- Generation failures
- Interrupted streams
- Provider errors

Show clean user-facing errors.

Do not show raw stack traces to normal users.

Log useful technical errors for debugging.

---

# New Chat Behavior

Users must be able to create multiple chats for the same character.

Each chat belongs to exactly one associated character.

Creating a chat should be fast.

Do not force unnecessary dialogs before a user can start chatting.

A new chat may initially receive an automatic title such as:

- Untitled Chat
- New Conversation

Users should be able to rename chats later.

Each expanded character group should include an easy:

`+ New Chat`

action.

The new chat created there must automatically belong to that character.

---

# Context Management

NyxAI should support configurable context settings.

Users should be able to:

- Configure context length
- View the configured context size
- Configure maximum response length

Do not assume every AI provider supports identical context configuration.

The provider capability system should expose what is supported.

The UI should adapt gracefully.

---

# Model Settings

The default visible model settings should remain simple.

Default controls:

- Model
- Context Length
- Creativity / Temperature
- Response Length

Advanced settings should be hidden inside a collapsible section.

Potential advanced settings:

- Top P
- Top K
- Min P
- Repetition Penalty
- Seed
- Provider-specific generation parameters

Do not overwhelm the default UI with a wall of sampler parameters.

The goal is:

> Simple by default. Advanced when needed.

---

# Settings Organization

Settings should be grouped logically.

Suggested categories:

## Connection

- AI Provider
- Server URL
- Test Connection

## Model

- Selected Model
- Context
- Basic Generation Settings
- Advanced Generation Settings

## Appearance

- Theme
- Chat Colors
- UI Preferences

## Data

- Import
- Export
- Database Management

Do not place every application setting on one enormous page.

Use mobile-friendly navigation and collapsible sections where appropriate.

---

# NyxAI Visual Identity

Application name:

**NyxAI**

Visual direction:

- Midnight black
- Dark haze purple
- Subtle gold accents

The aesthetic should feel:

- Dark
- Atmospheric
- Modern
- Premium
- Clean
- Minimal

Avoid:

- Excessive gradients
- Neon overload
- Excessive glow effects
- Visual clutter
- Giant decorative panels

Suggested default palette:

| Purpose | Color |
|---|---|
| Background | `#08080B` |
| Secondary Background | `#101014` |
| Dark Purple | `#21172D` |
| Purple Accent | `#4A2C63` |
| Gold Accent | `#C9A227` |
| Bright Gold | `#E8C55A` |
| Primary Text | `#F2EEF5` |
| Muted Text | `#9A94A3` |

Gold must be used sparingly.

Gold should primarily be used for:

- Active indicators
- Selected states
- Important controls
- Small accents
- Icons
- Focus indicators

Do not make large portions of the interface gold.

The overall atmosphere should primarily be midnight black and dark haze purple.

Gold should act as a subtle visual accent.

---

# Chat Appearance Customization

Users must be able to customize chat appearance.

Support customization of:

- Application background color
- User message bubble color
- User message text color
- Character message bubble color
- Character message text color
- Accent color

Users should be able to customize colors through:

- Hexadecimal color input
- Color picker

Example:

```text
User Text
#F2EEF5

Character Text
#EDE8F0

Accent
#C9A227
```

Required functionality:

- Live preview
- Reset to default
- Persistent settings
- Save custom theme configuration

The default NyxAI theme should always be available as a reset option.

Future per-character themes may be supported later.

Do not overbuild future features before the MVP is stable.

---

# Responsive Design

NyxAI is mobile-first.

The primary target is:

**Portrait phones.**

The interface should prioritize:

- Large touch targets
- Responsive layouts
- Comfortable text sizing
- Bottom-oriented interaction where appropriate
- Slide-out drawers
- Bottom sheets
- Touch-friendly menus
- Collapsible advanced settings

Avoid:

- Tiny desktop controls
- Hover-only interactions
- Complex multi-column layouts on phones
- Permanent large sidebars on small screens
- Dense settings pages
- Desktop UI compressed into narrow screens

Desktop layouts may expand gracefully.

Desktop layouts must not define the primary UX.

---

# Persistence

Use SQLite for persistent local application data.

Persist:

- Characters
- Character metadata
- Chats
- Messages
- Application settings
- Provider configurations
- Theme settings
- Chat appearance settings

Database access must remain separate from UI logic.

Use a reasonable relational schema.

Include migrations.

Do not store the entire application state as one giant JSON blob.

---

# Data Safety and Privacy

NyxAI is a local and self-hosted application.

Do not:

- Send user conversations to external services
- Require cloud services
- Require user accounts
- Require telemetry
- Require third-party analytics

NyxAI should function completely independently from cloud services.

The intended remote access mechanism is Tailscale.

The MVP does not need a complex public-facing authentication system.

Do not expose Ollama publicly.

Network behavior should remain explicit and configurable.

---

# Image Generation

Image generation is NOT part of the initial MVP.

Do not implement image generation until the core application is stable.

However, do not design the architecture in a way that makes future image generation difficult.

Potential future providers:

- ComfyUI
- AUTOMATIC1111-compatible APIs
- Forge-compatible APIs

Potential future features:

- Generate character avatars
- Generate scene images
- Attach images to chats

These are future expansion possibilities.

Do not prioritize them over the core chat experience.

---

# Future Features

The architecture should reasonably allow future support for:

- Additional AI providers
- Image generation
- Text-to-speech
- Speech-to-text
- Lorebooks
- World information
- Character memory
- Chat branching
- Per-character themes
- Chat search
- Character search
- Improved import/export functionality

Do not overengineer the MVP around hypothetical future features.

Use extension points only where they provide genuine value.

---

# Project Structure

Create a balanced and understandable project structure.

A conceptual example:

```text
src/

    main.rs

    app/
        state.rs
        routes.rs

    domain/
        character.rs
        chat.rs
        message.rs
        settings.rs

    providers/
        mod.rs
        provider.rs
        ollama.rs

    services/
        chat_service.rs
        character_service.rs

    storage/
        database.rs
        migrations.rs

    ui/
        chat.rs
        sidebar.rs
        character_editor.rs
        settings.rs

    theme/
        mod.rs
```

This is only an example.

Adapt the structure based on the selected framework.

Do not follow this example mechanically.

The actual structure should remain:

- Easy to understand
- Logically organized
- Balanced
- Not excessively fragmented

---

# Code Quality Requirements

The codebase must prioritize:

- Readability
- Maintainability
- Modularity
- Efficiency
- Clear separation of responsibilities
- Easy modification
- Easy debugging

Use descriptive names.

Avoid cryptic variable names.

Avoid unnecessary global state.

Keep responsibilities clearly separated.

Keep:

> UI logic

separate from:

> AI provider communication

and:

> Persistence

and:

> Domain logic

Use comments for:

- Non-obvious decisions
- Complex logic
- Important architectural reasoning

Do not add comments that merely repeat what obvious code already says.

Avoid:

- Massive files containing unrelated functionality
- Extremely fragmented file structures
- Unnecessary abstractions
- Premature optimization
- Excessive dependencies

---

# Performance Requirements

Optimize for responsiveness.

Priorities include:

- Fast UI interactions
- Smooth message streaming
- Efficient message rendering
- Efficient database access
- Responsive mobile behavior
- Avoid blocking the UI
- Avoid unnecessary full-state redraws
- Avoid unnecessary cloning of large data structures

Use async networking appropriately.

Do not prematurely optimize obscure code paths.

Focus performance attention on realistic bottlenecks:

- Streaming responses
- Large chat histories
- Database operations
- Message rendering

---

# Error Handling

Provide clean user-facing errors.

Examples:

- Cannot connect to AI backend.
- No models available.
- Generation failed.
- Connection lost.
- Invalid character card.
- Database error.

Do not expose raw Rust stack traces to normal users.

Log useful technical information for debugging.

---

# Accessibility

Provide:

- Sufficient text contrast
- Reasonable font sizes
- Touch-friendly controls
- Accessible labels
- Keyboard support where practical
- Responsive layouts

Do not rely exclusively on color to communicate important state.

---

# Testing

Add tests for important non-UI functionality.

Prioritize:

- Character card parsing
- Character normalization
- Chat creation
- Message handling
- Provider behavior
- Database operations
- Context construction

Do not spend the majority of initial development building an enormous test suite before the MVP works.

Focus tests on important logic and likely failure points.

---

# Documentation

Create a README containing:

- Project description
- Architecture overview
- Technology choices
- Linux requirements
- Development setup
- Running locally
- Running with Docker
- Running with Docker Compose
- Environment variables
- Connecting to Ollama
- Configuring Ollama URL
- Port 8000 usage
- LAN access
- Tailscale access
- Persistent data locations
- Docker networking notes for Linux

Document:

How NyxAI connects to AI providers.

How to configure:

`OLLAMA_BASE_URL`

How to configure:

`NYXAI_PORT`

How data persistence works.

How Docker networking should be configured when Ollama runs:

- On the Linux host
- On another LAN machine
- In another Docker container

---

# Implementation Milestones

Implement the project incrementally.

Do not attempt all future features immediately.

## Milestone 1: Project Foundation

Implement:

- Rust project initialization
- Clean project structure
- Web server
- Port configuration
- Application listening on `0.0.0.0:8000`
- Basic responsive mobile UI shell
- NyxAI visual foundation
- Dockerfile
- Docker Compose
- SQLite initialization
- Database migrations
- Basic settings persistence
- Basic README

## Milestone 2: Ollama Integration

Implement:

- Configurable Ollama URL
- Connection testing
- Model listing
- Refresh model list
- Model selection
- Basic generation
- Streaming responses
- Provider error handling

## Milestone 3: Character System

Implement:

- Character domain model
- Character creation
- Character editing
- Character deletion
- Character duplication
- Character persistence
- Avatar support

## Milestone 4: Chat System

Implement:

- Chat creation
- Multiple chats per character
- Message persistence
- Chat loading
- Sending messages
- Streaming AI responses
- Basic message actions

## Milestone 5: Hierarchical Sidebar

Implement:

- Character groups
- Expand/collapse behavior
- Nested chats
- New chat action
- Character action menu
- Chat action menu
- Mobile slide-out drawer
- Active chat indication

Important:

Character tap expands or collapses chats.

Character tap does NOT automatically open a chat.

Chat tap opens that specific conversation.

## Milestone 6: Character Card Import

Implement:

- JSON card importing
- PNG embedded metadata importing where practical
- Character normalization
- Import validation
- User-friendly import errors

## Milestone 7: Customization

Implement:

- NyxAI default theme
- Midnight black styling
- Dark haze purple styling
- Gold accent styling
- User message colors
- Character message colors
- Hex color input
- Color picker
- Live preview
- Theme persistence

## Milestone 8: Polish

Implement:

- Loading states
- Empty states
- Error states
- Mobile UX improvements
- Performance review
- Code cleanup
- Accessibility review
- Documentation improvements
- PWA support

---

# Important Development Rules

Do not rewrite working architecture without a strong reason.

Do not introduce unnecessary dependencies.

Do not create fake placeholder implementations and claim features are complete.

Do not leave major functionality as TODO placeholders when a feature is described as implemented.

When making architectural decisions prefer:

- Simple
- Clear
- Maintainable

over:

- Clever
- Overengineered
- Excessively abstract

Before adding a major module, dependency, abstraction, or architectural pattern:

Determine whether it solves a real current problem.

If not:

> Keep the design simpler.

---

# Definition of the MVP

The first usable version of NyxAI must allow a user to:

1. Run NyxAI on Linux.
2. Run NyxAI using Docker.
3. Run NyxAI using Docker Compose.
4. Access NyxAI from a phone browser.
5. Access NyxAI through a local network.
6. Access NyxAI through Tailscale.
7. Open NyxAI on port 8000.
8. Configure an Ollama backend URL.
9. Test the Ollama connection.
10. View available Ollama models.
11. Refresh available models.
12. Select an AI model.
13. Create characters.
14. Edit characters.
15. Delete characters.
16. Import supported character cards.
17. Create multiple chats for a single character.
18. Navigate characters and chats through a collapsible hierarchical sidebar.
19. Expand a character to reveal its chats.
20. Tap an individual chat to open it.
21. Create a new chat directly beneath a selected character.
22. Send messages.
23. Receive streaming AI responses.
24. Persist chats and messages.
25. Configure context settings where supported.
26. Configure basic generation settings.
27. Customize user and character chat colors.
28. Use the NyxAI midnight black, dark haze purple, and subtle gold visual design.

---

# Final Design Principle

NyxAI should feel intentionally designed.

It should feel like a dedicated mobile interface for interacting with local AI characters.

It should not feel like:

- A developer dashboard.
- A desktop application compressed into a phone.
- A settings menu pretending to be an application.

The core user flow should be simple:

```text
Open NyxAI
        |
Open Sidebar
        |
Expand Character
        |
Choose Existing Chat
or
Create New Chat
        |
Talk to Local AI
```

Make this flow:

- Fast
- Obvious
- Clean
- Pleasant
- Touch-friendly

Advanced functionality should be available without overwhelming the user.

The priority is the quality of the core experience.

Build the MVP first.

Keep the architecture clean.

Keep the code readable.

Keep the project modular without excessive fragmentation.

NyxAI should become a polished, self-hosted, mobile-first interface for talking to local AI characters.
