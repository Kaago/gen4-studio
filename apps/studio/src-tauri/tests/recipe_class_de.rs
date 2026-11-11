//! `needs_store` in the German locale: whether a `recipe_apply` waits for the session's store is
//! decided by the recipe (`recipes::creates_level_script`), not by the words of its reason. The
//! locale is process-wide, so this file holds the one test that switches it and runs alone.

use g4_kernel::i18n::set_locale;
use g4_kernel::test_support::synthetic_project;
use serde_json::{Value, json};
use studio_lib::mcp::access::{Class, classify, needs_store};
use studio_lib::policy::UserPolicy;

/// Sets the locale back to `en` when dropped (also when the test fails).
struct EnglishAfter;

impl Drop for EnglishAfter {
    fn drop(&mut self) {
        set_locale("en");
    }
}

fn call(recipe: &str, input: Value) -> Value {
    json!({ "recipe": recipe, "input": input })
}

#[test]
fn a_visibility_and_a_scene_still_need_the_store_in_german() {
    let _english = EnglishAfter;
    let project = synthetic_project();
    let policy = UserPolicy::new(project.dir.path(), &[], None).unwrap();
    let story = "STORY_TESTFADEN";
    let figure = json!({ "header": 0, "local_id": 0 });
    let appears = call(
        "figure_appears",
        json!({ "story": story, "figure": figure, "step": 0, "appears": true, "flag": 1 }),
    );
    let scene = call(
        "scene_on_enter",
        json!({ "story": story, "header": 0, "step": 0, "text": "Willkommen" }),
    );
    let talk = call(
        "talk_advances",
        json!({ "story": story, "figure": figure, "from_step": 0, "text": "Hallo" }),
    );
    let new_story = call("new_story", json!({ "name": "Testfaden", "var": 1 }));
    let empty = json!({ "recipe": "empty_region" });
    let class_of = |args: &Value| classify("recipe_apply", args, &policy);

    // The reason of a class is in the language of the moment: the two differ.
    set_locale("en");
    let english = class_of(&appears);
    set_locale("de");
    let german = class_of(&appears);
    assert!(matches!(english, Class::Irreversible(_)), "{english:?}");
    assert!(matches!(german, Class::Irreversible(_)), "{german:?}");
    assert_ne!(english, german, "the German reason is no English one");

    // Classified and asked in German.
    for args in [&appears, &scene] {
        let class = class_of(args);
        assert!(matches!(class, Class::Irreversible(_)), "{args}");
        assert!(needs_store("recipe_apply", args, &class), "{args}");
    }
    for args in [&talk, &new_story] {
        let class = class_of(args);
        assert_eq!(class, Class::Write, "{args}");
        assert!(!needs_store("recipe_apply", args, &class), "{args}");
    }
    // Irreversible, but no level script file is created.
    let class = class_of(&empty);
    assert!(matches!(class, Class::Irreversible(_)));
    assert!(!needs_store("recipe_apply", &empty, &class));

    // A class made in English and asked after the switch to German (and the other way round)
    // keeps its answer: the words of the reason decide nothing.
    assert!(needs_store("recipe_apply", &appears, &english));
    set_locale("en");
    assert!(needs_store("recipe_apply", &scene, &german));
    // Another command never needs it.
    assert!(!needs_store("recipe_plan", &appears, &german));
}
