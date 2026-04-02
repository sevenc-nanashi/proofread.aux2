pub mod client;
pub mod config;
pub mod gui;
pub mod marker;
pub mod prompt;
pub mod result;
pub mod service;

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ProjectData {
    pub project_prompt: String,
}

const PROJECT_DATA_KEY: &str = "proofread_project_data";

#[aviutl2::plugin(GenericPlugin)]
struct ProofreadPlugin {
    gui: aviutl2_eframe::EframeWindow,
    state: Arc<Mutex<ProjectData>>,
    marker_target_layer_text: aviutl2::generic::SubPlugin<marker::TargetLayerTextMarker>,
    marker_target_single_text: aviutl2::generic::SubPlugin<marker::TargetSingleTextMarker>,
    marker_target_layer_audio: aviutl2::generic::SubPlugin<marker::TargetLayerAudioMarker>,
    marker_target_single_audio: aviutl2::generic::SubPlugin<marker::TargetSingleAudioMarker>,
    marker_memo: aviutl2::generic::SubPlugin<marker::MemoMarker>,
}

impl aviutl2::generic::GenericPlugin for ProofreadPlugin {
    fn new(info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        let state = Arc::new(Mutex::new(ProjectData::default()));
        let ui_state = Arc::clone(&state);
        Ok(Self {
            gui: aviutl2_eframe::EframeWindow::new("proofread.aux2", move |cc, handle| {
                gui::create_gui(cc, handle, Arc::clone(&ui_state))
            })?,
            state,
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

    fn on_project_load(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        let loaded = project.deserialize::<ProjectData>(PROJECT_DATA_KEY);
        let mut state = self.state.lock().expect("project state lock poisoned");
        match loaded {
            Ok(data) => *state = data,
            Err(_) => *state = ProjectData::default(),
        }
        let _ = self.gui.egui_ctx().map(|ctx| ctx.request_repaint());
    }

    fn on_project_save(&mut self, project: &mut aviutl2::generic::ProjectFile) {
        let snapshot = self
            .state
            .lock()
            .expect("project state lock poisoned")
            .clone();
        let _ = project.serialize(PROJECT_DATA_KEY, &snapshot);
    }
}

aviutl2::register_generic_plugin!(ProofreadPlugin);
