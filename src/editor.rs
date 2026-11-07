//! Éditeur natif (egui/eframe, rendu GPU OpenGL/glow) — 100% natif, sans technologie web.
//! Fonctions : ouverture .c3p, import universel (drag & drop + dialogue),
//! arborescence projet, édition de layout, feuilles d'événements, aperçu jouable.

use crate::assets::{self, Asset, AssetKind};
use crate::c3p;
use crate::model::*;
use crate::runtime::{Engine, InputState};
use crate::save::save_c3p;
use eframe::egui;
use egui::{Align, Color32, FontData, FontDefinitions, Pos2, TextureOptions, Vec2};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// palette sombre originale (pas d'assets tiers)
pub const COL_BG: Color32 = Color32::from_rgb(0x1B, 0x1C, 0x1F);
pub const COL_PANEL: Color32 = Color32::from_rgb(0x25, 0x27, 0x2B);
pub const COL_RAISED: Color32 = Color32::from_rgb(0x2E, 0x31, 0x36);
pub const COL_ACCENT: Color32 = Color32::from_rgb(0xE8, 0x8C, 0x2E);
pub const COL_SEL: Color32 = Color32::from_rgb(0x3A, 0x6E, 0xA5);
pub const COL_TEXT: Color32 = Color32::from_rgb(0xD8, 0xDA, 0xDC);
pub const COL_DIM: Color32 = Color32::from_rgb(0x8B, 0x90, 0x96);
pub const COL_COND: Color32 = Color32::from_rgb(0x2F, 0x3D, 0x4C);
pub const COL_ACT: Color32 = Color32::from_rgb(0x2F, 0x45, 0x38);
pub const COL_GRID: Color32 = Color32::from_rgb(0x2A, 0x2D, 0x32);
pub const COL_OK: Color32 = Color32::from_rgb(0x7A, 0xC7, 0x6E);
pub const COL_WARN: Color32 = Color32::from_rgb(0xE0, 0xB0, 0x50);
pub const COL_ERR: Color32 = Color32::from_rgb(0xE0, 0x6C, 0x5A);

#[derive(Clone, PartialEq)]
pub enum Tab {
    Layout(usize),
    Events(usize),
    Media,
    Preview,
}

#[derive(Clone, PartialEq)]
pub enum Tool {
    Select,
    Place(String),
}

pub struct EditorApp {
    pub project: Option<Project>,
    pub path: Option<PathBuf>,
    pub warnings: Vec<String>,
    pub media: Vec<Asset>,
    pub tex: HashMap<String, egui::TextureHandle>,
    pub views: HashMap<usize, (f32, Pos2)>,
    pub sel: Option<usize>,
    pub tool: Tool,
    pub tab: Tab,
    pub preview: Option<Engine>,
    pub preview_tex: Option<egui::TextureHandle>,
    pub prev_down: HashSet<u32>,
    pub prev_mouse: bool,
    pub fonts: FontDefinitions,
    pub font_dirty: bool,
    pub log: Vec<(u8, String)>, // 0=info 1=warn 2=err
    pub show_console: bool,
    pub path_buf: String,
    pub show_pathwin: bool,
    pub pathwin_save: bool,
    pub open_buf: String,
    pub show_open: bool,
    pub dirty: bool,
    pub next_uid: usize,
    pub place_layer: usize,
}

fn unique_name(existing: &[String], base: &str) -> String {
    if !existing.iter().any(|e| e.eq_ignore_ascii_case(base)) {
        return base.to_string();
    }
    for i in 2..9999 {
        let c = format!("{base} {i}");
        if !existing.iter().any(|e| e.eq_ignore_ascii_case(&c)) {
            return c;
        }
    }
    format!("{base} {}", existing.len() + 1)
}

