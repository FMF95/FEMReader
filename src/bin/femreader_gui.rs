use eframe::egui::{
    self, Color32, FontFamily, FontId, RichText, text::LayoutJob, text::TextFormat,
};
use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
use femreader::highlighting::{
    HighlightKind, RgbColor, SyntaxHighlighter, embedded_highlighting_xml,
    highlighting_file_for_solver,
};
use femreader::model::{
    FileCategory, FileNode, HyperMeshCommentFields, Model, ModelContent, OptiStructCardKind,
    OptiStructCommentKind, OptiStructItem, SolverKind,
};
use femreader::pipeline::ReaderPipeline;
use femreader::scanner::DefaultFileSystemScanner;
use femreader::serializer::JsonSerializer;
use femreader::solver::{AbaqusAdapter, NastranAdapter, OptiStructAdapter, SolverAdapter};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

fn main() -> eframe::Result<()> {
    let initial_directory = std::env::args().nth(1).unwrap_or_default();
    let native_options = eframe::NativeOptions::default();

    eframe::run_native(
        "FEMReader GUI",
        native_options,
        Box::new(move |_| {
            let mut app = FemReaderGuiApp::new(initial_directory.clone());
            if !app.input_directory.is_empty() {
                app.load_content();
            }
            Ok(Box::new(app))
        }),
    )
}

#[derive(Default)]
struct FemReaderGuiApp {
    input_directory: String,
    content: Option<ModelContent>,
    selected_model_index: Option<usize>,
    status_message: String,
    error_message: Option<String>,
    delete_cache_after_load: bool,
    highlight_enabled: bool,
    syntax_highlighting: SolverSyntaxHighlighting,
    visual_text_background: HighlightBackgroundChoice,
}

#[derive(Default)]
struct SolverSyntaxHighlighting {
    optistruct: Option<SyntaxHighlighter>,
    nastran: Option<SyntaxHighlighter>,
    abaqus: Option<SyntaxHighlighter>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum HighlightBackgroundChoice {
    #[default]
    GuiDefault,
    White,
    Cornsilk,
    Gray,
}

impl HighlightBackgroundChoice {
    const fn all() -> [Self; 4] {
        [Self::GuiDefault, Self::White, Self::Cornsilk, Self::Gray]
    }

    const fn label(self) -> &'static str {
        match self {
            Self::GuiDefault => "Predeterminado GUI",
            Self::White => "Blanco (#FFFFFF)",
            Self::Cornsilk => "Cornsilk (#FFF8DC)",
            Self::Gray => "Gris (#A8A8A8)",
        }
    }

    const fn color(self) -> Option<Color32> {
        match self {
            Self::GuiDefault => None,
            Self::White => Some(Color32::from_rgb(0xFF, 0xFF, 0xFF)),
            Self::Cornsilk => Some(Color32::from_rgb(0xFF, 0xF8, 0xDC)),
            Self::Gray => Some(Color32::from_rgb(0xA8, 0xA8, 0xA8)),
        }
    }
}

impl SolverSyntaxHighlighting {
    fn load() -> Self {
        Self {
            optistruct: load_highlighter_for_solver(&SolverKind::OptiStruct),
            nastran: load_highlighter_for_solver(&SolverKind::Nastran),
            abaqus: load_highlighter_for_solver(&SolverKind::Abaqus),
        }
    }

    fn for_solver(&self, solver: &SolverKind) -> Option<&SyntaxHighlighter> {
        match solver {
            SolverKind::OptiStruct => self.optistruct.as_ref(),
            SolverKind::Nastran => self.nastran.as_ref(),
            SolverKind::Abaqus => self.abaqus.as_ref(),
            SolverKind::Unknown => None,
        }
    }
}

impl FemReaderGuiApp {
    fn new(input_directory: String) -> Self {
        Self {
            input_directory,
            content: None,
            selected_model_index: None,
            status_message: String::new(),
            error_message: None,
            delete_cache_after_load: true,
            highlight_enabled: true,
            syntax_highlighting: SolverSyntaxHighlighting::load(),
            visual_text_background: HighlightBackgroundChoice::GuiDefault,
        }
    }

    fn load_content(&mut self) {
        self.error_message = None;
        let input = PathBuf::from(self.input_directory.trim());

        if self.input_directory.trim().is_empty() {
            self.content = None;
            self.selected_model_index = None;
            self.status_message.clear();
            return;
        }

        if !input.exists() || !input.is_dir() {
            self.error_message = Some(format!(
                "El directorio no existe o no es válido: {}",
                display_path_for_gui(&input)
            ));
            return;
        }

        match run_pipeline(&input, self.delete_cache_after_load) {
            Ok(content) => {
                let model_files = content
                    .files
                    .iter()
                    .filter(|file| file.category == FileCategory::Model)
                    .count();
                let non_model_files = content.files.len().saturating_sub(model_files);
                let optistruct_items = content.optistruct_items.len();
                self.status_message = format!(
                    "Cargado: {} modelos, {} archivos de modelo, {} archivos no modelo, {} dependencias, {} items OptiStruct.",
                    content.models.len(),
                    model_files,
                    non_model_files,
                    content.dependencies.len(),
                    optistruct_items
                );
                self.selected_model_index = if content.models.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.content = Some(content);
            }
            Err(error) => {
                self.error_message = Some(error);
            }
        }
    }

