use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use std::sync::{Arc, Mutex};

use crate::ProjectData;
use crate::config::{Credentials, Preset, credentials_path, load_credentials, save_credentials};
use crate::result::ProofreadResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    SetupRun,
    Settings,
    Result,
}

pub(crate) struct ProofreadGuiApp {
    _handle: AviUtl2EframeHandle,
    state: Arc<Mutex<ProjectData>>,
    screen: Screen,
    result: Option<ProofreadResult>,
    credentials_path: Option<std::path::PathBuf>,
    credentials: Credentials,
    status_message: Option<String>,
}

impl ProofreadGuiApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        handle: AviUtl2EframeHandle,
        state: Arc<Mutex<ProjectData>>,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "M+ 1p".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "./fonts/mplus-1p-regular.ttf"
            ))),
        );
        fonts
            .families
            .get_mut(&egui::FontFamily::Proportional)
            .expect("Failed to get Proportional font family")
            .insert(0, "M+ 1p".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = aviutl2_eframe::aviutl2_visuals();
        });

        let (screen, credentials_path, credentials, status_message) = match credentials_path() {
            Ok(path) => match load_credentials(&path) {
                Ok(credentials) => (Screen::SetupRun, Some(path), credentials, None),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
                    Screen::Settings,
                    Some(path),
                    Credentials::default(),
                    Some("初回設定を入力して保存してください。".to_string()),
                ),
                Err(err) => (
                    Screen::Settings,
                    Some(path),
                    Credentials::default(),
                    Some(format!("設定の読込に失敗しました: {err}")),
                ),
            },
            Err(err) => (
                Screen::Settings,
                None,
                Credentials::default(),
                Some(format!("保存先の解決に失敗しました: {err}")),
            ),
        };

        Self {
            _handle: handle,
            state,
            screen,
            result: None,
            credentials_path,
            credentials,
            status_message,
        }
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("proofread.aux2");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("設定").clicked() {
                        self.screen = Screen::Settings;
                    }
                });
            });
        });
    }

    fn render_setup_run(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if ui
                .add_sized(
                    egui::vec2(ui.available_width(), 40.0),
                    egui::Button::new("校正を開始"),
                )
                .clicked()
            {
                self.screen = Screen::Result;
            }
            ui.add_space(12.0);
            ui.label("プロンプト:");
            let mut project_prompt = self
                .state
                .lock()
                .expect("project state lock poisoned")
                .project_prompt
                .clone();
            ui.add_sized(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::TextEdit::multiline(&mut project_prompt)
                    .hint_text("どのような動画か、どのような視聴者かなどを入力"),
            );
            if let Ok(mut state) = self.state.lock() {
                state.project_prompt = project_prompt;
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(message) = &self.status_message {
                ui.label(message);
                ui.add_space(8.0);
            }

            let mut selected_preset = self.credentials.preset;
            egui::ComboBox::from_label("プリセット")
                .selected_text(selected_preset.display_name())
                .show_ui(ui, |ui| {
                    for preset in Preset::ALL {
                        ui.selectable_value(&mut selected_preset, preset, preset.display_name());
                    }
                });
            if selected_preset != self.credentials.preset {
                self.credentials.preset = selected_preset;
                let defaults = selected_preset.defaults();
                self.credentials.base_url = defaults.base_url.to_string();
                self.credentials.model = defaults.model.to_string();
            }

            ui.add_space(8.0);
            ui.label("エンドポイント");
            ui.text_edit_singleline(&mut self.credentials.base_url);
            ui.add_space(4.0);
            ui.label("モデル");
            ui.text_edit_singleline(&mut self.credentials.model);
            ui.add_space(4.0);
            ui.label("APIキー");
            ui.add(egui::TextEdit::singleline(&mut self.credentials.api_key).password(true));
            ui.add_space(4.0);
            let key_url = self.credentials.preset.defaults().key_url;
            ui.hyperlink_to("APIキーの発行ページを開く", key_url);
            ui.add_space(12.0);

            let can_save = self.credentials_path.is_some()
                && !self.credentials.base_url.trim().is_empty()
                && !self.credentials.model.trim().is_empty()
                && !self.credentials.api_key.trim().is_empty();
            let save_response = ui.add_enabled(
                can_save,
                egui::Button::new("保存").min_size(egui::vec2(
                    ui.available_width(),
                    ui.spacing().interact_size.y,
                )),
            );
            if save_response.clicked() {
                if let Some(path) = &self.credentials_path {
                    match save_credentials(path, &self.credentials) {
                        Ok(_) => {
                            self.status_message = Some("設定を保存しました。".to_string());
                            self.screen = Screen::SetupRun;
                        }
                        Err(err) => {
                            self.status_message = Some(format!("設定の保存に失敗しました: {err}"));
                        }
                    }
                }
            }

            if ui.button("戻る").clicked() && self.credentials_path.is_some() {
                self.screen = Screen::SetupRun;
            }
        });
    }

    fn render_result(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            if ui
                .add_sized(
                    egui::vec2(ui.available_width(), 32.0),
                    egui::Button::new("戻る"),
                )
                .clicked()
            {
                self.screen = Screen::SetupRun;
            }
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label("全体の指摘・コメント");
                let all = self
                    .result
                    .as_ref()
                    .map(|v| v.all.as_str())
                    .unwrap_or("（まだ結果はありません）");
                ui.label(all);
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(result) = &self.result {
                    for detail in &result.details {
                        ui.group(|ui| {
                            ui.label(format!("位置: {}", detail.id));
                            ui.label(format!("優先度: {:?}", detail.priority));
                            ui.horizontal(|ui| {
                                ui.add(egui::Button::new("ジャンプ"));
                                ui.add(egui::Button::new("メモを追加"));
                            });
                            ui.separator();
                            ui.label(&detail.comment);
                        });
                        ui.add_space(4.0);
                    }
                }
            });
        });
    }
}

impl eframe::App for ProofreadGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render_header(ui);
        match self.screen {
            Screen::SetupRun => self.render_setup_run(ui),
            Screen::Settings => self.render_settings(ui),
            Screen::Result => self.render_result(ui),
        }
    }
}

pub(crate) fn create_gui(
    cc: &eframe::CreationContext<'_>,
    handle: AviUtl2EframeHandle,
    state: Arc<Mutex<ProjectData>>,
) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Box::new(ProofreadGuiApp::new(cc, handle, state)))
}
