//! CLI c3rs : `run` (fenêtre), `render` (PNG headless), `inspect`, `validate`.

use c3rs::c3p;
use c3rs::render::FrameBuf;
use c3rs::runtime::{Engine, InputState};
use std::path::PathBuf;

fn help() -> ! {
    println!("c3rs v0.1.0 — runtime de jeux Construct 3 (.c3p) en Rust");
    println!();
    println!("USAGE:");
    println!("  c3rs run      <fichier.c3p> [options]   Lance le jeu dans une fenêtre");
    println!("  c3rs render   <fichier.c3p> [out.png] [options]  Rend une image (headless)");
    println!("  c3rs inspect  <fichier.c3p>             Affiche la structure du projet");
    println!("  c3rs validate <fichier.c3p>             Vérifie le projet (références)");
    println!();
    println!("OPTIONS:");
    println!("  --layout <nom>    Layout de départ (défaut : firstLayout)");
    println!("  --w <px>          Largeur de fenêtre (défaut : viewport du projet)");
    println!("  --h <px>          Hauteur de fenêtre");
    println!("  --ticks <n>       (render) nombre de ticks simulés avant capture");
    println!("  --keys <code,..>  (render) touches maintenues (keycodes JS: 39=droite...)");
    println!();
    println!("CONTRÔLES en jeu: flèches/WASD + Espace (plateforme), Échap pour quitter");
    std::process::exit(0);
}

struct Args {
    cmd: String,
    file: PathBuf,
    extra: Vec<String>,
    layout: String,
    width: Option<f32>,
    height: Option<f32>,
    ticks: usize,
    keys: Vec<u32>,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.is_empty() || argv[0] == "help" || argv[0] == "--help" || argv[0] == "-h" {
        help();
    }
    let cmd = argv[0].clone();
    let mut file: Option<PathBuf> = None;
    let mut extra = Vec::new();
    let mut layout = String::new();
    let mut width = None;
    let mut height = None;
    let mut ticks = 1usize;
    let mut keys = Vec::new();
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--layout" => {
                i += 1;
                layout = argv.get(i).cloned().unwrap_or_default();
            }
            "--w" => {
                i += 1;
                width = argv.get(i).and_then(|v| v.parse().ok());
            }
            "--h" => {
                i += 1;
                height = argv.get(i).and_then(|v| v.parse().ok());
            }
            "--ticks" => {
                i += 1;
                ticks = argv.get(i).and_then(|v| v.parse().ok()).unwrap_or(1);
            }
            "--keys" => {
                i += 1;
                if let Some(list) = argv.get(i) {
                    keys = list.split(',').filter_map(|k| k.trim().parse().ok()).collect();
                }
            }
            other => {
                if file.is_none() {
                    file = Some(PathBuf::from(other));
                } else {
                    extra.push(other.to_string());
                }
            }
        }
        i += 1;
    }
    match file {
        Some(f) => Args { cmd, file: f, extra, layout, width, height, ticks, keys },
        None => help(),
    }
}

fn load_or_die(path: &PathBuf) -> (c3rs::model::Project, Vec<String>) {
    match c3p::load(path) {
        Ok((p, warns)) => (p, warns),
        Err(e) => {
            eprintln!("Erreur : {e}");
            std::process::exit(1);
        }
    }
}

fn main() {
    let args = parse_args();
    match args.cmd.as_str() {
        "inspect" | "info" => {
            let (project, warns) = load_or_die(&args.file);
            print_inspect(&project, &warns);
        }
        "validate" => {
            let (project, warns) = load_or_die(&args.file);
            let errors = validate(&project);
            for w in &warns {
                println!("avertissement: {w}");
            }
            if errors.is_empty() {
                println!("OK : {} layout(s), {} objet(s), {} feuille(s) d'événements",
                    project.layouts.len(), project.types.len(), project.sheets.len());
            } else {
                for e in &errors {
                    println!("erreur: {e}");
                }
                std::process::exit(1);
            }
        }
        "render" => {
            let (project, warns) = load_or_die(&args.file);
            for w in &warns {
                eprintln!("avertissement: {w}");
            }
            let out = args.extra.first().cloned().unwrap_or_else(|| "frame.png".to_string());
            let mut eng = Engine::new(project, &args.layout).expect("moteur");
            if let (Some(w), Some(h)) = (args.width, args.height) {
                eng.view_w = w;
                eng.view_h = h;
            }
            // injecte les touches maintenues
            for k in &args.keys {
                eng.input.down.insert(*k);
            }
            for _ in 0..args.ticks {
                eng.tick(1.0 / 60.0);
            }
            let fb = c3rs::runtime::render_frame(&eng);
            write_png(&out, &fb).expect("écriture PNG");
            println!("image écrite : {out} ({}x{})", fb.w, fb.h);
            for w in &eng.warnings {
                eprintln!("runtime: {w}");
            }
        }
        "run" => {
            let (project, warns) = load_or_die(&args.file);
            for w in &warns {
                eprintln!("avertissement: {w}");
            }
            run_window(project, &args.layout, args.width, args.height);
        }
        _ => help(),
    }
}