    fn render_model_details(
        ui: &mut egui::Ui,
        content: &ModelContent,
        model: &Model,
        syntax_highlighting: &SolverSyntaxHighlighting,
        highlight_enabled: bool,
        background: Option<Color32>,
    ) {
        ui.heading(&model.name);
        ui.label(format!("ID: {}", model.id));
        if let Some(root_file) = Self::file_by_id(content, &model.root_file_id) {
            ui.label(format!("Archivo raíz: {}", root_file.name));
            ui.label(format!(
                "Path raíz: {}",
                display_path_for_gui(&root_file.path)
            ));
        } else {
            ui.label(format!("Archivo raíz: {}", model.root_file_id));
        }
        ui.label(format!("Solver: {}", solver_label(&model.solver)));
        ui.label(format!("Archivos en el modelo: {}", model.file_ids.len()));
        ui.separator();

        full_width_collapsing(ui, "Archivos del modelo", |ui| {
            for file_id in &model.file_ids {
                if let Some(file) = Self::file_by_id(content, file_id) {
                    ui.collapsing(file.name.clone(), |ui| {
                        ui.label(format!("Path: {}", display_path_for_gui(&file.path)));
                        ui.label(format!("ID: {}", file.id));
                    });
                } else {
                    ui.label(file_id);
                }
            }
        });

        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();

        full_width_collapsing(ui, "Dependencias del modelo", |ui| {
            for edge in &content.dependencies {
                if model_file_ids.contains(edge.from_file_id.as_str())
                    || model_file_ids.contains(edge.to_file_id.as_str())
                {
                    let from = Self::file_by_id(content, &edge.from_file_id)
                        .map(|file| file.name.clone())
                        .unwrap_or_else(|| edge.from_file_id.clone());
                    let to = Self::file_by_id(content, &edge.to_file_id)
                        .map(|file| file.name.clone())
                        .unwrap_or_else(|| edge.to_file_id.clone());
                    ui.label(format!("{} -> {} ({})", from, to, edge.reference));
                }
            }
        });

        full_width_collapsing(ui, "Estructura OptiStruct (visual)", |ui| {
            Self::render_optistruct_structure(
                ui,
                content,
                model,
                syntax_highlighting,
                highlight_enabled,
                background,
            );
        });

        full_width_collapsing(ui, "Resumen OptiStruct del modelo", |ui| {
            Self::render_optistruct_model_summary(ui, content, model);
        });

        full_width_collapsing(ui, "Resumen OptiStruct por archivo", |ui| {
            Self::render_optistruct_file_summaries(ui, content, model);
        });
    }

