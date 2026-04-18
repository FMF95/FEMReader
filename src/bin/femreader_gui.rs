use eframe::egui::{self, Color32, RichText};
use femreader::assembler::DefaultModelAssembler;
use femreader::cache::IncrementalCache;
use femreader::dependency::DefaultDependencyResolver;
use femreader::model::{Model, ModelContent, SolverKind};
use femreader::pipeline::ReaderPipeline;
use femreader::scanner::DefaultFileSystemScanner;
use femreader::serializer::JsonSerializer;
use femreader::solver::{AbaqusAdapter, NastranAdapter, OptiStructAdapter, SolverAdapter};
use std::collections::HashSet;
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
}

impl FemReaderGuiApp {
    fn new(input_directory: String) -> Self {
        Self {
            input_directory,
            content: None,
            selected_model_index: None,
            status_message: String::new(),
            error_message: None,
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

        match run_pipeline(&input) {
            Ok(content) => {
                self.status_message = format!(
                    "Cargado: {} modelos, {} archivos, {} dependencias.",
                    content.models.len(),
                    content.files.len(),
                    content.dependencies.len()
                );
                self.selected_model_index = if content.models.is_empty() { None } else { Some(0) };
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
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                for file_id in &model.file_ids {
                    ui.label(file_id);
                }
            });
        });

        let model_file_ids: HashSet<&str> = model.file_ids.iter().map(String::as_str).collect();

        ui.collapsing("Dependencias del modelo", |ui| {
            egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
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
                columns[0].label(format!("Dependencias: {}", content.dependencies.len()));
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
                columns[0].collapsing("Referencias sin resolver", |ui| {
                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
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

fn run_pipeline(root_directory: &Path) -> Result<ModelContent, String> {
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

    cache
        .save(&cache_path)
        .map_err(|e| format!("No se pudo guardar cache {}: {e}", cache_path.display()))?;

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
