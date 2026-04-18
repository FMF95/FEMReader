use eframe::egui::{self, Color32, RichText};
use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
use femreader::model::{
    FileCategory, Model, ModelContent, OptiStructCardKind, OptiStructItem, SolverKind,
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
        ui.label(format!("Archivo raíz: {}", model.root_file_id));
        ui.label(format!("Solver: {}", solver_label(&model.solver)));
        ui.label(format!("Archivos en el modelo: {}", model.file_ids.len()));
        ui.separator();

        ui.collapsing("Archivos del modelo", |ui| {
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for file_id in &model.file_ids {
                        ui.label(file_id);
                    }
                });
        });

        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();

        ui.collapsing("Dependencias del modelo", |ui| {
            egui::ScrollArea::vertical()
                .max_height(220.0)
                .show(ui, |ui| {
                    for edge in &content.dependencies {
                        if model_file_ids.contains(edge.from_file_id.as_str())
                            || model_file_ids.contains(edge.to_file_id.as_str())
                        {
                            ui.label(format!(
                                "{} -> {} ({})",
                                edge.from_file_id, edge.to_file_id, edge.reference
                            ));
                        }
                    }
                });
        });

        ui.collapsing("Estructura OptiStruct (visual)", |ui| {
            Self::render_optistruct_structure(ui, content, model);
        });
    }

    fn render_optistruct_structure(ui: &mut egui::Ui, content: &ModelContent, model: &Model) {
        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();
        let mut grouped = BTreeMap::<String, Vec<&OptiStructItem>>::new();

        for item in &content.optistruct_items {
            let (file_id, file_path) = match item {
                OptiStructItem::Comment(comment) => (
                    comment.location.file_id.as_str(),
                    comment.location.file_path.display().to_string(),
                ),
                OptiStructItem::Card(card) => (
                    card.location.file_id.as_str(),
                    card.location.file_path.display().to_string(),
                ),
            };

            if model_file_ids.contains(file_id) {
                grouped.entry(file_path).or_default().push(item);
            }
        }

        if grouped.is_empty() {
            ui.label("No hay tarjetas/comentarios OptiStruct para este modelo.");
            return;
        }

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for (file_path, mut items) in grouped {
                    items.sort_by_key(|item| match item {
                        OptiStructItem::Comment(comment) => comment.location.ordinal,
                        OptiStructItem::Card(card) => card.location.ordinal,
                    });

                    ui.collapsing(file_path, |ui| {
                        for item in items {
                            match item {
                                OptiStructItem::Comment(comment) => {
                                    ui.label(format!(
                                        "L{} [COMMENT] {}",
                                        comment.location.start_line, comment.text
                                    ));
                                }
                                OptiStructItem::Card(card) => {
                                    let line_label =
                                        if card.location.start_line == card.location.end_line {
                                            format!("L{}", card.location.start_line)
                                        } else {
                                            format!(
                                                "L{}-L{}",
                                                card.location.start_line, card.location.end_line
                                            )
                                        };
                                    ui.collapsing(
                                        format!(
                                            "{} [{}] {}",
                                            line_label,
                                            if card.long_format { "LONG" } else { "SHORT" },
                                            card_kind_summary(&card.kind)
                                        ),
                                        |ui| {
                                            ui.label(format!("Tarjeta: {}", card.card_name));
                                            ui.label(format!("Ordinal: {}", card.location.ordinal));
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
            });
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

            ui.columns(2, |columns| {
                columns[0].heading("Resumen");
                columns[0].label(format!("Root: {}", content.root_directory.display()));
                columns[0].label(format!("Directorios: {}", content.directories.len()));
                columns[0].label(format!("Archivos: {}", content.files.len()));
                let model_files = content
                    .files
                    .iter()
                    .filter(|file| file.category == FileCategory::Model)
                    .count();
                columns[0].label(format!("Archivos de modelo: {}", model_files));
                columns[0].label(format!(
                    "Archivos no modelo: {}",
                    content.files.len().saturating_sub(model_files)
                ));
                columns[0].label(format!("Dependencias: {}", content.dependencies.len()));
                columns[0].label(format!(
                    "Items OptiStruct: {}",
                    content.optistruct_items.len()
                ));
                columns[0].label(format!(
                    "Referencias sin resolver: {}",
                    content.unresolved_references.len()
                ));
                columns[0].separator();
                columns[0].heading("Modelos");

                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .show(&mut columns[0], |ui| {
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

                columns[0].separator();
                columns[0].collapsing("Archivos no modelo", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(160.0)
                        .show(ui, |ui| {
                            for file in &content.files {
                                if file.category == FileCategory::NonModel {
                                    ui.label(file.path.display().to_string());
                                }
                            }
                        });
                });

                columns[0].separator();
                columns[0].collapsing("Referencias sin resolver", |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for unresolved in &content.unresolved_references {
                                ui.label(unresolved);
                            }
                        });
                });

                columns[1].heading("Detalle del modelo");
                if let Some(model_index) = selected_model_index {
                    if let Some(model) = content.models.get(model_index) {
                        Self::render_model_details(&mut columns[1], content, model);
                    } else {
                        columns[1].label("No hay modelo seleccionado.");
                    }
                } else {
                    columns[1].label("No hay modelo seleccionado.");
                }
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
        OptiStructCardKind::Unknown(card) => format!("{} (sin parser específico)", card.card_name),
    }
}
