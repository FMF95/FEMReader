use eframe::egui::{self, Color32, RichText};
use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
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
}

impl FemReaderGuiApp {
    fn new(input_directory: String) -> Self {
        Self {
            input_directory,
            content: None,
            selected_model_index: None,
            status_message: String::new(),
            error_message: None,
            delete_cache_after_load: false,
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
                input.display()
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

    fn render_model_details(ui: &mut egui::Ui, content: &ModelContent, model: &Model) {
        ui.heading(&model.name);
        ui.label(format!("ID: {}", model.id));
        if let Some(root_file) = Self::file_by_id(content, &model.root_file_id) {
            ui.label(format!("Archivo raíz: {}", root_file.name));
            ui.label(format!("Path raíz: {}", root_file.path.display()));
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
                        ui.label(format!("Path: {}", file.path.display()));
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
            Self::render_optistruct_structure(ui, content, model);
        });

        full_width_collapsing(ui, "Resumen OptiStruct del modelo", |ui| {
            Self::render_optistruct_model_summary(ui, content, model);
        });

        full_width_collapsing(ui, "Resumen OptiStruct por archivo", |ui| {
            Self::render_optistruct_file_summaries(ui, content, model);
        });
    }

    fn render_optistruct_structure(ui: &mut egui::Ui, content: &ModelContent, model: &Model) {
        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();
        let mut grouped = BTreeMap::<(String, String, String), Vec<&OptiStructItem>>::new();

        for item in &content.optistruct_items {
            let (file_id, file_name, file_path) = match item {
                OptiStructItem::Comment(comment) => (
                    comment.location.file_id.as_str(),
                    comment.location.file_name.clone(),
                    comment.location.file_path.display().to_string(),
                ),
                OptiStructItem::Text(text) => (
                    text.location.file_id.as_str(),
                    text.location.file_name.clone(),
                    text.location.file_path.display().to_string(),
                ),
                OptiStructItem::Card(card) => (
                    card.location.file_id.as_str(),
                    card.location.file_name.clone(),
                    card.location.file_path.display().to_string(),
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

            full_width_collapsing(ui, file_name, |ui| {
                ui.label(format!("Path: {}", file_path));
                ui.label(format!("ID: {}", file_id));
                for item in items {
                    match item {
                        OptiStructItem::Comment(comment) => {
                            full_width_collapsing(
                                ui,
                                format!(
                                    "L{} [Comment] {}",
                                    comment.location.start_line,
                                    first_visual_line(&comment.text)
                                ),
                                |ui| {
                                    ui.label(format!("ID: {}", comment.id));
                                    ui.label(format!("Texto: {}", comment.text));
                                    ui.label(format!(
                                        "Path: {}",
                                        comment.location.file_path.display()
                                    ));
                                    ui.label(format!("File ID: {}", comment.location.file_id));
                                    ui.label(format!("Ordinal: {}", comment.location.ordinal));
                                    if let OptiStructCommentKind::HyperMeshComment {
                                        fields, ..
                                    } = &comment.kind
                                    {
                                        render_hypermesh_fields(ui, fields);
                                    }
                                },
                            );
                        }
                        OptiStructItem::Text(text) => {
                            full_width_collapsing(
                                ui,
                                format!(
                                    "L{} [Text] {}",
                                    text.location.start_line,
                                    first_visual_line(&text.text)
                                ),
                                |ui| {
                                    ui.label(format!("ID: {}", text.id));
                                    ui.label(format!("Texto: {}", text.text));
                                    ui.label(format!(
                                        "Path: {}",
                                        text.location.file_path.display()
                                    ));
                                    ui.label(format!("File ID: {}", text.location.file_id));
                                    ui.label(format!("Ordinal: {}", text.location.ordinal));
                                },
                            );
                        }
                        OptiStructItem::Card(card) => {
                            let line_label = if card.location.start_line == card.location.end_line {
                                format!("L{}", card.location.start_line)
                            } else {
                                format!("L{}-L{}", card.location.start_line, card.location.end_line)
                            };
                            full_width_collapsing(
                                ui,
                                format!("{} [Card] {}", line_label, first_visual_line(&card.text)),
                                |ui| {
                                    ui.label(format!("ID: {}", card.id));
                                    ui.label(format!("Línea(s) original(es):\n{}", card.text));
                                    ui.label(format!("Tarjeta: {}", card.card_name));
                                    ui.label(format!("Ordinal: {}", card.location.ordinal));
                                    ui.label(format!(
                                        "Formato: {}",
                                        if card.long_format { "LONG" } else { "SHORT" }
                                    ));
                                    ui.label(format!(
                                        "Path: {}",
                                        card.location.file_path.display()
                                    ));
                                    ui.label(format!("File ID: {}", card.location.file_id));
                                    render_card_named_fields(ui, card);
                                },
                            );
                        }
                    }
                }
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
                ui.label(format!("Path: {}", summary.file_path.display()));
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
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Directorio de modelos:");
                let text_edit = ui.text_edit_singleline(&mut self.input_directory);
                let pressed_enter =
                    text_edit.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
                ui.checkbox(
                    &mut self.delete_cache_after_load,
                    "Eliminar cache tras leer (.femreader-cache)",
                );

                if ui.button("Cargar").clicked() || pressed_enter {
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
                    ui.label(format!("Root: {}", content.root_directory.display()));
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
                                            ui.label(format!("Path: {}", file.path.display()));
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
                                Self::render_model_details(ui, content, model);
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

fn render_card_named_fields(ui: &mut egui::Ui, card: &femreader::model::OptiStructCardItem) {
    ui.label("Campos:");
    if card.fields.is_empty() {
        ui.label("  (sin campos)");
        return;
    }
    let names = card_field_names(&card.kind);
    for (index, value) in card.fields.iter().enumerate() {
        if let Some(name) = names.get(index).copied() {
            ui.label(format!("  {} = {}", name, value));
        } else {
            ui.label(format!("  EXTRA_F{} = {}", index + 1, value));
        }
    }
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
        OptiStructCardKind::Include(_) => vec!["CARD", "TARGET"],
        OptiStructCardKind::BeginBulk(_) => Vec::new(),
        OptiStructCardKind::EndData(_) => Vec::new(),
        OptiStructCardKind::Unknown(_) => vec!["CARD"],
    }
}
