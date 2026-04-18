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
                                    "L{} [{}]",
                                    comment.location.start_line,
                                    comment_kind_label(&comment.kind)
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
                        OptiStructItem::Card(card) => {
                            let line_label = if card.location.start_line == card.location.end_line {
                                format!("L{}", card.location.start_line)
                            } else {
                                format!("L{}-L{}", card.location.start_line, card.location.end_line)
                            };
                            full_width_collapsing(
                                ui,
                                format!("{} {}", line_label, card_kind_summary(&card.kind)),
                                |ui| {
                                    ui.label(format!("ID: {}", card.id));
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
                                    ui.label("Campos:");
                                    for (index, value) in card.fields.iter().enumerate() {
                                        ui.label(format!("  F{} = {}", index + 1, value));
                                    }
                                },
                            );
                        }
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

fn card_kind_summary(kind: &OptiStructCardKind) -> String {
    match kind {
        OptiStructCardKind::Grid(card) => format!(
            "GRID id={:?} xyz=({:?}, {:?}, {:?})",
            card.id, card.x1, card.x2, card.x3
        ),
        OptiStructCardKind::Cquad4(card) => format!(
            "CQUAD4 id={:?} pid={:?} g=({:?},{:?},{:?},{:?})",
            card.id, card.pid, card.g1, card.g2, card.g3, card.g4
        ),
        OptiStructCardKind::Ctria3(card) => format!(
            "CTRIA3 id={:?} pid={:?} g=({:?},{:?},{:?})",
            card.id, card.pid, card.g1, card.g2, card.g3
        ),
        OptiStructCardKind::Force(card) => format!(
            "FORCE sid={:?} g={:?} f={:?} n=({:?},{:?},{:?})",
            card.sid, card.g, card.f, card.n1, card.n2, card.n3
        ),
        OptiStructCardKind::Moment(card) => format!(
            "MOMENT sid={:?} g={:?} m={:?} n=({:?},{:?},{:?})",
            card.sid, card.g, card.m, card.n1, card.n2, card.n3
        ),
        OptiStructCardKind::Include(card) => format!("INCLUDE target={}", card.target),
        OptiStructCardKind::BeginBulk(_) => "BEGIN BULK".to_string(),
        OptiStructCardKind::EndData(_) => "ENDDATA".to_string(),
        OptiStructCardKind::Unknown(card) => format!("{} (sin parser específico)", card.card_name),
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
            ui.label(format!("  entity_type: {:?}", fields.entity_type));
            ui.label(format!("  id: {:?}", fields.id));
            ui.label(format!("  name: {:?}", fields.name));
            ui.label(format!("  id_pool: {:?}", fields.id_pool));
        }
        HyperMeshCommentFields::HWColor(fields) => {
            ui.label("HyperMesh (HWCOLOR):");
            ui.label(format!("  entity_type: {:?}", fields.entity_type));
            ui.label(format!("  id: {:?}", fields.id));
            ui.label(format!("  color_id: {:?}", fields.color_id));
        }
        HyperMeshCommentFields::HMMove(fields) => {
            ui.label("HyperMesh (HMMOVE):");
            ui.label(format!("  collector_id: {:?}", fields.collector_id));
        }
        HyperMeshCommentFields::HMDPrp(fields) => {
            ui.label("HyperMesh (HMDPRP):");
            ui.label(format!("  elements: {:?}", fields.elements));
            ui.label(format!("  ids: {:?}", fields.ids));
        }
    }
}

fn comment_kind_label(kind: &OptiStructCommentKind) -> &'static str {
    match kind {
        OptiStructCommentKind::Comment => "Comment",
        OptiStructCommentKind::HyperMeshComment { .. } => "HyperMesh Comment",
    }
}
