use aviutl2_eframe::{AviUtl2EframeHandle, eframe, egui};

use crate::result::ProofreadResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    SetupRun,
    Result,
}

pub(crate) struct ProofreadGuiApp {
    _handle: AviUtl2EframeHandle,
    screen: Screen,
    project_prompt: String,
    result: Option<ProofreadResult>,
}

impl ProofreadGuiApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>, handle: AviUtl2EframeHandle) -> Self {
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

        Self {
            _handle: handle,
            screen: Screen::SetupRun,
            project_prompt: String::new(),
            result: None,
        }
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        egui::Panel::top("toolbar").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("proofread.aux2");
            });
        });
    }

    fn render_setup_run(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), 32.0),
                egui::Button::new("設定"),
            );
            ui.add_space(8.0);
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
            ui.add_sized(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::TextEdit::multiline(&mut self.project_prompt)
                    .hint_text("どのような動画か、どのような視聴者かなどを入力"),
            );
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
            Screen::Result => self.render_result(ui),
        }
    }
}

pub(crate) fn create_gui(
    cc: &eframe::CreationContext<'_>,
    handle: AviUtl2EframeHandle,
) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>> {
    Ok(Box::new(ProofreadGuiApp::new(cc, handle)))
}
