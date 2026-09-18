mod avatars;
mod characters;
mod chats;
mod database;
mod images;
mod lorebooks;
mod memories;
mod personas;
mod settings;

pub use avatars::{
    avatar_url, copy_avatar, ensure_avatar_exists, remove_avatar, store_avatar, store_image,
    AvatarStorageError,
};
pub use characters::{
    create_character, delete_character, duplicate_character, get_character, list_characters,
    update_character,
};
pub use chats::{
    append_assistant_message, append_user_message, create_chat, delete_chat,
    get_chat_for_character, list_chat_messages, list_chats_for_character, rename_chat,
    set_chat_persona,
};
pub use database::Database;
pub use images::{create_chat_image, delete_chat_image, list_chat_images, remove_chat_image_file};
pub use lorebooks::{
    create_lorebook, create_lorebook_entry, delete_lorebook, delete_lorebook_entry,
    duplicate_lorebook, get_lorebook, get_lorebook_entry, list_character_lorebooks,
    list_chat_lorebooks, list_lorebook_entries, list_lorebooks, list_message_lore_entries,
    list_resolvable_lore_entries, record_message_lore_entries, set_character_lorebooks,
    set_chat_lorebooks, update_lorebook, update_lorebook_entry,
};
pub use memories::{
    count_memories, create_memory_with_vector, delete_memory, get_memory, list_memories,
    list_memories_without_current_vector, list_message_memories, mark_messages_processed,
    memory_vectors_for_context, record_message_memories, replace_memory_vector,
    unprocessed_messages, update_memory_with_vector,
};
pub use personas::{
    count_chats_for_persona, create_persona, delete_persona, duplicate_persona, get_persona,
    list_personas, update_persona,
};
pub use settings::{load_settings, save_settings};
