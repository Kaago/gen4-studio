//! Stories: a project entity of the text project that orders what the game already has into
//! the steps of one thread (Gutachten A §3.2, model (c)). A story is `data/stories/NNNN.json`
//! with the side file `data/stories/NNNN.layout.json` (where its nodes sit in the graph), its
//! id `STORY_…` is in the register `data/ids/stories.json`, and it is the third data kind
//! ([`DATA_KINDS`](crate::textproject::kinds::DATA_KINDS)): export, import and the build leave
//! it alone, the check judges its files ([`schema::check_file`]).
//!
//! A story is no store asset: making and changing one are each one file step on the undo
//! stack ([`entities::create_story`], [`entities::update_story`]), written at once like naming a
//! flag. Its references (`progress`, `uses`, `flags`) are weak: an id the project does not know
//! is a warning (`story.dangling-ref`), never a refusal. The format is documented in
//! `docs/formats/stories.md`.

pub mod entities;
pub mod schema;

pub use entities::{
    NOTHING_TO_CHANGE, StoryEntry, StoryPatch, StoryReport, StoryRequest, create_story,
    create_story_files, duplicate_key_text, list_stories, name_empty_text, update_story,
    update_story_files,
};
pub use schema::{
    DANGLING_REF_CODE, Layout, Made, Position, Progress, STORIES_DIR, Step, StepKind, Story, Use,
    check_file, layout_rel, number_of, read_layout, read_story, story_rel,
};
