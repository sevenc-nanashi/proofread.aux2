use crate::marker::{CollectedTarget, TargetType};

pub fn build_prompt(
    project_info: &str,
    project_prompt: &str,
    targets: &[CollectedTarget],
) -> String {
    let mut out = String::new();
    out.push_str("あなたは動画の校正者です。以下の情報をもとに、動画のテキストを校正し、修正するべき点を指摘してください。\n\n");
    out.push_str("出力は以下のフォーマットを用いてください：\n");
    out.push_str("```json\n");
    out.push_str("{\n");
    out.push_str("  \"all\": \"全体の指摘・コメント\",\n");
    out.push_str("  \"details\": [\n");
    out.push_str("    {\n");
    out.push_str("      \"id\": \"テキストのID\",\n");
    out.push_str("      \"priority\": \"指摘の優先度（'low', 'medium', 'high'のいずれか）\",\n");
    out.push_str("      \"comment\": \"個別の指摘・コメント\"\n");
    out.push_str("    }\n");
    out.push_str("  ]\n");
    out.push_str("}\n");
    out.push_str("```\n\n");
    out.push_str("# 注意\n");
    out.push_str("- テキストの色は校正の際に重要な情報となります。\n");
    out.push_str("- 修正する必要のないところはノーコメントで構いません。\n");
    out.push_str("# ユーザー指定プロンプト\n");
    out.push_str(project_prompt);
    out.push_str("\n\n# プロジェクトの情報\n");
    out.push_str(project_info);
    out.push_str("\n\n# 校正対象のテキスト\n");

    for target in targets {
        out.push_str("## ");
        out.push_str(&target.id);
        out.push('\n');
        out.push_str("種別：");
        out.push_str(match target.target_type {
            TargetType::Text => "テキスト",
        });
        out.push('\n');
        if let Some(color) = &target.color {
            out.push_str("色：");
            out.push_str(color);
            out.push('\n');
        }
        out.push_str("レイヤー：");
        out.push_str(&target.layer_name);
        out.push('\n');
        out.push_str("開始時間：");
        out.push_str(&target.start_time);
        out.push('\n');
        out.push_str("内容：\n");
        out.push_str(&target.content);
        out.push('\n');
        out.push_str("メモ：\n");
        match &target.memo {
            Some(memo) => out.push_str(memo),
            None => out.push_str("(なし)"),
        }
        out.push_str("\n\n");
    }

    out
}

#[cfg(test)]
mod tests {
    use crate::marker::CollectedTarget;

    use super::build_prompt;

    #[test]
    fn prompt_contains_required_sections() {
        let targets = vec![CollectedTarget::text(
            "t-1",
            "Layer 1",
            "00:00:01",
            "こんにちは",
            Some("#ffffff".into()),
        )];

        let prompt = build_prompt("project", "視聴者は中学生", &targets);
        assert!(prompt.contains("# 注意"));
        assert!(prompt.contains("# ユーザー指定プロンプト"));
        assert!(prompt.contains("視聴者は中学生"));
        assert!(prompt.contains("## t-1"));
        assert!(prompt.contains("色：#ffffff"));
    }
}
