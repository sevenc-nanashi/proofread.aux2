use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};
use std::sync::{Arc, Mutex, mpsc};

use crate::ProjectData;
use crate::config::{Credentials, Preset, credentials_path, load_credentials, save_credentials};
use crate::prompt::{self, PromptTemplate};
use crate::result::{CommentPart, DetailAction, ProofreadResult, parse_detail_comment_actions};
use crate::service::ProofreadService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Prepare,
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
    prompt_templates: Vec<PromptTemplate>,
    status_message: Option<String>,
    proofreading_rx: Option<mpsc::Receiver<Result<ProofreadResult, String>>>,
}

impl ProofreadGuiApp {
    pub(crate) fn new(
        cc: &eframe::CreationContext<'_>,
        handle: AviUtl2EframeHandle,
        state: Arc<Mutex<ProjectData>>,
    ) -> Self {
        cc.egui_ctx.set_fonts(aviutl2_eframe::aviutl2_fonts());

        cc.egui_ctx.all_styles_mut(|style| {
            style.visuals = aviutl2_eframe::aviutl2_visuals();
        });

        let mut startup_statuses = Vec::new();
        let prompt_templates = match prompt::list_prompt_templates() {
            Ok(v) => v,
            Err(err) => {
                startup_statuses.push(format!("prompts フォルダの初期化に失敗しました: {err}"));
                vec![PromptTemplate {
                    id: prompt::BUILTIN_TEMPLATE_ID.to_string(),
                    label: prompt::BUILTIN_TEMPLATE_LABEL.to_string(),
                }]
            }
        };

        let (screen, credentials_path, credentials, credentials_status) = match credentials_path() {
            Ok(path) => match load_credentials(&path) {
                Ok(credentials) => (Screen::Prepare, Some(path), credentials, None),
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
        if let Some(message) = credentials_status {
            startup_statuses.push(message);
        }
        let status_message = if startup_statuses.is_empty() {
            None
        } else {
            Some(startup_statuses.join("\n"))
        };

        Self {
            _handle: handle,
            state,
            screen,
            result: None,
            credentials_path,
            credentials,
            prompt_templates,
            status_message,
            proofreading_rx: None,
        }
    }

    fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    fn render_header(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("proofread.aux2");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // if ui.button("設定").clicked() {
                    if ui
                        .add_enabled(self.screen == Screen::Prepare, egui::Button::new("設定"))
                        .clicked()
                    {
                        self.clear_status_message();
                        self.screen = Screen::Settings;
                    }
                });
            });
        });
    }

    fn render_setup_run(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(message) = &self.status_message {
                ui.label(message);
                ui.add_space(8.0);
            }
            ui.horizontal(|ui| {
                ui.label("プロンプトテンプレート:");

                let mut state = self.state.lock().expect("project state lock poisoned");
                if !self
                    .prompt_templates
                    .iter()
                    .any(|v| v.id == state.prompt_template_id)
                {
                    state.prompt_template_id = prompt::BUILTIN_TEMPLATE_ID.to_string();
                }
                let selected_id = state.prompt_template_id.clone();

                egui::ComboBox::from_id_salt("prompt_template")
                    .selected_text(self.label_for_template(&selected_id))
                    .show_ui(ui, |ui| {
                        for template in &self.prompt_templates {
                            ui.selectable_value(
                                &mut state.prompt_template_id,
                                template.id.clone(),
                                template.label.clone(),
                            );
                        }
                    });
                drop(state);

                if ui.button("再読込").clicked() {
                    self.reload_prompt_templates();
                }
                if ui.button("開く").clicked() && let Ok(prompts_dir) = crate::prompt::prompts_dir() {
                    open::that(prompts_dir).ok();
                }
            });
            ui.add_space(8.0);
            let start_response = ui
                .add_sized(
                    egui::vec2(ui.available_width(), 40.0),
                    egui::Button::new("校正を開始"),
                )
                .on_hover_text("Shift+クリックでMarkdownをコピー");
            if start_response.clicked() {
                if ui.input(|input| input.modifiers.shift) {
                    self.copy_prompt_markdown(ui.ctx());
                } else {
                    self.start_proofread();
                }
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
                    .hint_text("どのような動画か、どのような視聴者層か、キャラクターの口調はなにか、などを入力"),
            );
            if let Ok(mut state) = self.state.lock() {
                state.project_prompt = project_prompt;
            }
        });
    }

    fn render_settings(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            if let Some(message) = &self.status_message {
                ui.label(message);
                ui.add_space(8.0);
            }

            ui.vertical(|ui| {
                let mut selected_preset = self.credentials.preset;
                let full_width = ui.available_width();
                let full_width_size = egui::Vec2 {
                    x: full_width,
                    y: 32.0,
                };
                ui.label("プリセット");
                egui::ComboBox::new("preset", "")
                    .selected_text(selected_preset.display_name())
                    .width(full_width)
                    .show_ui(ui, |ui| {
                        for preset in Preset::ALL {
                            ui.selectable_value(
                                &mut selected_preset,
                                preset,
                                preset.display_name(),
                            );
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
                // ui.text_edit_singleline(&mut self.credentials.base_url);
                ui.add(
                    egui::TextEdit::singleline(&mut self.credentials.base_url)
                        .hint_text("例: https://api.openai.com/v1")
                        .font(egui::TextStyle::Monospace)
                        .min_size(full_width_size),
                );
                ui.add_space(4.0);
                ui.label("モデル");
                // ui.text_edit_singleline(&mut self.credentials.model);
                ui.add(
                    egui::TextEdit::singleline(&mut self.credentials.model)
                        .hint_text("例: gpt-5.5")
                        .font(egui::TextStyle::Monospace)
                        .min_size(full_width_size),
                );
                ui.add_space(4.0);
                ui.label("APIキー");
                ui.add(
                    egui::TextEdit::singleline(&mut self.credentials.api_key)
                        .password(true)
                        .font(egui::TextStyle::Monospace)
                        .min_size(full_width_size),
                );
                ui.add_space(4.0);
                let key_url = self.credentials.preset.defaults().key_url;
                ui.hyperlink_to("APIキーの発行ページを開く", key_url);
                ui.add_space(12.0);

                ui.columns_const(|[left, right]| {
                    let can_save = self.credentials_path.is_some()
                        && !self.credentials.base_url.trim().is_empty()
                        && !self.credentials.model.trim().is_empty()
                        && !self.credentials.api_key.trim().is_empty();
                    let save_response = left.add_enabled(can_save, egui::Button::new("保存"));
                    if save_response.clicked()
                        && let Some(path) = &self.credentials_path
                    {
                        match save_credentials(path, &self.credentials) {
                            Ok(_) => {
                                self.status_message = Some("設定を保存しました。".to_string());
                                self.screen = Screen::Prepare;
                            }
                            Err(err) => {
                                self.status_message =
                                    Some(format!("設定の保存に失敗しました: {err}"));
                            }
                        }
                    }

                    if right.button("戻る").clicked() && self.credentials_path.is_some() {
                        self.screen = Screen::Prepare;
                    }
                });
            });
        });
    }

    fn render_result(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
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
                self.clear_status_message();
                self.screen = Screen::Prepare;
            }
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label("全体の指摘・コメント");
                ui.separator();
                if let Some(all) = self.result.as_ref().map(|v| v.all.as_str()) {
                    let action = self.render_comment(ui, all);
                    if let Some(action) = action {
                        self.process_comment_action(action);
                    }
                } else {
                    ui.label("校正結果がありません。");
                }
            });
            ui.add_space(8.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                if let Some(result) = self.result.clone() {
                    for detail_index in
                        resolved_last_indices(result.details.iter().map(|detail| detail.resolved))
                    {
                        let detail = &result.details[detail_index];
                        ui.group(|ui| {
                            if detail.resolved {
                                ui.set_opacity(0.55);
                            }
                            ui.label(format!(
                                "位置: レイヤー{} / フレーム{}",
                                detail.position.layer + 1,
                                detail.position.start + 1
                            ));
                            ui.label(format!("優先度: {}", detail.priority.label_ja()));
                            ui.horizontal(|ui| {
                                if ui.add(egui::Button::new("ジャンプ")).clicked() {
                                    self.jump_to_object(detail.object, detail.position);
                                }
                                if ui.add(egui::Button::new("メモを追加")).clicked() {
                                    self.add_memo_to_object(detail.object, detail.position);
                                }
                                let resolved_button_label = if detail.resolved {
                                    "未解決に戻す"
                                } else {
                                    "解決済にする"
                                };
                                if ui.add(egui::Button::new(resolved_button_label)).clicked() {
                                    self.set_detail_resolved(detail_index, !detail.resolved);
                                }
                            });
                            ui.separator();
                            let action = self.render_comment(ui, &detail.comment);
                            if let Some(action) = action {
                                if matches!(action, DetailAction::Suggestion { .. }) {
                                    self.set_detail_resolved(detail_index, true);
                                }
                                self.process_comment_action(action);
                            }
                        });
                        ui.add_space(4.0);
                    }
                }
            });
        });
    }

    fn set_detail_resolved(&mut self, detail_index: usize, resolved: bool) {
        let result = self
            .result
            .as_mut()
            .expect("proofreading result must exist while rendering results");
        let detail = result
            .details
            .get_mut(detail_index)
            .expect("rendered detail index must exist");
        detail.resolved = resolved;
    }

    fn render_comment(
        &self,
        ui: &mut egui::Ui,
        comment: &str,
    ) -> Option<crate::result::DetailAction> {
        let parsed = parse_detail_comment_actions(comment);
        let mut action_to_return = None;
        ui.horizontal_wrapped(|ui| {
            ui.style_mut().spacing.item_spacing.x = 0.0;
            for part in &parsed.parts {
                match part {
                    CommentPart::Text(text) => {
                        ui.label(text);
                    }
                    CommentPart::Action { label, action } => {
                        let response = ui.link(label);
                        if let DetailAction::Jump { target_id } = action
                            && let Ok(tooltip) = jump_target_tooltip(target_id)
                        {
                            response.clone().on_hover_text(tooltip);
                        }
                        if response.clicked() {
                            action_to_return = Some(action.clone());
                        }
                    }
                }
            }
        });

        action_to_return
    }
    fn process_comment_action(&mut self, action: crate::result::DetailAction) {
        match action {
            DetailAction::Jump { target_id } => {
                self.jump_to_target_id(&target_id);
            }
            DetailAction::Suggestion { replacement } => {
                if let Some(result) = &self.result {
                    for detail in &result.details {
                        if detail.comment.contains(&replacement) {
                            self.replace_text(detail.object, detail.position, &replacement);
                            break;
                        }
                    }
                }
            }
        }
    }

    fn add_memo_to_object(
        &mut self,
        object: aviutl2::generic::ObjectHandle,
        position: aviutl2::generic::ObjectLayerFrame,
    ) {
        crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                edit.set_cursor_layer_frame(position.layer, position.start)?;
                edit.set_display_layer_frame(position.layer, position.start)?;
                if edit.get_object_layer_frame(object).is_ok() {
                    edit.set_focus_object(Some(object))?;
                    edit.create_effect(object, crate::marker::MEMO_MARKER_NAME)?;
                    Ok::<bool, aviutl2::generic::EditSectionError>(true)
                } else {
                    Ok::<bool, aviutl2::generic::EditSectionError>(false)
                }
            })
            .map_err(|err| {
                self.status_message = Some(format!("メモ追加処理に失敗しました: {err}"));
            })
            .ok();
    }
    fn replace_text(
        &mut self,
        object: aviutl2::generic::ObjectHandle,
        position: aviutl2::generic::ObjectLayerFrame,
        replacement: &str,
    ) {
        crate::EDIT_HANDLE
            .call_edit_section(|edit| {
                edit.set_cursor_layer_frame(position.layer, position.start)?;
                edit.set_display_layer_frame(position.layer, position.start)?;
                if edit.get_object_layer_frame(object).is_ok() {
                    edit.set_focus_object(Some(object))?;
                    for effect in edit.get_effects(object)? {
                        if edit
                            .set_effect_item_value(effect, "テキスト", replacement)
                            .is_ok()
                        {
                            break;
                        }
                    }
                    Ok::<bool, aviutl2::generic::EditSectionError>(true)
                } else {
                    Ok::<bool, aviutl2::generic::EditSectionError>(false)
                }
            })
            .map_err(|err| {
                self.status_message = Some(format!("テキスト置換処理に失敗しました: {err}"));
            })
            .ok();
    }

    fn render_running(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
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
        let (project_prompt, prompt_template_id) = {
            let state = self.state.lock().expect("project state lock poisoned");
            (
                state.project_prompt.clone(),
                state.prompt_template_id.clone(),
            )
        };
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
            let run_result = ProofreadService::run(
                &prompt_template_id,
                &project_info,
                &project_prompt,
                &targets.targets,
                &targets.locations,
                &credentials,
            )
            .map_err(|err| err.to_string());
            let _ = tx.send(run_result);
        });
        self.proofreading_rx = Some(rx);
        self.clear_status_message();
        self.screen = Screen::Running;
    }

    fn copy_prompt_markdown(&mut self, context: &egui::Context) {
        self.status_message = None;
        let (project_prompt, prompt_template_id) = {
            let state = self.state.lock().expect("project state lock poisoned");
            (
                state.project_prompt.clone(),
                state.prompt_template_id.clone(),
            )
        };
        let project_info = match crate::scan::collect_project_info() {
            Ok(value) => value,
            Err(err) => {
                self.status_message = Some(format!("プロジェクト情報の取得に失敗しました: {err}"));
                return;
            }
        };
        let targets = match crate::scan::collect_marked_targets() {
            Ok(value) => value,
            Err(err) => {
                self.status_message = Some(format!("校正対象の収集に失敗しました: {err}"));
                return;
            }
        };
        let markdown = match ProofreadService::build_prompt(
            &prompt_template_id,
            &project_info,
            &project_prompt,
            &targets.targets,
        ) {
            Ok(value) => value,
            Err(err) => {
                self.status_message = Some(format!("Markdownの生成に失敗しました: {err}"));
                return;
            }
        };

        context.copy_text(markdown);
        self.status_message = Some("校正用Markdownをコピーしました。".to_string());
    }

    fn label_for_template(&self, id: &str) -> String {
        self.prompt_templates
            .iter()
            .find(|v| v.id == id)
            .map(|v| v.label.clone())
            .unwrap_or_else(|| format!("不明なテンプレート: {id}"))
    }

    fn reload_prompt_templates(&mut self) {
        match prompt::list_prompt_templates() {
            Ok(v) => {
                self.prompt_templates = v;
                self.status_message = Some("プロンプトテンプレートを再読込しました。".to_string());
            }
            Err(err) => {
                self.status_message = Some(format!(
                    "プロンプトテンプレートの再読込に失敗しました: {err}"
                ));
            }
        }
    }

    fn jump_to_target_id(&mut self, target_id: &str) {
        let (layer, frame) = match parse_target_id(target_id) {
            Ok(v) => v,
            Err(err) => {
                self.status_message = Some(format!("ジャンプIDが不正です（{target_id}）: {err}"));
                return;
            }
        };

        match crate::EDIT_HANDLE.call_edit_section(|edit| {
            edit.set_cursor_layer_frame(layer, frame)?;
            edit.set_display_layer_frame(layer, frame)?;
            if let Some(handle) = edit.find_object_after(layer, frame)? {
                edit.set_focus_object(Some(handle))?;
                Ok::<bool, aviutl2::generic::EditSectionError>(true)
            } else {
                Ok::<bool, aviutl2::generic::EditSectionError>(false)
            }
        }) {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => {
                self.status_message = Some(format!("オブジェクトが見つかりませんでした。"));
            }
            Ok(Err(err)) => {
                self.status_message = Some(format!("ジャンプ処理に失敗しました: {err}"));
            }
            Err(err) => {
                self.status_message = Some(format!("ジャンプ処理に失敗しました: {err}"));
            }
        }
    }

    fn jump_to_object(
        &mut self,
        object: aviutl2::generic::ObjectHandle,
        position: aviutl2::generic::ObjectLayerFrame,
    ) {
        match crate::EDIT_HANDLE.call_edit_section(|edit| {
            edit.set_cursor_layer_frame(position.layer, position.start)?;
            edit.set_display_layer_frame(position.layer, position.start)?;
            edit.set_focus_object(Some(object))
        }) {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                self.status_message = Some(format!("ジャンプ処理に失敗しました: {err}"));
            }
            Err(err) => {
                self.status_message = Some(format!("ジャンプ処理に失敗しました: {err}"));
            }
        }
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
                self.screen = Screen::Prepare;
                self.proofreading_rx = None;
            }
            Err(mpsc::TryRecvError::Empty) => {}
            Err(mpsc::TryRecvError::Disconnected) => {
                self.status_message = Some("校正処理が中断されました。".to_string());
                self.screen = Screen::Prepare;
                self.proofreading_rx = None;
            }
        }
    }
}