    fn render_optistruct_structure(
        ui: &mut egui::Ui,
        content: &ModelContent,
        model: &Model,
        syntax_highlighting: &SolverSyntaxHighlighting,
        highlight_enabled: bool,
        background: Option<Color32>,
    ) {
        let highlighter =
            resolve_visual_highlighter(highlight_enabled, syntax_highlighting, &model.solver);
        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();
        let mut grouped = BTreeMap::<(String, String, String), Vec<&OptiStructItem>>::new();

        for item in &content.optistruct_items {
            let (file_id, file_name, file_path) = match item {
                OptiStructItem::Comment(comment) => (
                    comment.location.file_id.as_str(),
                    comment.location.file_name.clone(),
                    display_path_for_gui(&comment.location.file_path),
                ),
                OptiStructItem::Text(text) => (
                    text.location.file_id.as_str(),
                    text.location.file_name.clone(),
                    display_path_for_gui(&text.location.file_path),
                ),
                OptiStructItem::Card(card) => (
                    card.location.file_id.as_str(),
                    card.location.file_name.clone(),
                    display_path_for_gui(&card.location.file_path),
                ),
            };

            if model_file_ids.contains(file_id) {
                grouped
                    .entry((file_name, file_id.to_string(), file_path))
                    .or_default()
                    .push(item);
            }
        }

        if grouped.is_empty() {
            ui.label("No hay tarjetas/comentarios OptiStruct para este modelo.");
            return;
        }

        for ((file_name, file_id, file_path), mut items) in grouped {
            items.sort_by_key(|item| match item {
                OptiStructItem::Comment(comment) => comment.location.ordinal,
                OptiStructItem::Text(text) => text.location.ordinal,
                OptiStructItem::Card(card) => card.location.ordinal,
            });
            let visual_prefix_width = items
                .iter()
                .map(|item| visual_item_prefix(item).chars().count())
                .max()
                .unwrap_or(0);
            let text_column_width = visual_text_column_width_chars(&model.solver, &items);

            full_width_collapsing(ui, file_name, |ui| {
                ui.label(format!("Path: {}", file_path));
                ui.label(format!("ID: {}", file_id));
                ui.scope(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    for item in items {
                        match item {
                            OptiStructItem::Comment(comment) => {
                                let prefix = aligned_prefix_cell(
                                    &visual_item_prefix(item),
                                    visual_prefix_width,
                                );
                                let label = build_highlighted_collapsing_label(
                                    ui,
                                    &prefix,
                                    &visual_line_with_fixed_width(
                                        first_visual_line(&comment.text),
                                        text_column_width,
                                    ),
                                    highlighter,
                                    background,
                                );
                                full_width_collapsing(ui, label, |ui| {
                                    ui.label(format!("ID: {}", comment.id));
                                    render_highlighted_text_block(
                                        ui,
                                        "Texto:",
                                        &comment.text,
                                        highlighter,
                                        background,
                                        text_column_width,
                                    );
                                    ui.label(format!(
                                        "Path: {}",
                                        display_path_for_gui(&comment.location.file_path)
                                    ));
                                    ui.label(format!("File ID: {}", comment.location.file_id));
                                    ui.label(format!("Ordinal: {}", comment.location.ordinal));
                                    if let OptiStructCommentKind::HyperMeshComment {
                                        fields, ..
                                    } = &comment.kind
                                    {
                                        render_hypermesh_fields(ui, fields);
                                    }
                                });
                            }
                            OptiStructItem::Text(text) => {
                                let prefix = aligned_prefix_cell(
                                    &visual_item_prefix(item),
                                    visual_prefix_width,
                                );
                                let label = build_highlighted_collapsing_label(
                                    ui,
                                    &prefix,
                                    &visual_line_with_fixed_width(
                                        first_visual_line(&text.text),
                                        text_column_width,
                                    ),
                                    highlighter,
                                    background,
                                );
                                full_width_collapsing(ui, label, |ui| {
                                    ui.label(format!("ID: {}", text.id));
                                    render_highlighted_text_block(
                                        ui,
                                        "Texto:",
                                        &text.text,
                                        highlighter,
                                        background,
                                        text_column_width,
                                    );
                                    ui.label(format!(
                                        "Path: {}",
                                        display_path_for_gui(&text.location.file_path)
                                    ));
                                    ui.label(format!("File ID: {}", text.location.file_id));
                                    ui.label(format!("Ordinal: {}", text.location.ordinal));
                                });
                            }
                            OptiStructItem::Card(card) => {
                                let prefix = aligned_prefix_cell(
                                    &visual_item_prefix(item),
                                    visual_prefix_width,
                                );
                                let label = build_highlighted_collapsing_label(
                                    ui,
                                    &prefix,
                                    &visual_line_with_fixed_width(
                                        first_visual_line(&card.text),
                                        text_column_width,
                                    ),
                                    highlighter,
                                    background,
                                );
                                full_width_collapsing(ui, label, |ui| {
                                    ui.label(format!("ID: {}", card.id));
                                    render_highlighted_text_block(
                                        ui,
                                        "Línea(s) original(es):",
                                        &card.text,
                                        highlighter,
                                        background,
                                        text_column_width,
                                    );
                                    ui.label(format!("Tarjeta: {}", card.card_name));
                                    ui.label(format!("Ordinal: {}", card.location.ordinal));
                                    ui.label(format!(
                                        "Formato: {}",
                                        if card.long_format { "LONG" } else { "SHORT" }
                                    ));
                                    ui.label(format!(
                                        "Path: {}",
                                        display_path_for_gui(&card.location.file_path)
                                    ));
                                    ui.label(format!("File ID: {}", card.location.file_id));
                                    render_card_named_fields(ui, card);
                                });
                            }
                        }
                    }
                });
            });
        }
    }

    fn render_optistruct_model_summary(ui: &mut egui::Ui, content: &ModelContent, model: &Model) {
        let Some(summary) = content
            .optistruct_model_summaries
            .iter()
            .find(|entry| entry.model_id == model.id)
        else {
            ui.label("No hay resumen OptiStruct para este modelo.");
            return;
        };

        ui.label(format!("Items totales: {}", summary.total_items));
        ui.label(format!("Tarjetas: {}", summary.card_items));
        ui.label(format!(
            "Comentarios: {} (HyperMesh: {})",
            summary.comment_items, summary.hypermesh_comment_items
        ));
        ui.label(format!("Líneas de texto: {}", summary.text_items));
        ui.label(format!(
            "Líneas no vacías: {}",
            summary.total_non_empty_lines
        ));
        if summary.line_coverage_ok {
            ui.label(format!(
                "Check líneas: OK ({} / {}).",
                summary.covered_non_empty_lines, summary.total_non_empty_lines
            ));
        } else {
            ui.label(
                RichText::new(format!(
                    "Check líneas: revisar ({} / {}).",
                    summary.covered_non_empty_lines, summary.total_non_empty_lines
                ))
                .color(Color32::from_rgb(220, 70, 70)),
            );
        }
        if summary.card_counts.is_empty() {
            ui.label("Conteo de tarjetas: (sin tarjetas parseadas)");
        } else {
            ui.label("Conteo de tarjetas:");
            for entry in &summary.card_counts {
                ui.label(format!("  {}: {}", entry.card_name, entry.count));
            }
        }
    }

    fn render_optistruct_file_summaries(ui: &mut egui::Ui, content: &ModelContent, model: &Model) {
        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();
        let summaries = content
            .optistruct_file_summaries
            .iter()
            .filter(|summary| model_file_ids.contains(summary.file_id.as_str()))
            .collect::<Vec<_>>();

        if summaries.is_empty() {
            ui.label("No hay resúmenes OptiStruct para este modelo.");
            return;
        }

        let model_summary = content
            .optistruct_model_summaries
            .iter()
            .find(|summary| summary.model_id == model.id);

        let total_comments = model_summary
            .map(|summary| summary.comment_items)
            .unwrap_or_else(|| summaries.iter().map(|entry| entry.comment_items).sum());
        let total_hypermesh_comments = model_summary
            .map(|summary| summary.hypermesh_comment_items)
            .unwrap_or_else(|| {
                summaries
                    .iter()
                    .map(|entry| entry.hypermesh_comment_items)
                    .sum()
            });
        let total_text_items = model_summary
            .map(|summary| summary.text_items)
            .unwrap_or_else(|| summaries.iter().map(|entry| entry.text_items).sum());
        let total_cards = model_summary
            .map(|summary| summary.card_items)
            .unwrap_or_else(|| summaries.iter().map(|entry| entry.card_items).sum());
        let total_items = model_summary
            .map(|summary| summary.total_items)
            .unwrap_or_else(|| summaries.iter().map(|entry| entry.total_items).sum());
        let model_total_non_empty_lines = model_summary
            .map(|summary| summary.total_non_empty_lines)
            .unwrap_or_else(|| {
                summaries
                    .iter()
                    .map(|entry| entry.total_non_empty_lines)
                    .sum()
            });
        let model_covered_non_empty_lines = model_summary
            .map(|summary| summary.covered_non_empty_lines)
            .unwrap_or_else(|| {
                summaries
                    .iter()
                    .map(|entry| entry.covered_non_empty_lines)
                    .sum()
            });
        let model_line_coverage_ok = model_summary
            .map(|summary| summary.line_coverage_ok)
            .unwrap_or(model_total_non_empty_lines == model_covered_non_empty_lines);
        ui.label(format!(
            "Totales del modelo: {} items, {} tarjetas, {} comentarios ({} HyperMesh), {} líneas de texto.",
            total_items, total_cards, total_comments, total_hypermesh_comments, total_text_items
        ));
        if model_line_coverage_ok {
            ui.label(format!(
                "Check líneas (modelo): OK ({} / {}).",
                model_covered_non_empty_lines, model_total_non_empty_lines
            ));
        } else {
            ui.label(
                RichText::new(format!(
                    "Check líneas (modelo): revisar ({} / {}).",
                    model_covered_non_empty_lines, model_total_non_empty_lines
                ))
                .color(Color32::from_rgb(220, 70, 70)),
            );
        }
        ui.separator();

        for summary in summaries {
            full_width_collapsing(ui, &summary.file_name, |ui| {
                ui.label(format!(
                    "Path: {}",
                    display_path_for_gui(&summary.file_path)
                ));
                ui.label(format!("File ID: {}", summary.file_id));
                ui.label(format!(
                    "Líneas no vacías: {}",
                    summary.total_non_empty_lines
                ));
                if summary.line_coverage_ok {
                    ui.label(format!(
                        "Check líneas: OK ({} / {}).",
                        summary.covered_non_empty_lines, summary.total_non_empty_lines
                    ));
                } else {
                    ui.label(
                        RichText::new(format!(
                            "Check líneas: revisar ({} / {}).",
                            summary.covered_non_empty_lines, summary.total_non_empty_lines
                        ))
                        .color(Color32::from_rgb(220, 70, 70)),
                    );
                }
                ui.label(format!("Items totales: {}", summary.total_items));
                ui.label(format!("Tarjetas: {}", summary.card_items));
                ui.label(format!(
                    "Comentarios: {} (HyperMesh: {})",
                    summary.comment_items, summary.hypermesh_comment_items
                ));
                ui.label(format!("Líneas de texto: {}", summary.text_items));
                if summary.card_counts.is_empty() {
                    ui.label("Conteo de tarjetas: (sin tarjetas parseadas)");
                } else {
                    ui.label("Conteo de tarjetas:");
                    for entry in &summary.card_counts {
                        ui.label(format!("  {}: {}", entry.card_name, entry.count));
                    }
                }
            });
        }
    }

    fn file_by_id<'a>(content: &'a ModelContent, id: &str) -> Option<&'a FileNode> {
        content.files.iter().find(|file| file.id == id)
    }
}

