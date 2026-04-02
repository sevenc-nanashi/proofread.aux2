use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    TargetLayerText,
    TargetSingleText,
    TargetLayerAudio,
    TargetSingleAudio,
    Memo,
}

impl MarkerKind {
    pub fn marker_name(self) -> &'static str {
        match self {
            Self::TargetLayerText => TARGET_LAYER_TEXT_MARKER_NAME,
            Self::TargetSingleText => TARGET_SINGLE_TEXT_MARKER_NAME,
            Self::TargetLayerAudio => TARGET_LAYER_AUDIO_MARKER_NAME,
            Self::TargetSingleAudio => TARGET_SINGLE_AUDIO_MARKER_NAME,
            Self::Memo => MEMO_MARKER_NAME,
        }
    }
}

pub const TARGET_LAYER_TEXT_MARKER_NAME: &str = "校正対象（レイヤー、テキスト）@proofread.aux2";
pub const TARGET_SINGLE_TEXT_MARKER_NAME: &str = "校正対象（単一、テキスト）@proofread.aux2";
pub const TARGET_LAYER_AUDIO_MARKER_NAME: &str = "校正対象（レイヤー、音声）@proofread.aux2";
pub const TARGET_SINGLE_AUDIO_MARKER_NAME: &str = "校正対象（単一、音声）@proofread.aux2";
pub const MEMO_MARKER_NAME: &str = "校正メモ@proofread.aux2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkerSpec {
    as_object: bool,
    video: bool,
    audio: bool,
}

fn marker_spec(kind: MarkerKind) -> MarkerSpec {
    match kind {
        MarkerKind::TargetLayerText => MarkerSpec {
            as_object: true,
            video: true,
            audio: false,
        },
        MarkerKind::TargetSingleText => MarkerSpec {
            as_object: false,
            video: true,
            audio: false,
        },
        MarkerKind::TargetLayerAudio => MarkerSpec {
            as_object: true,
            video: false,
            audio: true,
        },
        MarkerKind::TargetSingleAudio => MarkerSpec {
            as_object: false,
            video: false,
            audio: true,
        },
        MarkerKind::Memo => MarkerSpec {
            as_object: false,
            video: true,
            audio: false,
        },
    }
}

fn build_filter_table(kind: MarkerKind) -> aviutl2::filter::FilterPluginTable {
    let spec = marker_spec(kind);
    let config_items = if kind == MarkerKind::Memo {
        vec![aviutl2::filter::FilterConfigItem::Text(
            aviutl2::filter::FilterConfigText {
                name: "内容".to_string(),
                value: String::new(),
            },
        )]
    } else {
        vec![]
    };
    aviutl2::filter::FilterPluginTable {
        name: kind.marker_name().to_string(),
        label: Some("proofread.aux2".to_string()),
        information: "proofread.aux2 marker plugin".to_string(),
        flags: aviutl2::bitflag! {
            aviutl2::filter::FilterPluginFlags {
                video: spec.video,
                audio: spec.audio,
                as_object: spec.as_object,
            }
        },
        config_items,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Text,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedTarget {
    pub id: String,
    pub target_type: TargetType,
    pub layer_name: String,
    pub start_time: String,
    pub content: String,
    pub color: Option<String>,
    pub memo: Option<String>,
}

impl CollectedTarget {
    pub fn text(
        id: impl Into<String>,
        layer_name: impl Into<String>,
        start_time: impl Into<String>,
        content: impl Into<String>,
        color: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            target_type: TargetType::Text,
            layer_name: layer_name.into(),
            start_time: start_time.into(),
            content: content.into(),
            color,
            memo: None,
        }
    }
}

pub fn attach_memos(targets: &mut [CollectedTarget], memo_by_target_id: &HashMap<String, String>) {
    for target in targets {
        if let Some(memo) = memo_by_target_id.get(&target.id) {
            target.memo = Some(memo.clone());
        }
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct TargetLayerTextMarker;

impl aviutl2::filter::FilterPlugin for TargetLayerTextMarker {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::TargetLayerText)
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct TargetSingleTextMarker;

impl aviutl2::filter::FilterPlugin for TargetSingleTextMarker {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::TargetSingleText)
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct TargetLayerAudioMarker;

impl aviutl2::filter::FilterPlugin for TargetLayerAudioMarker {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::TargetLayerAudio)
    }

    fn proc_audio(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _audio: &mut aviutl2::filter::FilterProcAudio,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct TargetSingleAudioMarker;

impl aviutl2::filter::FilterPlugin for TargetSingleAudioMarker {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::TargetSingleAudio)
    }

    fn proc_audio(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _audio: &mut aviutl2::filter::FilterProcAudio,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct MemoMarker;

impl aviutl2::filter::FilterPlugin for MemoMarker {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::Memo)
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{CollectedTarget, MarkerKind, attach_memos, marker_spec};

    #[test]
    fn attaches_memo_by_id() {
        let mut targets = vec![
            CollectedTarget::text(
                "t-1",
                "layer-a",
                "00:00:10",
                "hello",
                Some("#ffffff".into()),
            ),
            CollectedTarget::text("t-2", "layer-b", "00:00:12", "world", None),
        ];
        let mut memos = HashMap::new();
        memos.insert("t-2".to_string(), "意図的な表記".to_string());

        attach_memos(&mut targets, &memos);

        assert_eq!(targets[0].memo, None);
        assert_eq!(targets[1].memo.as_deref(), Some("意図的な表記"));
    }

    #[test]
    fn marker_as_object_flags_match_spec() {
        assert!(marker_spec(MarkerKind::TargetLayerText).as_object);
        assert!(!marker_spec(MarkerKind::TargetSingleText).as_object);
        assert!(marker_spec(MarkerKind::TargetLayerAudio).as_object);
        assert!(!marker_spec(MarkerKind::TargetSingleAudio).as_object);
        assert!(!marker_spec(MarkerKind::Memo).as_object);
    }
}