fn resolved_last_indices(resolved: impl IntoIterator<Item = bool>) -> Vec<usize> {
    let mut unresolved = Vec::new();
    let mut resolved_indices = Vec::new();
    for (index, is_resolved) in resolved.into_iter().enumerate() {
        if is_resolved {
            resolved_indices.push(index);
        } else {
            unresolved.push(index);
        }
    }
    unresolved.extend(resolved_indices);
    unresolved
}

fn jump_target_tooltip(target_id: &str) -> Result<String, &'static str> {
    let (layer, frame) = parse_target_id(target_id)?;
    Ok(format!(
        "ジャンプ先: レイヤー{} / フレーム{}",
        layer + 1,
        frame + 1
    ))
}

fn parse_target_id(input: &str) -> Result<(usize, usize), &'static str> {
    let mut layer = None;
    let mut frame = None;
    for part in input.split('-') {
        if let Some(v) = part.strip_prefix('l') {
            layer = v.parse::<usize>().ok();
            continue;
        }
        if let Some(v) = part.strip_prefix('f') {
            frame = v.parse::<usize>().ok();
            continue;
        }
    }
    match (layer, frame) {
        (Some(layer), Some(frame)) => Ok((layer, frame)),
        (None, _) => Err("layer が見つかりません"),
        (_, None) => Err("frame が見つかりません"),
    }
}

impl eframe::App for ProofreadGuiApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_proofreading_result();
        self.render_header(ui);
        match self.screen {
            Screen::Prepare => self.render_setup_run(ui),
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

#[cfg(test)]
mod tests {
    use super::{jump_target_tooltip, parse_target_id, resolved_last_indices};

    #[test]
    fn parse_target_id_extracts_layer_and_frame() {
        assert_eq!(parse_target_id("l10-f32-n10").ok(), Some((10, 32)));
    }

    #[test]
    fn parse_target_id_rejects_invalid_input() {
        assert!(parse_target_id("f32-n10").is_err());
        assert!(parse_target_id("l10-n10").is_err());
        assert!(parse_target_id("abc").is_err());
    }

    #[test]
    fn resolved_details_are_ordered_last_stably() {
        assert_eq!(
            resolved_last_indices([true, false, true, false]),
            vec![1, 3, 0, 2]
        );
    }

    #[test]
    fn jump_target_tooltip_describes_destination() {
        assert_eq!(
            jump_target_tooltip("l10-f32-n10").as_deref(),
            Ok("ジャンプ先: レイヤー11 / フレーム33")
        );
        assert!(jump_target_tooltip("invalid").is_err());
    }
}
