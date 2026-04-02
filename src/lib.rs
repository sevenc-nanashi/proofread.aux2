pub mod client;
pub mod config;
pub mod gui;
pub mod marker;
pub mod prompt;
pub mod result;
pub mod service;

#[aviutl2::plugin(GenericPlugin)]
struct ProofreadPlugin {
    gui: aviutl2_eframe::EframeWindow,
    marker_target_layer_text: aviutl2::generic::SubPlugin<marker::TargetLayerTextMarker>,
    marker_target_single_text: aviutl2::generic::SubPlugin<marker::TargetSingleTextMarker>,
    marker_target_layer_audio: aviutl2::generic::SubPlugin<marker::TargetLayerAudioMarker>,
    marker_target_single_audio: aviutl2::generic::SubPlugin<marker::TargetSingleAudioMarker>,
    marker_memo: aviutl2::generic::SubPlugin<marker::MemoMarker>,
}

impl aviutl2::generic::GenericPlugin for ProofreadPlugin {
    fn new(info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        Ok(Self {
            gui: aviutl2_eframe::EframeWindow::new("proofread.aux2", gui::create_gui)?,
            marker_target_layer_text: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
            marker_target_single_text: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
            marker_target_layer_audio: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
            marker_target_single_audio: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
            marker_memo: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
        })
    }

    fn plugin_info(&self) -> aviutl2::generic::GenericPluginTable {
        aviutl2::generic::GenericPluginTable {
            name: "proofread.aux2".to_string(),
            information: "AI Powered Proofreading Plugin for AviUtl2 / https://github.com/sevenc-nanashi/proofread.aux2".to_string(),
        }
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        if let Ok(handle) = self.gui.handle() {
            let _ = registry.register_window_client("proofread.aux2", &handle);
        }

        registry.register_filter_plugin(&self.marker_target_layer_text);
        registry.register_filter_plugin(&self.marker_target_single_text);
        registry.register_filter_plugin(&self.marker_target_layer_audio);
        registry.register_filter_plugin(&self.marker_target_single_audio);
        registry.register_filter_plugin(&self.marker_memo);
    }
}

aviutl2::register_generic_plugin!(ProofreadPlugin);