impl eframe::App for FemReaderGuiApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let selected_background_text = self.visual_text_background.label();
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Directorio de modelos:");
                let text_edit = ui.text_edit_singleline(&mut self.input_directory);
                let pressed_enter =
                    text_edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                let trigger_load = ui.button("Cargar").clicked() || pressed_enter;
                ui.separator();
                ui.checkbox(
                    &mut self.delete_cache_after_load,
                    "Eliminar cache tras leer (.femreader-cache)",
                );
                ui.checkbox(&mut self.highlight_enabled, "Activar highlighting");
                ui.label("Fondo texto visual:");
                egui::ComboBox::from_id_salt("visual_text_background")
                    .selected_text(selected_background_text)
                    .show_ui(ui, |ui| {
                        for choice in HighlightBackgroundChoice::all() {
                            ui.selectable_value(
                                &mut self.visual_text_background,
                                choice,
                                choice.label(),
                            );
                        }
                    });

                if trigger_load {
                    self.load_content();
                }
            });

            if !self.status_message.is_empty() {
                ui.label(&self.status_message);
            }

            if let Some(error) = &self.error_message {
                ui.label(RichText::new(error).color(Color32::from_rgb(220, 70, 70)));
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(content) = &self.content else {
                ui.label("Carga un directorio para visualizar su contenido FEM.");
                return;
            };
            let mut selected_model_index = self.selected_model_index;
            egui::SidePanel::left("left_summary_panel")
                .resizable(true)
                .default_width(300.0)
                .width_range(220.0..=420.0)
                .show_inside(ui, |ui| {
                    ui.heading("Resumen");
                    ui.label(format!(
                        "Root: {}",
                        display_path_for_gui(&content.root_directory)
                    ));
                    ui.label(format!("Directorios: {}", content.directories.len()));
                    ui.label(format!("Archivos: {}", content.files.len()));
                    let model_files = content
                        .files
                        .iter()
                        .filter(|file| file.category == FileCategory::Model)
                        .count();
                    ui.label(format!("Archivos de modelo: {}", model_files));
                    ui.label(format!(
                        "Archivos no modelo: {}",
                        content.files.len().saturating_sub(model_files)
                    ));
                    ui.label(format!("Dependencias: {}", content.dependencies.len()));
                    ui.label(format!(
                        "Items OptiStruct: {}",
                        content.optistruct_items.len()
                    ));
                    ui.label(format!(
                        "Referencias sin resolver: {}",
                        content.unresolved_references.len()
                    ));
                    ui.separator();
                    ui.heading("Modelos");

                    egui::ScrollArea::vertical()
                        .max_height(360.0)
                        .show(ui, |ui| {
                            for (index, model) in content.models.iter().enumerate() {
                                let label = format!(
                                    "{} [{}] ({} archivos)",
                                    model.name,
                                    solver_label(&model.solver),
                                    model.file_ids.len()
                                );
                                let selected = self.selected_model_index == Some(index);
                                if ui.selectable_label(selected, label).clicked() {
                                    selected_model_index = Some(index);
                                }
                            }
                        });

                    ui.separator();
                    ui.collapsing("Archivos no modelo", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(160.0)
                            .show(ui, |ui| {
                                for file in &content.files {
                                    if file.category == FileCategory::NonModel {
                                        ui.collapsing(file.name.clone(), |ui| {
                                            ui.label(format!(
                                                "Path: {}",
                                                display_path_for_gui(&file.path)
                                            ));
                                            ui.label(format!("ID: {}", file.id));
                                        });
                                    }
                                }
                            });
                    });

                    ui.separator();
                    ui.collapsing("Referencias sin resolver", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(200.0)
                            .show(ui, |ui| {
                                for unresolved in &content.unresolved_references {
                                    ui.label(unresolved);
                                }
                            });
                    });
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                ui.heading("Detalle del modelo");
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if let Some(model_index) = selected_model_index {
                            if let Some(model) = content.models.get(model_index) {
                                let visual_background = self.visual_text_background.color();
                                Self::render_model_details(
                                    ui,
                                    content,
                                    model,
                                    &self.syntax_highlighting,
                                    self.highlight_enabled,
                                    visual_background,
                                );
                            } else {
                                ui.label("No hay modelo seleccionado.");
                            }
                        } else {
                            ui.label("No hay modelo seleccionado.");
                        }
                    });
            });
            self.selected_model_index = selected_model_index;
        });
    }
}

