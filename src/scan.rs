use std::collections::{HashMap, HashSet};

use aviutl2::generic::ObjectHandle;

use crate::marker::{
    CollectedTarget, CollectedTargets, MEMO_MARKER_NAME, ObjectLocation, TARGET_LAYER_MARKER_NAME,
    TARGET_SINGLE_MARKER_NAME, TargetType,
};

pub fn collect_project_info() -> aviutl2::AnyResult<String> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        let scene_name = edit
            .get_scene_name()
            .unwrap_or_else(|_| "(シーン名不明)".to_string());
        let fps = *edit.info.fps.numer() as f64 / *edit.info.fps.denom() as f64;
        Ok(format!(
            "シーン名: {scene_name}\n解像度: {}x{}\nFPS: {:.3}\nサンプルレート: {}",
            edit.info.width, edit.info.height, fps, edit.info.sample_rate
        ))
    })?
}

pub fn collect_marked_targets() -> aviutl2::AnyResult<CollectedTargets> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        let mut layer_text_targets = HashSet::new();

        for layer in edit.layers() {
            for (_, object) in layer.objects() {
                let caller = edit.object(object);
                if caller.count_effect(TARGET_LAYER_MARKER_NAME)? > 0 {
                    layer_text_targets.insert(layer.index);
                }
            }
        }

        let fps = *edit.info.fps.numer() as f64 / *edit.info.fps.denom() as f64;
        let mut targets = Vec::new();
        let mut locations = HashMap::new();

        for layer in edit.layers() {
            let layer_name = layer.get_name()?.unwrap_or_else(|| {
                format!(
                    "{}{}",
                    aviutl2::config::get_language_text("Name", "Layer"),
                    layer.index + 1
                )
            });
            for (position, object) in layer.objects() {
                let caller = edit.object(object);
                if caller.count_effect(TARGET_LAYER_MARKER_NAME)? > 0 {
                    continue;
                }
                let has_single_text = caller.count_effect(TARGET_SINGLE_MARKER_NAME)? > 0;
                let has_layer_text = layer_text_targets.contains(&layer.index);
                if !has_single_text && !has_layer_text {
                    continue;
                }

                let content = extract_item_value(&edit, object, "テキスト").unwrap_or_default();
                let color = extract_item_value(&edit, object, "文字色");
                // PSDToolkit用。
                // 正直なところ、こうやってプラグイン毎にハードコードするのは拡張性が悪いのであまり良くないが
                // いい機構が思いつかない...
                let character_id = extract_item_value(&edit, object, "キャラクターID");
                let memo = extract_memo(&edit, object);
                let object_id = build_target_id(position.layer, position.start, targets.len());
                let start_time = format_frame_time(position.start, fps);
                let has_non_empty_text = !content.trim().is_empty();

                if (has_single_text || has_layer_text) && has_non_empty_text {
                    locations.insert(object_id.clone(), ObjectLocation { object, position });
                    targets.push(CollectedTarget {
                        id: object_id.clone(),
                        target_type: TargetType::Text,
                        layer_name: layer_name.clone(),
                        start_time: start_time.clone(),
                        content: content.clone(),
                        color: color.clone(),
                        character_id: character_id.clone(),
                        memo: memo.clone(),
                    });
                }
            }
        }
        Ok(CollectedTargets { targets, locations })
    })?
}

fn extract_memo(read: &aviutl2::generic::ReadSection, object: ObjectHandle) -> Option<String> {
    let caller = read.object(object);
    if caller.count_effect(MEMO_MARKER_NAME).ok()? == 0 {
        return None;
    }
    caller
        .get_effect_item(MEMO_MARKER_NAME, 0, "内容")
        .ok()
        .and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
}

fn extract_item_value(
    read: &aviutl2::generic::ReadSection,
    object: ObjectHandle,
    item_name: &str,
) -> Option<String> {
    for effect in read.get_effects(object).ok()? {
        if let Ok(value) = read.get_effect_item_value(effect, item_name) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(
                    trimmed
                        .replace("\\n", "\n")
                        .replace("\\t", "\t")
                        .replace("\\\\", "\\"),
                );
            }
        }
    }
    None
}

fn build_target_id(layer: usize, start: usize, serial: usize) -> String {
    format!("l{layer}-f{start}-n{serial}")
}

fn format_frame_time(frame: usize, fps: f64) -> String {
    if fps <= 0.0 {
        return "00:00:00.000".to_string();
    }
    let total_ms = ((frame as f64 / fps) * 1000.0).round() as u64;
    let hours = total_ms / 3_600_000;
    let minutes = (total_ms % 3_600_000) / 60_000;
    let seconds = (total_ms % 60_000) / 1_000;
    let millis = total_ms % 1_000;
    format!("{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}
