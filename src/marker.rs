use std::collections::HashMap;

use aviutl2::generic::{ObjectHandle, ObjectLayerFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkerKind {
    TargetLayer,
    TargetSingle,
    Memo,
}

impl MarkerKind {
    pub fn marker_name(self) -> &'static str {
        match self {
            Self::TargetLayer => TARGET_LAYER_MARKER_NAME,
            Self::TargetSingle => TARGET_SINGLE_MARKER_NAME,
            Self::Memo => MEMO_MARKER_NAME,
        }
    }
}

pub const TARGET_LAYER_MARKER_NAME: &str = "校正対象（レイヤー）@proofread.aux2";
pub const TARGET_SINGLE_MARKER_NAME: &str = "校正対象（単一）@proofread.aux2";
pub const MEMO_MARKER_NAME: &str = "校正メモ@proofread.aux2";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MarkerSpec {
    input: bool,
    video: bool,
}

fn marker_spec(kind: MarkerKind) -> MarkerSpec {
    match kind {
        MarkerKind::TargetLayer => MarkerSpec {
            input: true,
            video: true,
        },
        MarkerKind::TargetSingle => MarkerSpec {
            input: false,
            video: true,
        },
        MarkerKind::Memo => MarkerSpec {
            input: false,
            video: true,
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
                input: spec.input,
            }
        },
        config_items,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectedTarget {
    pub id: String,
    pub target_type: TargetType,
    pub layer_name: String,
    pub start_time: String,
    pub content: String,
    pub color: Option<String>,
    pub character_id: Option<String>,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectLocation {
    pub object: ObjectHandle,
    pub position: ObjectLayerFrame,
}

#[derive(Debug, Clone)]
pub struct CollectedTargets {
    pub targets: Vec<CollectedTarget>,
    pub locations: HashMap<String, ObjectLocation>,
}

impl CollectedTarget {
    pub fn text(
        id: impl Into<String>,
        layer_name: impl Into<String>,
        start_time: impl Into<String>,
        content: impl Into<String>,
        color: Option<String>,
        character_id: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            target_type: TargetType::Text,
            layer_name: layer_name.into(),
            start_time: start_time.into(),
            content: content.into(),
            color,
            character_id,
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
pub struct TargetLayerMarker;

impl aviutl2::filter::FilterPlugin for TargetLayerMarker {
    type Userdata = ();

    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::TargetLayer)
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo<Self::Userdata>,
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct TargetSingleMarker;

impl aviutl2::filter::FilterPlugin for TargetSingleMarker {
    type Userdata = ();

    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::TargetSingle)
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo<Self::Userdata>
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}

#[aviutl2::plugin(FilterPlugin)]
pub struct MemoMarker;

impl aviutl2::filter::FilterPlugin for MemoMarker {
    type Userdata = ();

    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self)
    }

    fn plugin_info(&self) -> aviutl2::filter::FilterPluginTable {
        build_filter_table(MarkerKind::Memo)
    }

    fn proc_video(
        &self,
        _config: &[aviutl2::filter::FilterConfigItem],
        _video: &mut aviutl2::filter::FilterProcVideo<Self::Userdata>
    ) -> aviutl2::AnyResult<()> {
        Ok(())
    }
}