fn run_pipeline(
    root_directory: &Path,
    delete_cache_after_run: bool,
) -> Result<ModelContent, String> {
    let mut cache_path = root_directory.to_path_buf();
    cache_path.push(".femreader-cache");
    let mut cache = IncrementalCache::load(&cache_path)
        .map_err(|e| format!("No se pudo cargar cache {}: {e}", cache_path.display()))?;

    let adapters: Vec<Box<dyn SolverAdapter + Send + Sync>> = vec![
        Box::new(NastranAdapter),
        Box::new(OptiStructAdapter),
        Box::new(AbaqusAdapter),
    ];

    let pipeline = ReaderPipeline {
        scanner: DefaultFileSystemScanner,
        dependency_resolver: DefaultDependencyResolver::new(adapters),
        model_assembler: DefaultModelAssembler,
        serializer: JsonSerializer,
    };

    let content = pipeline
        .run(root_directory, &mut cache)
        .map_err(|e| format!("Falló la ejecución del lector: {e}"))?;

    if delete_cache_after_run {
        IncrementalCache::remove_file(&cache_path)
            .map_err(|e| format!("No se pudo eliminar cache {}: {e}", cache_path.display()))?;
    } else {
        cache
            .save(&cache_path)
            .map_err(|e| format!("No se pudo guardar cache {}: {e}", cache_path.display()))?;
    }

    Ok(content)
}

fn solver_label(kind: &SolverKind) -> &'static str {
    match kind {
        SolverKind::Unknown => "Unknown",
        SolverKind::Nastran => "NASTRAN",
        SolverKind::OptiStruct => "OptiStruct",
        SolverKind::Abaqus => "ABAQUS",
    }
}

fn full_width_collapsing(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    let label = label.into();
    ui.allocate_ui_with_layout(
        egui::vec2(width, 0.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_min_width(width);
            ui.collapsing(label, add_contents);
        },
    );
}

fn render_hypermesh_fields(ui: &mut egui::Ui, fields: &HyperMeshCommentFields) {
    match fields {
        HyperMeshCommentFields::HMName(fields) => {
            ui.label("HyperMesh (HMNAME):");
            ui.label(format!(
                "  entity_type: {}",
                format_optional_str(fields.entity_type.as_deref())
            ));
            ui.label(format!(
                "  id: {}",
                format_optional_str(fields.id.as_deref())
            ));
            ui.label(format!(
                "  name: {}",
                format_optional_str(fields.name.as_deref())
            ));
            ui.label(format!(
                "  id_pool: {}",
                format_optional_str(fields.id_pool.as_deref())
            ));
        }
        HyperMeshCommentFields::HWColor(fields) => {
            ui.label("HyperMesh (HWCOLOR):");
            ui.label(format!(
                "  entity_type: {}",
                format_optional_str(fields.entity_type.as_deref())
            ));
            ui.label(format!(
                "  id: {}",
                format_optional_str(fields.id.as_deref())
            ));
            ui.label(format!(
                "  color_id: {}",
                format_optional_str(fields.color_id.as_deref())
            ));
        }
        HyperMeshCommentFields::HMMove(fields) => {
            ui.label("HyperMesh (HMMOVE):");
            ui.label(format!(
                "  collector_id: {}",
                format_optional_str(fields.collector_id.as_deref())
            ));
        }
        HyperMeshCommentFields::HMDPrp(fields) => {
            ui.label("HyperMesh (HMDPRP):");
            ui.label(format!(
                "  elements: {}",
                format_optional_str(fields.elements.as_deref())
            ));
            ui.label(format!("  ids: {:?}", fields.ids));
        }
    }
}

fn format_optional_str(value: Option<&str>) -> &str {
    value.unwrap_or("-")
}

fn first_visual_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}

fn visual_item_prefix(item: &OptiStructItem) -> String {
    match item {
        OptiStructItem::Comment(comment) => format!("L{} [Comment]", comment.location.start_line),
        OptiStructItem::Text(text) => format!("L{} [Text]", text.location.start_line),
        OptiStructItem::Card(card) => {
            if card.location.start_line == card.location.end_line {
                format!("L{} [Card]", card.location.start_line)
            } else {
                format!(
                    "L{}-L{} [Card]",
                    card.location.start_line, card.location.end_line
                )
            }
        }
    }
}

fn aligned_prefix_cell(prefix: &str, width: usize) -> String {
    format!("{prefix:<width$}  ")
}

fn render_card_named_fields(ui: &mut egui::Ui, card: &femreader::model::OptiStructCardItem) {
    ui.label("Campos:");
    let names = card_field_names(&card.kind);
    if names.is_empty() && card.fields.is_empty() {
        ui.label("  (sin campos)");
        return;
    }
    let field_count = names.len().max(card.fields.len());
    for index in 0..field_count {
        let name = card_field_name(&card.kind, &names, &card.fields, index);
        let value = card_field_value(&card.fields, index);
        ui.label(format!("  {} = {}", name, value));
    }
    for name in missing_optional_card_fields(&card.kind) {
        ui.label(format!("  {} = null", name));
    }
}

fn missing_optional_card_fields(kind: &OptiStructCardKind) -> Vec<&'static str> {
    match kind {
        OptiStructCardKind::Rbe2(card) if card.alpha.is_none() => vec!["ALPHA"],
        OptiStructCardKind::Rbe3(card) if card.alpha.is_none() => vec!["ALPHA"],
        _ => Vec::new(),
    }
}

