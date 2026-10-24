//! Fixture host only: shared production Settings + native deserialize/validate.
//! No compositor, installation, session configuration or native qualification.
use dock_islands_core::{Arrangement, Placement, Section, Settings};
use serde_json::{Value, json};
use std::{env, fs, path::Path};

fn record(
    dir: &Path,
    rows: &mut Vec<Value>,
    name: &str,
    text: String,
    expected: &str,
    enabled: bool,
    generated: Option<&Settings>,
    unsupported: bool,
) {
    let path = dir.join(format!("{name}.ron"));
    fs::write(&path, text).unwrap();
    // Load the retained bytes, exactly the Settings path in islands::load_settings.
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.len() <= dock_islands_core::MAX_WIRE_BYTES);
    let (outcome, actual_enabled) = match ron::de::from_bytes::<Settings>(&bytes) {
        Err(_) => ("deserialize_error", false),
        Ok(value) => match value.validate() {
            Err(_) => ("validation_error", false),
            Ok(()) => {
                if let Some(original) = generated {
                    assert_eq!(&value, original);
                }
                ("valid", value.enabled)
            }
        },
    };
    assert_eq!(outcome, expected, "{name}");
    assert_eq!(actual_enabled, enabled, "{name}");
    rows.push(
        json!({"file":format!("{name}.ron"), "native_outcome":outcome,
        "enabled":actual_enabled, "serialized_settings":generated.is_some(),
        "python_unsupported":unsupported}),
    );
}

fn main() {
    let output = env::args().nth(1).expect("output directory required");
    let dir = Path::new(&output);
    fs::create_dir_all(dir).unwrap();
    let mut rows = Vec::new();
    let mut enabled = Settings {
        enabled: true,
        ..Settings::default()
    };
    let mut saved = enabled.clone();
    saved.placement = Placement::Separate;
    let mut layout = Arrangement::default();
    layout.together.order = [Section::End, Section::Start, Section::Center];
    layout.together.position = 1234;
    layout.separate.order = [Section::Center, Section::End, Section::Start];
    layout.separate.positions = [10000, 4321, 0];
    saved.outputs.insert("DP-9".into(), layout.clone());
    saved.outputs.insert("eDP-1".into(), layout);
    let mut limits = saved.clone();
    limits.gap = 64;
    limits.outputs.clear();
    limits
        .outputs
        .insert("é".repeat(64), Arrangement::default());
    for n in 0..31 {
        limits.outputs.insert(n.to_string(), Arrangement::default());
    }
    for (name, value) in [
        ("enabled", &enabled),
        ("disabled", &Settings::default()),
        ("outputs-enabled", &saved),
        ("limits", &limits),
    ] {
        record(
            dir,
            &mut rows,
            name,
            ron::ser::to_string(value).unwrap(),
            "valid",
            value.enabled,
            Some(value),
            false,
        );
        record(
            dir,
            &mut rows,
            &format!("{name}-pretty"),
            ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default()).unwrap(),
            "valid",
            value.enabled,
            Some(value),
            false,
        );
    }
    saved.enabled = false;
    record(
        dir,
        &mut rows,
        "outputs-disabled",
        ron::ser::to_string(&saved).unwrap(),
        "valid",
        false,
        Some(&saved),
        false,
    );
    let good = ron::ser::to_string(&enabled).unwrap();
    let changes = [
        (
            "invalid-order-list",
            "order:(Start,Center,End)",
            "order:[Start,Center,End]",
            "deserialize_error",
        ),
        (
            "invalid-position-list",
            "positions:(0,5000,10000)",
            "positions:[0,5000,10000]",
            "deserialize_error",
        ),
        (
            "invalid-order-short",
            "order:(Start,Center,End)",
            "order:(Start,End)",
            "deserialize_error",
        ),
        (
            "invalid-order-long",
            "order:(Start,Center,End)",
            "order:(Start,Center,End,End)",
            "deserialize_error",
        ),
        (
            "invalid-position-short",
            "positions:(0,5000,10000)",
            "positions:(0,5000)",
            "deserialize_error",
        ),
        (
            "invalid-position-long",
            "positions:(0,5000,10000)",
            "positions:(0,5000,10000,1)",
            "deserialize_error",
        ),
        (
            "invalid-order-duplicate",
            "order:(Start,Center,End)",
            "order:(Start,Start,End)",
            "validation_error",
        ),
        (
            "invalid-order-enum",
            "order:(Start,Center,End)",
            "order:(Start,Other,End)",
            "deserialize_error",
        ),
        (
            "invalid-position-range",
            "positions:(0,5000,10000)",
            "positions:(0,5000,10001)",
            "validation_error",
        ),
        (
            "invalid-position-negative",
            "position:5000",
            "position:-1",
            "deserialize_error",
        ),
        (
            "invalid-position-overflow",
            "position:5000",
            "position:65536",
            "deserialize_error",
        ),
    ];
    for (name, from, to, outcome) in changes {
        assert!(good.contains(from));
        record(
            dir,
            &mut rows,
            name,
            good.replace(from, to),
            outcome,
            false,
            None,
            false,
        );
    }
    // The former handwritten positive fixture used lists in all three fields.
    record(
        dir,
        &mut rows,
        "invalid-all-lists",
        good.replace("order:(Start,Center,End)", "order:[Start,Center,End]")
            .replace("positions:(0,5000,10000)", "positions:[0,5000,10000]"),
        "deserialize_error",
        false,
        None,
        false,
    );
    enabled
        .outputs
        .insert("DP-1".into(), Arrangement::default());
    let output_text = ron::ser::to_string(&enabled).unwrap();
    for (name, text) in [
        ("unsupported-named", format!("Settings{good}")),
        ("unsupported-number", good.replace("gap:14", "gap:0x0e")),
        (
            "unsupported-unicode",
            output_text.replace("DP-1", "\\u{44}P-1"),
        ),
    ] {
        record(dir, &mut rows, name, text, "valid", true, None, true);
    }
    fs::write(
        dir.join("outcomes.json"),
        serde_json::to_string_pretty(&json!({
            "schema":1, "native_qualification":false, "fixtures":rows
        }))
        .unwrap()
            + "\n",
    )
    .unwrap();
    println!(
        "Generated and loaded {} retained Settings fixtures; all expected outcomes matched.",
        rows.len()
    );
}
