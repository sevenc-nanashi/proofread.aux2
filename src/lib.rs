pub mod client;
pub mod config;
pub mod gui;
pub mod marker;
pub mod prompt;
pub mod result;
pub mod scan;
pub mod service;

use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectData {
    #[serde(default = "default_prompt_template_id")]
    pub prompt_template_id: String,
    #[serde(default)]
    pub project_prompt: String,
}

impl Default for ProjectData {
    fn default() -> Self {
        Self {
            prompt_template_id: default_prompt_template_id(),
            project_prompt: String::new(),
        }
    }
}

fn default_prompt_template_id() -> String {
    prompt::BUILTIN_TEMPLATE_ID.to_string()
}

const PROJECT_DATA_KEY: &str = "proofread_project_data";
pub static EDIT_HANDLE: aviutl2::generic::GlobalEditHandle =
    aviutl2::generic::GlobalEditHandle::new();

#[aviutl2::plugin(GenericPlugin)]
struct ProofreadPlugin {
    gui: aviutl2_eframe::EframeWindow,
    state: Arc<Mutex<ProjectData>>,
    marker_target_layer: aviutl2::generic::SubPlugin<marker::TargetLayerMarker>,
    marker_target_single: aviutl2::generic::SubPlugin<marker::TargetSingleMarker>,
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
            marker_target_layer: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
            marker_target_single: aviutl2::generic::SubPlugin::new_filter_plugin(&info)?,
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
        EDIT_HANDLE.init(registry.create_edit_handle());

        registry.register_filter_plugin(&self.marker_target_layer);
        registry.register_filter_plugin(&self.marker_target_single);
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