fn card_field_name(
    kind: &OptiStructCardKind,
    names: &[&str],
    fields: &[String],
    index: usize,
) -> String {
    if let Some(name) = names.get(index) {
        return (*name).to_string();
    }

    match kind {
        OptiStructCardKind::Rbe2(card) => {
            let last_non_empty = fields.iter().rposition(|value| !value.trim().is_empty());
            if card.alpha.is_some() && last_non_empty == Some(index) {
                "ALPHA".to_string()
            } else if fields
                .get(index)
                .is_some_and(|value| !value.trim().is_empty())
            {
                let gm_index = fields
                    .iter()
                    .skip(4)
                    .take(index.saturating_sub(3))
                    .filter(|value| value.parse::<i64>().is_ok())
                    .count();
                format!("GM{gm_index}")
            } else {
                "BLANK".to_string()
            }
        }
        OptiStructCardKind::Rbe3(card)
            if card.alpha.is_some() && index == 5 + card.remaining_fields.len() =>
        {
            "ALPHA".to_string()
        }
        OptiStructCardKind::Rbe3(card)
            if card.alpha.is_some() && index == 6 + card.remaining_fields.len() =>
        {
            "ALPHA_VALUE".to_string()
        }
        OptiStructCardKind::Rbe3(_) => format!("RBE3_DATA_{}", index - 4),
        _ => format!("EXTRA_F{}", index + 1),
    }
}

fn card_field_value(fields: &[String], index: usize) -> String {
    fields
        .get(index)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("null")
        .to_string()
}

fn card_field_names(kind: &OptiStructCardKind) -> Vec<&'static str> {
    match kind {
        OptiStructCardKind::Grid(_) => {
            vec!["CARD", "ID", "CP", "X1", "X2", "X3", "CD", "PS", "SEID"]
        }
        OptiStructCardKind::Cquad4(_) => vec![
            "CARD",
            "ID",
            "PID",
            "G1",
            "G2",
            "G3",
            "G4",
            "THETA_OR_MCID",
            "ZOFFS",
            "TFLAG",
            "T1",
            "T2",
            "T3",
            "T4",
        ],
        OptiStructCardKind::Ctria3(_) => vec![
            "CARD",
            "ID",
            "PID",
            "G1",
            "G2",
            "G3",
            "THETA_OR_MCID",
            "ZOFFS",
            "TFLAG",
            "T1",
            "T2",
            "T3",
        ],
        OptiStructCardKind::Force(_) => {
            vec!["CARD", "SID", "G", "CID", "F", "N1", "N2", "N3"]
        }
        OptiStructCardKind::Moment(_) => {
            vec!["CARD", "SID", "G", "CID", "M", "N1", "N2", "N3"]
        }
        OptiStructCardKind::Cbush(_) => vec![
            "CARD", "EID", "PID", "GA", "GB", "G0/X1", "X2", "X3", "CID", "S", "OCID", "S1", "S2",
            "S3",
        ],
        OptiStructCardKind::Pbush(_) => vec![
            "CARD", "PID", "K1", "K2", "K3", "K4", "K5", "K6", "B1", "B2", "B3", "B4", "B5", "B6",
            "GE1", "GE2", "GE3", "GE4", "GE5", "GE6", "SA", "SB",
        ],
        OptiStructCardKind::Pshell(_) => vec![
            "CARD", "PID", "MID1", "T", "MID2", "12I/T^3", "MID3", "TS/T", "NSM", "Z1", "Z2",
            "MID4",
        ],
        OptiStructCardKind::Mat1(_) => vec![
            "CARD", "MID", "E", "G", "NU", "RHO", "A", "TREF", "GE", "ST", "SC", "SS", "MCSID",
        ],
        OptiStructCardKind::Rbe2(_) => vec!["CARD", "EID", "GN", "CM"],
        OptiStructCardKind::Rbe3(_) => vec!["CARD", "EID", "BLANK", "REFGRID", "REFC"],
        OptiStructCardKind::Include(_) => vec!["CARD", "TARGET"],
        OptiStructCardKind::BeginBulk(_) => Vec::new(),
        OptiStructCardKind::EndData(_) => Vec::new(),
        OptiStructCardKind::Unknown(_) => vec!["CARD"],
    }
}

fn load_highlighter_for_solver(solver: &SolverKind) -> Option<SyntaxHighlighter> {
    let file_name = highlighting_file_for_solver(solver)?;
    if let Some(path) = locate_highlighting_file(file_name)
        && let Ok(highlighter) = SyntaxHighlighter::from_notepadpp_file(&path)
    {
        return Some(highlighter);
    }

    embedded_highlighting_xml(file_name)
        .and_then(|xml| SyntaxHighlighter::from_notepadpp_xml(xml).ok())
}

fn locate_highlighting_file(file_name: &str) -> Option<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir.join("highlighting").join(file_name));
    }
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        candidates.push(exe_dir.join("highlighting").join(file_name));
    }

    candidates.into_iter().find(|path| path.is_file())
}

fn resolve_visual_highlighter<'a>(
    highlight_enabled: bool,
    syntax_highlighting: &'a SolverSyntaxHighlighting,
    solver: &SolverKind,
) -> Option<&'a SyntaxHighlighter> {
    if !highlight_enabled {
        return None;
    }
    syntax_highlighting
        .for_solver(solver)
        .or_else(|| syntax_highlighting.for_solver(&SolverKind::OptiStruct))
}

fn render_highlighted_text_block(
    ui: &mut egui::Ui,
    title: &str,
    text: &str,
    highlighter: Option<&SyntaxHighlighter>,
    background: Option<Color32>,
    text_column_width: usize,
) {
    ui.label(title);
    let lines = visual_lines_for_block(text, text_column_width);
    if let Some(background_color) = background {
        egui::Frame::NONE
            .fill(background_color)
            .inner_margin(egui::Margin::same(0))
            .outer_margin(egui::Margin::same(0))
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for line in lines {
                    render_highlighted_line(ui, &line, highlighter, None);
                }
            });
    } else {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            for line in lines {
                render_highlighted_line(ui, &line, highlighter, background);
            }
        });
    }
}

