use std::collections::HashSet;

use crate::marker::{
    CollectedTarget, MEMO_MARKER_NAME, TARGET_LAYER_MARKER_NAME, TARGET_SINGLE_MARKER_NAME,
    TargetType,
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

pub fn collect_marked_targets() -> aviutl2::AnyResult<Vec<CollectedTarget>> {
    crate::EDIT_HANDLE.call_edit_section(|edit| {
        let mut layer_text_targets = HashSet::new();

        for layer in edit.layers() {
            for (_, object) in layer.objects() {
                let caller = edit.object(&object);
                if caller.count_effect(TARGET_LAYER_MARKER_NAME)? > 0 {
                    layer_text_targets.insert(layer.index);
                }
            }
        }

        let fps = *edit.info.fps.numer() as f64 / *edit.info.fps.denom() as f64;
        let mut targets = Vec::new();

        for layer in edit.layers() {
            let layer_name = layer.get_name()?.unwrap_or_else(|| {
                format!(
                    "{}{}",
                    aviutl2::config::get_language_text("Name", "Layer")
                        .unwrap_or("Layer".to_string()),
                    layer.index + 1
                )
            });
            for (position, object) in layer.objects() {
                let caller = edit.object(&object);
                if caller.count_effect(TARGET_LAYER_MARKER_NAME)? > 0 {
                    continue;
                }
                let has_single_text = caller.count_effect(TARGET_SINGLE_MARKER_NAME)? > 0;
                let has_layer_text = layer_text_targets.contains(&layer.index);
                if !has_single_text && !has_layer_text {
                    continue;
                }

                let alias = caller.get_alias_parsed()?;
                let content = extract_content_from_alias(&alias).unwrap_or_default();
                let color = extract_color_from_alias(&alias);
                let memo = extract_memo(&caller);
                let object_id = build_target_id(position.layer, position.start, targets.len());
                let start_time = format_frame_time(position.start, fps);
                let has_non_empty_text = !content.trim().is_empty();

                if (has_single_text || has_layer_text) && has_non_empty_text {
                    targets.push(CollectedTarget {
                        id: object_id.clone(),
                        target_type: TargetType::Text,
                        layer_name: layer_name.clone(),
                        start_time: start_time.clone(),
                        content: content.clone(),
                        color: color.clone(),
                        memo: memo.clone(),
                    });
                }
            }
        }
        Ok(targets)
    })?
}

fn extract_memo(caller: &aviutl2::generic::EditSectionObjectCaller<'_>) -> Option<String> {
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

fn extract_content_from_alias(alias: &aviutl2::alias::Table) -> Option<String> {
    let mut stack = vec![alias];
    while let Some(table) = stack.pop() {
        for (k, v) in table.values() {
            if k == "テキスト" {
                let trimmed = v.trim();
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
        for (_, sub) in table.subtables() {
            stack.push(sub);
        }
    }
    None
}

fn extract_color_from_alias(alias: &aviutl2::alias::Table) -> Option<String> {
    let mut stack = vec![alias];
    while let Some(table) = stack.pop() {
        for (k, v) in table.values() {
            if k == "文字色" {
                let trimmed = v.trim();
                return Some(trimmed.to_string());
            }
        }
        for (_, sub) in table.subtables() {
            stack.push(sub);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        build_target_id, extract_color_from_alias, extract_content_from_alias, format_frame_time,
    };

    #[test]
    fn target_id_is_stable() {
        assert_eq!(build_target_id(2, 120, 3), "l2-f120-n3");
    }

    #[test]
    fn frame_time_is_formatted() {
        assert_eq!(format_frame_time(90, 30.0), "00:00:03.000");
    }

    #[test]
    fn content_extraction_finds_nested_text() {
        let alias: aviutl2::alias::Table = "[Object]\r\nfoo=bar\r\n[Object.0]\r\nテキスト=hello"
            .parse()
            .expect("table parse");
        let content = extract_content_from_alias(&alias);
        assert_eq!(content.as_deref(), Some("hello"));
    }

    #[test]
    fn color_extraction_formats_integer() {
        let alias: aviutl2::alias::Table = "[Object.0]\r\n文字色=16711680"
            .parse()
            .expect("table parse");
        let color = extract_color_from_alias(&alias);
        assert_eq!(color.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn empty_text_is_detected() {
        let alias: aviutl2::alias::Table =
            "[Object.0]\r\nテキスト=   ".parse().expect("table parse");
        let content = extract_content_from_alias(&alias);
        assert_eq!(content, None);
    }
}