fn print_inspect(project: &c3rs::model::Project, warns: &[String]) {
    println!("Projet      : {}", project.name);
    println!("Viewport    : {} x {}", project.viewport_w, project.viewport_h);
    println!("1er layout  : {}", project.first_layout);
    println!();
    println!("Layouts ({}):", project.layouts.len());
    for l in &project.layouts {
        let n: usize = l.layers.iter().map(|la| la.instances.len()).sum();
        println!("  - {:<20} {}x{}  {} instance(s)  events: {}",
            l.name, l.width, l.height, n,
            l.event_sheet.as_deref().unwrap_or("-"));
    }
    println!();
    println!("Objets ({}):", project.types.len());
    for t in &project.types {
        let behs: Vec<&str> = t.behaviors.iter().map(|b| b.name.as_str()).collect();
        println!("  - {:<20} {:?}{}{}", t.name,
            t.plugin,
            if behs.is_empty() { String::new() } else { format!("  [{}]", behs.join(", ")) },
            if t.is_family { " (famille)" } else { "" });
    }
    println!();
    println!("Feuilles d'événements ({}):", project.sheets.len());
    for s in &project.sheets {
        println!("  - {}", s.name);
    }
    if !warns.is_empty() {
        println!();
        println!("Avertissements de chargement :");
        for w in warns {
            println!("  ! {w}");
        }
    }
}

fn validate(project: &c3rs::model::Project) -> Vec<String> {
    let mut errors = Vec::new();
    if project.layouts.is_empty() {
        errors.push("aucun layout".into());
    }
    for l in &project.layouts {
        if let Some(es) = &l.event_sheet {
            if project.sheet(es).is_none() {
                errors.push(format!("layout « {} » → feuille « {} » introuvable", l.name, es));
            }
        }
        for la in &l.layers {
            for i in &la.instances {
                if project.type_of(&i.type_name).is_none() {
                    errors.push(format!("instance de type inconnu « {} » dans « {} »", i.type_name, l.name));
                }
            }
        }
    }
    errors
}

fn write_png(out: &str, fb: &FrameBuf) -> Result<(), String> {
    let file = std::fs::File::create(out).map_err(|e| e.to_string())?;
    let mut enc = png::Encoder::new(file, fb.w as u32, fb.h as u32);
    enc.set_color(png::ColorType::Rgba);
    enc.set_depth(png::BitDepth::Eight);
    let mut writer = enc.write_header().map_err(|e| e.to_string())?;
    let mut rgba = Vec::with_capacity(fb.px.len() * 4);
    for p in &fb.px {
        rgba.push(((p >> 16) & 0xFF) as u8);
        rgba.push(((p >> 8) & 0xFF) as u8);
        rgba.push((p & 0xFF) as u8);
        rgba.push(0xFF);
    }
    writer.write_image_data(&rgba).map_err(|e| e.to_string())
}

// ---------------- fenêtre (minifb) ----------------

fn minifb_to_js(k: minifb::Key) -> Option<u32> {
    use minifb::Key::*;
    Some(match k {
        Left => 37,
        Up => 38,
        Right => 39,
        Down => 40,
        Space => 32,
        Enter => 13,
        LeftShift | RightShift => 16,
        LeftCtrl | RightCtrl => 17,
        Escape => 27,
        A => 65, B => 66, C => 67, D => 68, E => 69, F => 70, G => 71, H => 72,
        I => 73, J => 74, K => 75, L => 76, M => 77, N => 78, O => 79, P => 80,
        Q => 81, R => 82, S => 83, T => 84, U => 85, V => 86, W => 87, X => 88,
        Y => 89, Z => 90,
        Num0 => 48, Num1 => 49, Num2 => 50, Num3 => 51, Num4 => 52,
        Num5 => 53, Num6 => 54, Num7 => 55, Num8 => 56, Num9 => 57,
        _ => return None,
    })
}

fn run_window(project: c3rs::model::Project, layout: &str, w: Option<f32>, h: Option<f32>) {
    use minifb::{Key, Window, WindowOptions};
    let vw = w.unwrap_or(project.viewport_w).max(64.0) as usize;
    let vh = h.unwrap_or(project.viewport_h).max(64.0) as usize;
    let title = format!("{} — c3rs", project.name);
    let mut window = match Window::new(&title, vw, vh, WindowOptions {
        resize: true,
        ..WindowOptions::default()
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("impossible de créer la fenêtre : {e}");
            std::process::exit(1);
        }
    };
    window.limit_update_rate(Some(std::time::Duration::from_micros(16_600)));
    let mut eng = match Engine::new(project, layout) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    eng.view_w = vw as f32;
    eng.view_h = vh as f32;
    let mut prev_keys: Vec<Key> = Vec::new();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // entrées clavier
        let keys = window.get_keys();
        let now: Vec<u32> = keys.iter().filter_map(|k| minifb_to_js(*k)).collect();
        eng.input.down = now.iter().copied().collect();
        eng.input.pressed = now.iter().filter(|k| !prev_keys_js(&prev_keys).contains(k)).copied().collect();
        if let Some((mx, my)) = window.get_mouse_pos(minifb::MouseMode::Pass) {
            eng.input.mouse = (mx, my);
        }
        if window.get_mouse_down(minifb::MouseButton::Left) {
            eng.input.mouse_down.insert(0);
            eng.input.mouse_pressed.insert(0);
        }
        prev_keys = keys;
        // simulation + rendu
        eng.tick(1.0 / 60.0);
        let fb = c3rs::runtime::render_frame(&eng);
        window
            .update_with_buffer(&fb.px, fb.w, fb.h)
            .expect("mise à jour framebuffer");
    }
}

fn prev_keys_js(prev: &[minifb::Key]) -> Vec<u32> {
    prev.iter().filter_map(|k| minifb_to_js(*k)).collect()
}

// évite l'avertissement sur InputState non utilisé directement
#[allow(dead_code)]
fn _touch(_: InputState) {}