fn build_highlighted_collapsing_label(
    ui: &egui::Ui,
    prefix: &str,
    line: &str,
    highlighter: Option<&SyntaxHighlighter>,
    background: Option<Color32>,
) -> LayoutJob {
    let default_color = ui.visuals().text_color();
    let token_default_color = default_text_color_for_highlighting(highlighter, default_color);
    let background_color = background.unwrap_or(Color32::TRANSPARENT);
    let mut job = LayoutJob::default();
    job.append(
        prefix,
        0.0,
        TextFormat {
            font_id: FontId::new(13.0, FontFamily::Monospace),
            color: default_color,
            background: Color32::TRANSPARENT,
            ..Default::default()
        },
    );

    if let Some(highlighter) = highlighter {
        append_highlighted_line_to_job(
            &mut job,
            line,
            highlighter,
            token_default_color,
            background_color,
        );
    } else {
        job.append(
            line,
            0.0,
            TextFormat {
                font_id: FontId::new(13.0, FontFamily::Monospace),
                color: default_color,
                background: background_color,
                ..Default::default()
            },
        );
    }

    job
}

fn render_highlighted_line(
    ui: &mut egui::Ui,
    line: &str,
    highlighter: Option<&SyntaxHighlighter>,
    background: Option<Color32>,
) {
    let default_color = ui.visuals().text_color();
    let token_default_color = default_text_color_for_highlighting(highlighter, default_color);
    let background_color = background.unwrap_or(Color32::TRANSPARENT);
    if let Some(highlighter) = highlighter {
        let mut job = LayoutJob::default();
        append_highlighted_line_to_job(
            &mut job,
            line,
            highlighter,
            token_default_color,
            background_color,
        );
        ui.label(job);
    } else {
        let mut rich = RichText::new(line).monospace();
        if background.is_some() {
            rich = rich.background_color(background_color);
        }
        ui.label(rich);
    }
}

fn append_highlighted_line_to_job(
    job: &mut LayoutJob,
    line: &str,
    highlighter: &SyntaxHighlighter,
    default_color: Color32,
    background_color: Color32,
) {
    for span in highlighter.highlight_line(line) {
        let color = match span.kind {
            HighlightKind::Default => default_color,
            HighlightKind::Comment => to_color32(highlighter.styles().comment),
            HighlightKind::Operator => to_color32(highlighter.styles().operator),
            HighlightKind::Keyword(group) => {
                let index = group.saturating_sub(1) as usize;
                highlighter
                    .styles()
                    .keywords
                    .get(index)
                    .copied()
                    .map(to_color32)
                    .unwrap_or(default_color)
            }
        };
        job.append(
            &span.text,
            0.0,
            TextFormat {
                font_id: FontId::new(13.0, FontFamily::Monospace),
                color,
                background: background_color,
                ..Default::default()
            },
        );
    }
}

fn to_color32(color: RgbColor) -> Color32 {
    Color32::from_rgb(color.r, color.g, color.b)
}

fn default_text_color_for_highlighting(
    highlighter: Option<&SyntaxHighlighter>,
    gui_default_color: Color32,
) -> Color32 {
    if highlighter.is_some() {
        Color32::BLACK
    } else {
        gui_default_color
    }
}

fn display_path_for_gui(path: &Path) -> String {
    strip_windows_extended_path_prefix(&path.display().to_string())
}

fn strip_windows_extended_path_prefix(path: &str) -> String {
    if let Some(rest) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{}", rest);
    }
    if let Some(rest) = path.strip_prefix(r"\\?\") {
        return rest.to_string();
    }
    path.to_string()
}

fn visual_text_column_width_chars(solver: &SolverKind, items: &[&OptiStructItem]) -> usize {
    let max_line_len = items
        .iter()
        .flat_map(|item| item_text(item).lines())
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0);
    text_column_width_from_max_line(solver, max_line_len)
}

fn text_column_width_from_max_line(solver: &SolverKind, max_line_len: usize) -> usize {
    solver_min_text_width_chars(solver).max(max_line_len)
}

fn solver_min_text_width_chars(solver: &SolverKind) -> usize {
    match solver {
        SolverKind::OptiStruct | SolverKind::Nastran | SolverKind::Unknown => 80,
        SolverKind::Abaqus => 0,
    }
}

fn item_text(item: &OptiStructItem) -> &str {
    match item {
        OptiStructItem::Comment(comment) => &comment.text,
        OptiStructItem::Text(text) => &text.text,
        OptiStructItem::Card(card) => &card.text,
    }
}

fn visual_lines_for_block(text: &str, text_column_width: usize) -> Vec<String> {
    let mut lines = text
        .lines()
        .map(|line| visual_line_with_fixed_width(line, text_column_width))
        .collect::<Vec<_>>();
    if text.is_empty() || text.ends_with('\n') {
        lines.push(visual_line_with_fixed_width("", text_column_width));
    }
    if lines.is_empty() {
        lines.push(visual_line_with_fixed_width("", text_column_width));
    }
    lines
}