impl EditorApp {
    pub fn new(cc: &eframe::CreationContext) -> Self {
        let mut fonts = FontDefinitions::default();
        if let Some(dir) = std::env::var_os("C3RS_FONT") {
            if let Some(b) = std::fs::read(dir).ok() {
                fonts.font_data.insert("import0".into(), FontData::from_owned(b));
                fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, "import0".into());
            }
        }
        let ctx = &cc.egui_ctx;
        ctx.set_visuals(egui::Visuals::dark());
        ctx.style_mut(|s| {
            s.visuals.panel_fill = COL_PANEL;
            s.visuals.window_fill = COL_RAISED;
            s.visuals.extreme_bg_color = COL_BG;
            s.visuals.faint_bg_color = COL_RAISED;
            s.visuals.selection.bg_fill = COL_SEL;
            s.visuals.selection.stroke = egui::Stroke::new(1.0, COL_ACCENT);
            s.spacing.item_spacing = Vec2::new(8.0, 4.0);
            s.visuals.widgets.hovered.bg_fill = COL_RAISED;
            s.visuals.widgets.active.bg_fill = COL_SEL;
            s.visuals.widgets.inactive.bg_fill = COL_RAISED;
            s.visuals.widgets.noninteractive.bg_fill = COL_PANEL;
            s.visuals.override_text_color = Some(COL_TEXT);
        });
        Self {
            project: None,
            path: None,
            warnings: Vec::new(),
            media: Vec::new(),
            tex: HashMap::new(),
            views: HashMap::new(),
            sel: None,
            tool: Tool::Select,
            tab: Tab::Layout(0),
            preview: None,
            preview_tex: None,
            prev_down: HashSet::new(),
            prev_mouse: false,
            fonts,
            font_dirty: false,
            log: vec![(0, "c3rs pret. Ouvrez un .c3p ou importez n'importe quel fichier (glisser-deposer bienvenu).".into())],
            show_console: false,
            path_buf: String::new(),
            show_pathwin: false,
            pathwin_save: false,
            open_buf: String::new(),
            show_open: false,
            dirty: false,
            next_uid: 1000,
            place_layer: 0,
        }
    }

    pub fn launch(path: Option<PathBuf>) -> Result<(), String> {
        let opts = eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1440.0, 880.0])
                .with_min_inner_size([960.0, 600.0])
                .with_title("c3rs — Construct Rust Edition")
                .with_drag_and_drop(true),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        };
        eframe::run_native(
            "c3rs — Construct Rust Edition",
            opts,
            Box::new(move |cc| {
                let mut app = EditorApp::new(cc);
                if let Some(p) = path {
                    app.open_project_path(&p);
                }
                Ok(Box::new(app))
            }),
        )
        .map_err(|e| format!("eframe : {e}"))
    }

    pub(crate) fn say(&mut self, level: u8, msg: impl Into<String>) {
        let m = msg.into();
        self.log.push((level, m));
        if self.log.len() > 400 {
            self.log.remove(0);
        }
    }

    // ---------------- projet ----------------

    pub fn new_project(&mut self) {
        let sheet = "Feuille 1".to_string();
        let layout = Layout {
            name: "Layout 1".into(),
            width: 2048.0,
            height: 2048.0,
            background: Some([0.10, 0.11, 0.13, 1.0]),
            layers: vec![Layer {
                name: "Calque 0".into(),
                parallax: (100.0, 100.0),
                visible: true,
                opacity: 1.0,
                instances: vec![],
            }],
            event_sheet: Some(sheet.clone()),
        };
        self.project = Some(Project {
            name: "Nouveau projet".into(),
            viewport_w: 1280.0,
            viewport_h: 720.0,
            first_layout: "Layout 1".into(),
            background: [0.10, 0.11, 0.13, 1.0],
            layouts: vec![layout],
            sheets: vec![Sheet { name: sheet, events: vec![] }],
            types: vec![],
            images: HashMap::new(),
        });
        self.path = None;
        self.warnings.clear();
        self.tex.clear();
        self.views.clear();
        self.sel = None;
        self.tool = Tool::Select;
        self.tab = Tab::Layout(0);
        self.dirty = false;
        self.say(0, "nouveau projet cree (1280x720, Layout 1 + Feuille 1)");
    }

    pub fn open_project_path(&mut self, p: &std::path::Path) {
        match std::fs::read(p) {
            Ok(bytes) => {
                let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                self.open_project_bytes(&name, bytes);
            }
            Err(e) => self.say(2, format!("lecture impossible {} : {e}", p.display())),
        }
    }

    pub fn open_project_bytes(&mut self, name: &str, bytes: Vec<u8>) -> bool {
        match c3p::load_bytes(&bytes) {
            Ok((proj, warns)) => {
                let n_layouts = proj.layouts.len();
                let n_inst: usize = proj.layouts.iter().map(|l| l.layers.iter().map(|la| la.instances.len()).sum::<usize>()).sum();
                self.warnings = warns.clone();
                for w in &warns {
                    self.say(1, format!("chargement : {w}"));
                }
                self.project = Some(proj);
                self.path = None;
                self.preview = None;
                self.preview_tex = None;
                self.tex.clear();
                self.views.clear();
                self.sel = None;
                self.tool = Tool::Select;
                self.tab = Tab::Layout(0);
                self.dirty = false;
                self.say(0, format!("projet « {name} » ouvert : {n_layouts} layout(s), {n_inst} instance(s)"));
                true
            }
            Err(e) => {
                self.say(2, format!("« {name} » n'est pas un projet valide : {e}"));
                false
            }
        }
    }

    pub fn save_project(&mut self, p: &std::path::Path) {
        let Some(proj) = self.project.clone() else { return };
        match save_c3p(&proj, p) {
            Ok(()) => {
                self.path = Some(p.to_path_buf());
                self.dirty = false;
                self.say(0, format!("projet enregistre : {}", p.display()));
            }
            Err(e) => self.say(2, format!("enregistrement impossible : {e}")),
        }
    }

    pub fn add_layout(&mut self) {
        let Some(p) = self.project.as_mut() else { return };
        let names: Vec<String> = p.layouts.iter().map(|l| l.name.clone()).collect();
        let name = unique_name(&names, "Layout");
        p.layouts.push(Layout {
            name: name.clone(),
            width: 2048.0,
            height: 2048.0,
            background: None,
            layers: vec![Layer {
                name: "Calque 0".into(),
                parallax: (100.0, 100.0),
                visible: true,
                opacity: 1.0,
                instances: vec![],
            }],
            event_sheet: None,
        });
        let i = p.layouts.len() - 1;
        self.tab = Tab::Layout(i);
        self.dirty = true;
        self.say(0, format!("layout « {name} » ajoute"));
    }

    pub fn add_sheet(&mut self) {
        let Some(p) = self.project.as_mut() else { return };
        let names: Vec<String> = p.sheets.iter().map(|s| s.name.clone()).collect();
        let name = unique_name(&names, "Feuille");
        p.sheets.push(Sheet { name: name.clone(), events: vec![] });
        let i = p.sheets.len() - 1;
        self.tab = Tab::Events(i);
        self.dirty = true;
        self.say(0, format!("feuille d'evenements « {name} » ajoutee"));
    }

    pub fn add_object_from_image(&mut self, stem: &str, tex: Tex) {
        let Some(p) = self.project.as_mut() else { return };
        let key = assets::image_key(stem);
        let mut k = key.clone();
        let mut n = 2;
        while p.images.contains_key(&k) {
            k = format!("{key}-{n}");
            n += 1;
        }
        p.images.insert(k.clone(), tex.clone());
        let names: Vec<String> = p.types.iter().map(|t| t.name.clone()).collect();
        let base: String = stem.chars().take(40).collect();
        let name = unique_name(&names, &base);
        p.types.push(ObjType {
            name: name.clone(),
            plugin: Plugin::Sprite,
            is_family: false,
            members: vec![],
            vars: vec![],
            behaviors: vec![],
            anims: vec![Anim { name: "Default".into(), frames: vec![crate::model::FrameSpec::simple(k.clone())], speed: 0.0, looping: false }],
            is_global: false,
        });
        self.dirty = true;
        self.say(0, format!("objet « {name} » cree depuis l'image importee ({}x{})", tex.w, tex.h));
    }

    // ---------------- import universel ----------------

    pub fn import_paths(&mut self, paths: &[PathBuf]) {
        if paths.is_empty() {
            self.pathwin_save = false;
            self.show_pathwin = true;
            return;
        }
        for p in paths {
            match std::fs::read(p) {
                Ok(bytes) => {
                    let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                    self.import_bytes(name, bytes);
                }
                Err(e) => self.say(2, format!("{} : {e}", p.display())),
            }
        }
    }

    /// Importe n'importe quel fichier : projet, image, audio, police, donnees…
    pub fn import_bytes(&mut self, name: String, bytes: Vec<u8>) {
        let lower = name.to_lowercase();
        // 1) projet c3p ?
        if lower.ends_with(".c3p") || (lower.ends_with(".zip") && c3p::load_bytes(&bytes).is_ok()) {
            if self.open_project_bytes(&name, bytes.clone()) {
                return;
            }
            if lower.ends_with(".c3p") {
                return;
            }
        }
        let kind = assets::classify(&name);
        match kind {
            AssetKind::Image => match assets::decode_image(&bytes) {
                Ok(tex) => {
                    let stem = name.rsplit('.').nth(1).unwrap_or(&name).to_string();
                    let stem = if stem.is_empty() { name.clone() } else { stem };
                    self.add_object_from_image(&stem, tex.clone());
                    self.media.push(Asset { name: name.clone(), kind, bytes, tex: Some(tex), tex_key: None });
                }
                Err(e) => self.say(2, format!("{name} : {e}")),
            },
            AssetKind::Font => {
                let stem = name.rsplit('.').nth(1).unwrap_or(&name).to_string();
                let id = format!("font-{}", assets::image_key(&stem));
                self.fonts.font_data.insert(id.clone(), FontData::from_owned(bytes.clone()));
                self.fonts.families.entry(egui::FontFamily::Proportional).or_default().insert(0, id);
                self.font_dirty = true;
                self.media.push(Asset { name: name.clone(), kind, bytes, tex: None, tex_key: None });
                self.say(0, format!("police « {name} » importee et appliquee a l'interface"));
            }
            AssetKind::Audio | AssetKind::Archive | AssetKind::Data | AssetKind::Other => {
                if lower.ends_with(".zip") {
                    if let Ok(mut rd) = zip::ZipArchive::new(std::io::Cursor::new(bytes.clone())) {
                        let n = rd.len();
                        let names: Vec<String> = (0..n)
                            .filter_map(|i| rd.by_index(i).ok().map(|f| f.name().to_string()))
                            .take(10)
                            .collect();
                        self.say(0, format!("archive « {name} » : {n} entree(s), ex : {}", names.join(", ")));
                    }
                }
                self.media.push(Asset { name: name.clone(), kind, bytes, tex: None, tex_key: None });
                self.say(0, format!("media « {name} » ({}) ajoute a la bibliotheque", kind.label()));
            }
        }
        self.dirty = true;
    }

    // ---------------- apercu ----------------

    pub fn run_preview(&mut self) {
        if self.preview.is_some() {
            self.tab = Tab::Preview;
            return;
        }
        let Some(proj) = self.project.clone() else {
            self.say(1, "aucun projet ouvert");
            return;
        };
        match Engine::new(proj, "") {
            Ok(mut eng) => {
                if eng.view_w < 320.0 || eng.view_h < 240.0 {
                    eng.view_w = 1280.0;
                    eng.view_h = 720.0;
                }
                self.preview = Some(eng);
                self.preview_tex = None;
                self.tab = Tab::Preview;
                self.say(0, "apercu lance (F5 ou Echap pour arreter)");
            }
            Err(e) => self.say(2, format!("apercu impossible : {e}")),
        }
    }

    pub fn stop_preview(&mut self) {
        let warns = self.preview.as_ref().map(|e| e.warnings.clone()).unwrap_or_default();
        for w in warns {
            self.say(1, format!("runtime : {w}"));
        }
        self.preview = None;
        self.prev_down.clear();
        if let Tab::Preview = self.tab {
            self.tab = Tab::Layout(0);
        }
        self.say(0, "apercu arrete");
    }

    pub(crate) fn key_to_js(k: egui::Key) -> Option<u32> {
        use egui::Key::*;
        Some(match k {
            ArrowLeft => 37,
            ArrowUp => 38,
            ArrowRight => 39,
            ArrowDown => 40,
            Space => 32,
            Enter => 13,
            Escape => 27,
            A => 65,
            B => 66,
            C => 67,
            D => 68,
            E => 69,
            F => 70,
            G => 71,
            H => 72,
            I => 73,
            J => 74,
            K => 75,
            L => 76,
            M => 77,
            N => 78,
            O => 79,
            P => 80,
            Q => 81,
            R => 82,
            S => 83,
            T => 84,
            U => 85,
            V => 86,
            W => 87,
            X => 88,
            Y => 89,
            Z => 90,
            Num0 => 48,
            Num1 => 49,
            Num2 => 50,
            Num3 => 51,
            Num4 => 52,
            Num5 => 53,
            Num6 => 54,
            Num7 => 55,
            Num8 => 56,
            Num9 => 57,
            _ => return None,
        })
    }

    pub(crate) fn build_preview_input(&mut self, ctx: &egui::Context) {
        let Some(eng) = self.preview.as_mut() else { return };
        let (now, mouse_pos, mouse_down) = ctx.input(|i| {
            let mut now: HashSet<u32> = HashSet::new();
            for k in &i.keys_down {
                if let Some(c) = Self::key_to_js(*k) {
                    now.insert(c);
                }
            }
            if i.modifiers.shift {
                now.insert(16);
            }
            if i.modifiers.ctrl {
                now.insert(17);
            }
            let mp = i.pointer.latest_pos().unwrap_or_default();
            let md = i.pointer.primary_down();
            (now, mp, md)
        });
        let pressed: HashSet<u32> = now.difference(&self.prev_down).copied().collect();
        let mut mp_down = HashSet::new();
        if mouse_down {
            mp_down.insert(0u8);
        }
        let mouse_pressed: HashSet<u8> =
            if mouse_down && !self.prev_mouse { mp_down.clone() } else { HashSet::new() };
        eng.input = InputState {
            down: now.clone(),
            pressed,
            released: HashSet::new(),
            mouse: (mouse_pos.x, mouse_pos.y),
            mouse_down: mp_down,
            mouse_pressed,
        };
        self.prev_down = now;
        self.prev_mouse = mouse_down;
    }

    pub(crate) fn load_tex(&mut self, ctx: &egui::Context, key: &str) -> Option<egui::TextureHandle> {
        if let Some(t) = self.tex.get(key) {
            return Some(t.clone());
        }
        let proj = self.project.as_ref()?;
        let t = proj.images.get(key)?;
        let img = egui::ColorImage::from_rgba_unmultiplied([t.w.max(1) as usize, t.h.max(1) as usize], &t.rgba);
        let handle = ctx.load_texture(format!("img:{key}"), img, TextureOptions::LINEAR);
        self.tex.insert(key.to_string(), handle.clone());
        Some(handle)
    }

    pub(crate) fn apply_fonts(&mut self, ctx: &egui::Context) {
        if self.font_dirty {
            ctx.set_fonts(self.fonts.clone());
            self.font_dirty = false;
        }
    }

    // ---------------- panneaux ----------------

    fn top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("Fichier", |ui| {
                    if ui.button("Nouveau projet").clicked() {
                        self.new_project();
                        ui.close_menu();
                    }
                    if ui.button("Ouvrir un .c3p…").clicked() {
                        let paths = assets::native_pick_files();
                        if paths.is_empty() {
                            self.show_open = true;
                        } else {
                            for p in &paths {
                                self.open_project_path(p);
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("Importer n'importe quel fichier…").clicked() {
                        let paths = assets::native_pick_files();
                        if paths.is_empty() {
                            self.pathwin_save = false;
                            self.show_pathwin = true;
                        } else {
                            self.import_paths(&paths);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Enregistrer le projet (.c3p)…").clicked() {
                        if let Some(p) = self.path.clone() {
                            self.save_project(&p);
                        } else {
                            self.pathwin_save = true;
                            self.path_buf =
                                format!("{}.c3p", self.project.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "projet".into()));
                            self.show_pathwin = true;
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quitter").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("Affichage", |ui| {
                    if ui.checkbox(&mut self.show_console, "Console").changed() {
                        ui.close_menu();
                    }
                    if ui.button("Reinitialiser les zooms").clicked() {
                        self.views.clear();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Aide", |ui| {
                    if ui.button("Controles et raccourcis").clicked() {
                        ui.close_menu();
                        self.say(0, "F5 : lancer/arreter l'apercu | Glisser-deposer : importer tout type de fichier");
                        self.say(0, "Layout : molette = zoom, clic-milieu/droit = deplacer la vue, clic = selectionner, Suppr = supprimer");
                        self.say(0, "Selectionnez un objet dans l'arborescence puis cliquez dans le layout pour le placer.");
                    }
                    if ui.button("A propos").clicked() {
                        ui.close_menu();
                        self.say(0, "c3rs v0.2 — runtime et editeur natifs en Rust pour projets Construct 3 (.c3p). Projet original, non affilie a Scirra.");
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    let label = match &self.tab {
                        Tab::Preview => "APERCU".to_string(),
                        Tab::Layout(i) => {
                            self.project.as_ref().and_then(|p| p.layouts.get(*i)).map(|l| l.name.clone()).unwrap_or_default()
                        }
                        Tab::Events(i) => {
                            self.project.as_ref().and_then(|p| p.sheets.get(*i)).map(|s| s.name.clone()).unwrap_or_default()
                        }
                        Tab::Media => "Medias".into(),
                    };
                    ui.colored_label(COL_ACCENT, label);
                    if self.dirty {
                        ui.colored_label(COL_DIM, "modifie");
                    }
                    if let Some(p) = &self.project {
                        ui.label(&p.name);
                    }
                });
            });
        });
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui.button("Ouvrir .c3p").clicked() {
                    let paths = assets::native_pick_files();
                    if paths.is_empty() {
                        self.show_open = true;
                    } else {
                        for p in &paths {
                            self.open_project_path(p);
                        }
                    }
                }
                if ui.button("Importer…").clicked() {
                    let paths = assets::native_pick_files();
                    if paths.is_empty() {
                        self.pathwin_save = false;
                        self.show_pathwin = true;
                    } else {
                        self.import_paths(&paths);
                    }
                }
                ui.separator();
                let running = self.preview.is_some();
                if ui.add(egui::Button::new(if running { "Recharger" } else { "Lancer (F5)" })).clicked() {
                    self.preview = None;
                    self.run_preview();
                }
                if ui.add_enabled(running, egui::Button::new("Arreter")).clicked() {
                    self.stop_preview();
                }
                ui.separator();
                let tool_txt = match &self.tool {
                    Tool::Select => "Outil : selection".to_string(),
                    Tool::Place(n) => format!("Outil : placer « {n} » (clic dans le layout, Echap pour annuler)"),
                };
                if ui.button(tool_txt).clicked() {
                    self.tool = Tool::Select;
                }
            });
            ui.add_space(2.0);
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        // instantane pour l'affichage (emprunts rendus avant l'UI)
        struct Snap {
            name: String,
            layouts: Vec<(String, usize)>,
            sheets: Vec<String>,
            types: Vec<(String, &'static str, bool)>,
        }
        let snap = self.project.as_ref().map(|p| Snap {
            name: p.name.clone(),
            layouts: p.layouts.iter().map(|l| (l.name.clone(), l.layers.iter().map(|la| la.instances.len()).sum())).collect(),
            sheets: p.sheets.iter().map(|s| s.name.clone()).collect(),
            types: p
                .types
                .iter()
                .map(|t| {
                    let tag = match t.plugin {
                        Plugin::Sprite => "spr",
                        Plugin::TiledBg => "tbg",
                        Plugin::Text => "txt",
                        Plugin::Keyboard => "kbd",
                        Plugin::Mouse => "mouse",
                        Plugin::Other => "obj",
                    };
                    (t.name.clone(), tag, t.is_family)
                })
                .collect(),
        });
        let mut go_tab: Option<Tab> = None;
        let mut go_place: Option<String> = None;
        let mut do_new = false;
        let mut do_open = false;
        let mut do_import = false;
        let mut add_layout = false;
        let mut add_sheet = false;

        egui::SidePanel::left("projet").resizable(true).default_width(250.0).show(ctx, |ui| {
            ui.heading("Projet");
            ui.separator();
            let Some(s) = snap else {
                ui.label("Aucun projet ouvert.");
                ui.add_space(8.0);
                if ui.button("Creer un projet").clicked() {
                    do_new = true;
                }
                if ui.button("Ouvrir un .c3p…").clicked() {
                    do_open = true;
                }
                return;
            };
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.label(egui::RichText::new(&s.name).strong());
                ui.add_space(4.0);
                ui.collapsing(format!("Layouts ({})", s.layouts.len()), |ui| {
                    for (i, (name, n)) in s.layouts.iter().enumerate() {
                        let sel_tab = matches!(self.tab, Tab::Layout(j) if j == i);
                        if ui.selectable_label(sel_tab, format!("{name}  ({n} instances)")).clicked() {
                            go_tab = Some(Tab::Layout(i));
                        }
                    }
                    if ui.small_button("+ Nouveau layout").clicked() {
                        add_layout = true;
                    }
                });
                ui.collapsing(format!("Feuilles d'evenements ({})", s.sheets.len()), |ui| {
                    for (i, name) in s.sheets.iter().enumerate() {
                        let sel_tab = matches!(self.tab, Tab::Events(j) if j == i);
                        if ui.selectable_label(sel_tab, name).clicked() {
                            go_tab = Some(Tab::Events(i));
                        }
                    }
                    if ui.small_button("+ Nouvelle feuille").clicked() {
                        add_sheet = true;
                    }
                });
                ui.collapsing(format!("Objets ({})", s.types.len()), |ui| {
                    for (name, tag, family) in &s.types {
                        let placing = matches!(&self.tool, Tool::Place(n) if n == name);
                        if ui.selectable_label(placing, format!("[{tag}] {name}")).clicked() {
                            if *family {
                                // rien
                            } else {
                                go_place = Some(name.clone());
                            }
                        }
                    }
                });
                if ui.selectable_label(matches!(self.tab, Tab::Media), format!("Medias importes ({})", self.media.len())).clicked() {
                    go_tab = Some(Tab::Media);
                }
            });
        });

        if let Some(t) = go_tab {
            let is_layout = matches!(t, Tab::Layout(_));
            self.tab = t;
            if is_layout {
                self.tool = Tool::Select;
            }
        }
        if let Some(n) = go_place {
            self.tool = Tool::Place(n.clone());
            self.say(0, format!("clic dans le layout pour placer « {n} »"));
        }
        if do_new {
            self.new_project();
        }
        if do_open {
            let paths = assets::native_pick_files();
            if paths.is_empty() {
                self.show_open = true;
            } else {
                for p in &paths {
                    self.open_project_path(p);
                }
            }
        }
        if do_import {
            let paths = assets::native_pick_files();
            if paths.is_empty() {
                self.pathwin_save = false;
                self.show_pathwin = true;
            } else {
                self.import_paths(&paths);
            }
        }
        if add_layout {
            self.add_layout();
        }
        if add_sheet {
            self.add_sheet();
        }
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        let mut messages: Vec<(u8, String)> = Vec::new();
        let mut delete_uid: Option<usize> = None;
        egui::SidePanel::right("proprietes").resizable(true).default_width(270.0).show(ctx, |ui| {
            ui.heading("Proprietes");
            ui.separator();
            let Some(p) = self.project.as_mut() else {
                ui.label("-");
                return;
            };
            // instance selectionnee ?
            let mut found: Option<(usize, usize, usize)> = None; // (layout, layer, idx)
            if let Some(uid) = self.sel {
                'outer: for (li, l) in p.layouts.iter().enumerate() {
                    for (la, layer) in l.layers.iter().enumerate() {
                        for (k, inst) in layer.instances.iter().enumerate() {
                            if inst.uid == uid {
                                found = Some((li, la, k));
                                break 'outer;
                            }
                        }
                    }
                }
            }
            if let Some((li, la, k)) = found {
                let l = &mut p.layouts[li];
                let inst = &mut l.layers[la].instances[k];
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(egui::RichText::new(&inst.type_name).strong());
                    ui.label(egui::RichText::new(format!("uid {}", inst.uid)).small().color(COL_DIM));
                    ui.separator();
                    ui.label("Position / taille");
                    grid_val(ui, "X", &mut inst.x, 1.0);
                    grid_val(ui, "Y", &mut inst.y, 1.0);
                    grid_val(ui, "Largeur", &mut inst.w, 1.0);
                    grid_val(ui, "Hauteur", &mut inst.h, 1.0);
                    grid_val(ui, "Angle", &mut inst.angle, 1.0);
                    grid_val(ui, "Origine X", &mut inst.ox, 0.01);
                    grid_val(ui, "Origine Y", &mut inst.oy, 0.01);
                    let mut op = inst.opacity * 100.0;
                    ui.horizontal(|ui| {
                        ui.label("Opacite");
                        ui.add(egui::DragValue::new(&mut op).speed(1.0).suffix(" %"));
                    });
                    inst.opacity = (op / 100.0).clamp(0.0, 1.0);
                    ui.separator();
                    if !inst.vars.is_empty() {
                        ui.label("Variables d'instance");
                        for (k, v) in &inst.vars {
                            ui.label(format!("{k} = {}", v.text()));
                        }
                        ui.separator();
                    }
                    if !inst.behaviors.is_empty() {
                        ui.label("Comportements");
                        for b in &inst.behaviors {
                            ui.label(format!("- {} ({})", b.name, b.kind.name()));
                        }
                    }
                    ui.separator();
                    if ui.button("Supprimer l'instance").clicked() {
                        delete_uid = Some(inst.uid);
                    }
                });
            } else if let Tab::Layout(li) = self.tab {
                if let Some(l) = p.layouts.get_mut(li) {
                    ui.label(egui::RichText::new("Layout").strong());
                    grid_val(ui, "Largeur", &mut l.width, 8.0);
                    grid_val(ui, "Hauteur", &mut l.height, 8.0);
                    ui.separator();
                    let names: Vec<String> = p.sheets.iter().map(|s| s.name.clone()).collect();
                    let mut cur = l.event_sheet.clone().unwrap_or_default();
                    egui::ComboBox::from_label("Feuille").selected_text(if cur.is_empty() { "-".to_string() } else { cur.clone() }).show_ui(ui, |ui| {
                        ui.selectable_value(&mut cur, String::new(), "-");
                        for n in &names {
                            ui.selectable_value(&mut cur, n.clone(), n);
                        }
                    });
                    l.event_sheet = if cur.is_empty() { None } else { Some(cur) };
                    ui.separator();
                    ui.label("Couches");
                    for la in l.layers.iter_mut() {
                        ui.horizontal(|ui| {
                            ui.label(&la.name);
                            ui.checkbox(&mut la.visible, "");
                        });
                    }
                    if ui.button("Ajouter une couche").clicked() {
                        let names: Vec<String> = l.layers.iter().map(|x| x.name.clone()).collect();
                        l.layers.push(Layer {
                            name: unique_name(&names, "Calque"),
                            parallax: (100.0, 100.0),
                            visible: true,
                            opacity: 1.0,
                            instances: vec![],
                        });
                        messages.push((0, "couche ajoutee".into()));
                    }
                }
            } else {
                ui.label("Selectionnez une instance dans un layout.");
            }
        });
        for (lvl, m) in messages {
            self.say(lvl, m);
        }
        if let Some(uid) = delete_uid {
            if let Some(p) = self.project.as_mut() {
                for l in &mut p.layouts {
                    for layer in &mut l.layers {
                        if let Some(k) = layer.instances.iter().position(|i| i.uid == uid) {
                            layer.instances.remove(k);
                            break;
                        }
                    }
                }
            }
            self.sel = None;
            self.dirty = true;
            self.say(0, format!("instance {uid} supprimee"));
        }
    }

    fn bottom_panel(&mut self, ctx: &egui::Context) {
        if self.show_console {
            egui::TopBottomPanel::bottom("console").resizable(true).default_height(140.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Console").strong());
                    if ui.small_button("Vider").clicked() {
                        self.log.clear();
                    }
                });
                egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                    for (lvl, m) in &self.log {
                        let c = match lvl {
                            1 => COL_WARN,
                            2 => COL_ERR,
                            _ => COL_DIM,
                        };
                        ui.label(egui::RichText::new(m).color(c).small().monospace());
                    }
                });
            });
        }
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let zoom = match self.tab {
                    Tab::Layout(i) => self.views.get(&i).copied().unwrap_or((1.0, Pos2::ZERO)).0,
                    _ => 1.0,
                };
                ui.label(format!("zoom {:.0}%", zoom * 100.0));
                ui.separator();
                match (&self.tool, &self.tab) {
                    (Tool::Place(n), Tab::Layout(_)) => ui.label(egui::RichText::new(format!("placement : {n}")).color(COL_ACCENT)),
                    _ => ui.label("selection"),
                };
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button(if self.show_console { "masquer la console" } else { "console" }).clicked() {
                        self.show_console = !self.show_console;
                    }
                    ui.label(format!("{} media(s)", self.media.len()));
                });
            });
        });
    }

    fn windows(&mut self, ctx: &egui::Context) {
        if self.show_pathwin {
            let save = self.pathwin_save;
            let title = if save { "Enregistrer le projet (.c3p)" } else { "Importer des fichiers" };
            let mut open = self.show_pathwin;
            egui::Window::new(title).open(&mut open).collapsible(false).show(ctx, |ui| {
                ui.label(if save {
                    "Chemin complet du fichier .c3p a enregistrer :"
                } else {
                    "Chemin du fichier a importer (tout type) :"
                });
                let resp = ui.text_edit_singleline(&mut self.path_buf);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Valider").clicked() || (resp.lost_focus() && enter) {
                    let path = PathBuf::from(self.path_buf.trim());
                    if save {
                        self.save_project(&path);
                    } else if path.is_file() {
                        match std::fs::read(&path) {
                            Ok(b) => {
                                let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                                self.import_bytes(name, b);
                            }
                            Err(e) => self.say(2, format!("{e}")),
                        }
                    } else {
                        self.say(1, "chemin introuvable");
                    }
                    self.show_pathwin = false;
                }
                if ui.button("Annuler").clicked() {
                    self.show_pathwin = false;
                }
            });
            self.show_pathwin = open;
        }
        if self.show_open {
            let mut open = self.show_open;
            egui::Window::new("Ouvrir un projet").open(&mut open).collapsible(false).show(ctx, |ui| {
                ui.label("Chemin du fichier .c3p :");
                let resp = ui.text_edit_singleline(&mut self.open_buf);
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui.button("Ouvrir").clicked() || (resp.lost_focus() && enter) {
                    let p = PathBuf::from(self.open_buf.trim());
                    self.open_project_path(&p);
                    self.show_open = false;
                }
                if ui.button("Annuler").clicked() {
                    self.show_open = false;
                }
            });
            self.show_open = open;
        }
    }

    fn handle_dropped(&mut self, ctx: &egui::Context) {
        let (dropped, hovered) = ctx.input(|i| (i.raw.dropped_files.clone(), !i.raw.hovered_files.is_empty()));
        if hovered {
            let sr = ctx.input(|i| i.screen_rect);
            ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("drop-overlay")))
                .rect_filled(sr, 0.0, Color32::from_rgba_unmultiplied(232, 140, 46, 26));
        }
        for f in dropped {
            let bytes = match f.bytes.as_ref() {
                Some(b) => b.to_vec(),
                None => match &f.path {
                    Some(p) => match std::fs::read(p) {
                        Ok(b) => b,
                        Err(e) => {
                            self.say(2, format!("{} : {e}", p.display()));
                            continue;
                        }
                    },
                    None => continue,
                },
            };
            let name = f
                .path
                .as_ref()
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().to_string()))
                .unwrap_or_else(|| f.name.clone());
            self.import_bytes(name, bytes);
        }
    }
}

impl eframe::App for EditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_fonts(ctx);
        self.handle_dropped(ctx);
        let (f5, esc) = ctx.input(|i| (i.key_pressed(egui::Key::F5), i.key_pressed(egui::Key::Escape)));
        if f5 {
            if self.preview.is_some() {
                self.stop_preview();
            } else {
                self.run_preview();
            }
        }
        if esc && self.preview.is_some() {
            self.stop_preview();
        }
        if esc {
            if matches!(self.tool, Tool::Place(_)) {
                self.tool = Tool::Select;
            }
        }
        self.top_menu(ctx);
        self.toolbar(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.bottom_panel(ctx);
        self.windows(ctx);
        egui::CentralPanel::default().show(ctx, |ui| {
            self.center(ui, ctx);
        });
        ctx.request_repaint();
    }
}

fn grid_val(ui: &mut egui::Ui, label: &str, v: &mut f32, speed: f64) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(egui::DragValue::new(v).speed(speed));
    });
}
