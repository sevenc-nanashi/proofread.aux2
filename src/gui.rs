use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use std::sync::{Arc, Mutex, mpsc};

use crate::ProjectData;
use crate::config::{Credentials, Preset, credentials_path, load_credentials, save_credentials};
use crate::result::{DetailAction, ProofreadResult, parse_detail_comment_actions};
use crate::service::ProofreadService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    SetupRun,
    Settings,
    Running,
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
    proofreading_rx: Option<mpsc::Receiver<Result<ProofreadResult, String>>>,
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
            proofreading_rx: None,
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
            if let Some(message) = &self.status_message {
                ui.label(message);
                ui.add_space(8.0);
            }
            if ui
                .add_sized(
                    egui::vec2(ui.available_width(), 40.0),
                    egui::Button::new("校正を開始"),
                )
                .clicked()
            {
                self.start_proofread();
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
            if let Some(message) = &self.status_message {
                ui.label(message);
                ui.add_space(8.0);
            }
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
                ui.separator();
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
                        let parsed = parse_detail_comment_actions(&detail.comment);
                        ui.group(|ui| {
                            ui.label(format!("位置: {}", detail.id));
                            ui.label(format!("優先度: {}", detail.priority.label_ja()));
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new("ジャンプ")).clicked() {
                                    self.status_message = Some(
                                        "ジャンプ機能は未実装です。記法付きボタンもしくは今後の実装を利用してください。"
                                            .to_string(),
                                    );
                                }
                                if ui.add(egui::Button::new("メモを追加")).clicked() {
                                    self.status_message =
                                        Some("メモ追加機能は未実装です。".to_string());
                                }
                            });
                            if !parsed.actions.is_empty() {
                                ui.add_space(4.0);
                                ui.horizontal_wrapped(|ui| {
                                    for action in &parsed.actions {
                                        match action {
                                            DetailAction::Jump { label, target_id } => {
                                                if ui
                                                    .add(egui::Button::new(format!(
                                                        "ジャンプ: {label}"
                                                    )))
                                                    .clicked()
                                                {
                                                    self.status_message = Some(format!(
                                                        "ジャンプ機能は未実装です（対象ID: {target_id}）。"
                                                    ));
                                                }
                                            }
                                            DetailAction::Suggestion { replacement } => {
                                                if ui
                                                    .add(egui::Button::new(
                                                        "テキストをこの範囲内で置き換える",
                                                    ))
                                                    .clicked()
                                                {
                                                    self.status_message = Some(format!(
                                                        "置換適用機能は未実装です（候補: {replacement}）。"
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                            ui.separator();
                            ui.label(&parsed.body);
                        });
                        ui.add_space(4.0);
                    }
                }
            });
        });
    }

    fn render_running(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.centered_and_justified(|ui| {
                ui.vertical(|ui| {
                    ui.heading("校正中...");
                    ui.label("AIに問い合わせています。しばらくお待ちください。");
                });
            });
            ui.request_repaint_after(std::time::Duration::from_millis(100));
        });
    }

    fn start_proofread(&mut self) {
        self.status_message = None;
        let project_prompt = self
            .state
            .lock()
            .expect("project state lock poisoned")
            .project_prompt
            .clone();
        let project_info = match crate::scan::collect_project_info() {
            Ok(v) => v,
            Err(err) => {
                self.status_message = Some(format!("プロジェクト情報の取得に失敗しました: {err}"));
                return;
            }
        };
        let targets = match crate::scan::collect_marked_targets() {
            Ok(v) => v,
            Err(err) => {
                self.status_message = Some(format!("校正対象の収集に失敗しました: {err}"));
                return;
            }
        };

        let credentials = self.credentials.clone();
        let (tx, rx) = mpsc::channel::<Result<ProofreadResult, String>>();
        std::thread::spawn(move || {
            let run_result =
                ProofreadService::run(&project_info, &project_prompt, &targets, &credentials)
                    .map_err(|err| err.to_string());
            let _ = tx.send(run_result);
        });
        self.proofreading_rx = Some(rx);
        self.screen = Screen::Running;
    }

    fn poll_proofreading_result(&mut self) {
        let Some(rx) = &self.proofreading_rx else {
            return;
        };
        match rx.try_recv() {
            Ok(Ok(result)) => {
                self.result = Some(result);
                self.status_message = Some("校正が完了しました。".to_string());
                self.screen = Screen::Result;
                self.proofreading_rx = None;
            }
            Ok(Err(err_message)) => {
                self.status_message = Some(format!("校正に失敗しました: {err_message}"));
                self.screen = Screen::SetupRun;
                self.proofreading_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status_message = Some("校正処理が中断されました。".to_string());
                self.screen = Screen::SetupRun;
                self.proofreading_rx = None;
            }
        }
    }
}

impl eframe::App for ProofreadGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_proofreading_result();
        self.render_header(ui);
        match self.screen {
            Screen::SetupRun => self.render_setup_run(ui),
            Screen::Settings => self.render_settings(ui),
            Screen::Running => self.render_running(ui),
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