fn visual_line_with_fixed_width(line: &str, text_column_width: usize) -> String {
    let line_chars = line.chars().count();
    if line_chars >= text_column_width {
        return line.to_string();
    }
    let mut value = String::with_capacity(text_column_width);
    value.push_str(line);
    value.push_str(&" ".repeat(text_column_width - line_chars));
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_card_fields_are_rendered_as_null() {
        let fields = vec![
            "CQUAD4".to_string(),
            "10".to_string(),
            "20".to_string(),
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
        ];

        assert_eq!(card_field_value(&fields, 7), "null");
        assert_eq!(card_field_value(&fields, 6), "4");
    }

    #[test]
    fn missing_rbe_alpha_values_are_rendered_as_null() {
        let rbe2 = OptiStructCardKind::Rbe2(femreader::model::Rbe2Card {
            eid: None,
            gn: None,
            cm: None,
            gm: Vec::new(),
            alpha: None,
        });
        let rbe3 = OptiStructCardKind::Rbe3(femreader::model::Rbe3Card {
            eid: None,
            refgrid: None,
            refc: None,
            remaining_fields: Vec::new(),
            alpha: None,
        });

        assert_eq!(missing_optional_card_fields(&rbe2), vec!["ALPHA"]);
        assert_eq!(missing_optional_card_fields(&rbe3), vec!["ALPHA"]);
    }

    #[test]
    fn aligned_prefix_cell_pads_prefix_to_fixed_width() {
        let padded = aligned_prefix_cell("L12 [Card]", 16);
        assert_eq!(padded, "L12 [Card]        ");
        assert_eq!(padded.chars().count(), 18);
    }

    #[test]
    fn aligned_prefix_cell_keeps_long_prefix_without_truncation() {
        let prefix = "L120-L130 [Card]";
        let padded = aligned_prefix_cell(prefix, 8);
        assert_eq!(padded, "L120-L130 [Card]  ");
    }

    #[test]
    fn highlight_background_choice_maps_expected_colors() {
        assert_eq!(HighlightBackgroundChoice::GuiDefault.color(), None);
        assert_eq!(
            HighlightBackgroundChoice::White.color(),
            Some(Color32::from_rgb(0xFF, 0xFF, 0xFF))
        );
        assert_eq!(
            HighlightBackgroundChoice::Cornsilk.color(),
            Some(Color32::from_rgb(0xFF, 0xF8, 0xDC))
        );
        assert_eq!(
            HighlightBackgroundChoice::Gray.color(),
            Some(Color32::from_rgb(0xA8, 0xA8, 0xA8))
        );
    }

    #[test]
    fn solver_min_text_width_for_nastran_and_optistruct_is_80() {
        assert_eq!(solver_min_text_width_chars(&SolverKind::OptiStruct), 80);
        assert_eq!(solver_min_text_width_chars(&SolverKind::Nastran), 80);
        assert_eq!(solver_min_text_width_chars(&SolverKind::Unknown), 80);
        assert_eq!(solver_min_text_width_chars(&SolverKind::Abaqus), 0);
    }

    #[test]
    fn text_column_width_respects_solver_minimum_and_long_lines() {
        assert_eq!(
            text_column_width_from_max_line(&SolverKind::OptiStruct, 12),
            80
        );
        assert_eq!(
            text_column_width_from_max_line(&SolverKind::Nastran, 95),
            95
        );
        assert_eq!(text_column_width_from_max_line(&SolverKind::Abaqus, 37), 37);
    }

    #[test]
    fn visual_line_with_fixed_width_pads_to_target_without_truncation() {
        assert_eq!(visual_line_with_fixed_width("GRID", 8), "GRID    ");
        assert_eq!(visual_line_with_fixed_width("BEGIN BULK", 4), "BEGIN BULK");
    }

    #[test]
    fn resolve_visual_highlighter_honors_enabled_flag() {
        let xml = r#"
<NotepadPlus>
  <UserLang name="X" ext="fem" udlVersion="2.1">
    <Settings><Global caseIgnored="yes" /></Settings>
    <KeywordLists>
      <Keywords name="Comments">00$$</Keywords>
      <Keywords name="Keywords1">GRID</Keywords>
    </KeywordLists>
    <Styles>
      <WordsStyle name="DEFAULT" fgColor="000000" />
      <WordsStyle name="LINE COMMENTS" fgColor="008000" />
      <WordsStyle name="KEYWORDS1" fgColor="0000FF" />
    </Styles>
  </UserLang>
</NotepadPlus>
"#;
        let highlighter =
            SyntaxHighlighter::from_notepadpp_xml(xml).expect("test highlighter xml should parse");
        let syntax = SolverSyntaxHighlighting {
            optistruct: Some(highlighter),
            nastran: None,
            abaqus: None,
        };

        assert!(resolve_visual_highlighter(true, &syntax, &SolverKind::OptiStruct).is_some());
        assert!(resolve_visual_highlighter(false, &syntax, &SolverKind::OptiStruct).is_none());
    }

    #[test]
    fn default_text_color_is_black_when_highlighting_is_active() {
        let xml = r#"
<NotepadPlus>
  <UserLang name="X" ext="fem" udlVersion="2.1">
    <Settings><Global caseIgnored="yes" /></Settings>
    <KeywordLists><Keywords name="Keywords1">GRID</Keywords></KeywordLists>
    <Styles>
      <WordsStyle name="DEFAULT" fgColor="000000" />
      <WordsStyle name="KEYWORDS1" fgColor="0000FF" />
    </Styles>
  </UserLang>
</NotepadPlus>
"#;
        let highlighter =
            SyntaxHighlighter::from_notepadpp_xml(xml).expect("test highlighter xml should parse");
        assert_eq!(
            default_text_color_for_highlighting(Some(&highlighter), Color32::WHITE),
            Color32::BLACK
        );
        assert_eq!(
            default_text_color_for_highlighting(None, Color32::WHITE),
            Color32::WHITE
        );
    }

    #[test]
    fn strip_windows_extended_path_prefix_removes_verbatim_prefix() {
        assert_eq!(
            strip_windows_extended_path_prefix(r"\\?\C:\models\main.fem"),
            r"C:\models\main.fem"
        );
        assert_eq!(
            strip_windows_extended_path_prefix(r"\\?\UNC\server\share\main.fem"),
            r"\\server\share\main.fem"
        );
        assert_eq!(
            strip_windows_extended_path_prefix(r"C:\models\main.fem"),
            r"C:\models\main.fem"
        );
    }

    #[test]
    fn new_gui_app_enables_delete_cache_by_default() {
        let app = FemReaderGuiApp::new(String::new());
        assert!(app.delete_cache_after_load);
    }
}
