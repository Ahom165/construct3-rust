//! c3-core — moteur de type Construct 3 (réimplémentation originale).
//! Parse les fichiers .c3p (zip + JSON), importe n'importe quel fichier,
//! exporte des builds HTML5 autonomes et des .c3p au format natif "c3r".

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use zip::{write::FileOptions, ZipArchive, ZipWriter};

pub const FORMAT: &str = "c3r-1";

// ---------------------------------------------------------------- data model

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Variable {
    pub name: String,
    #[serde(rename = "type", default = "default_type")]
    pub var_type: String,
    #[serde(default)]
    pub initial: Value,
}
fn default_type() -> String { "number".into() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Frame {
    #[serde(default)]
    pub texture: String,
    #[serde(default = "one")]
    pub duration: f64,
}
fn one() -> f64 { 1.0 }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Animation {
    #[serde(default = "default_anim_name")]
    pub name: String,
    #[serde(default = "five")]
    pub speed: f64,
    #[serde(default = "true_f")]
    pub looping: bool,
    #[serde(default)]
    pub frames: Vec<Frame>,
}
fn five() -> f64 { 5.0 }
fn true_f() -> bool { true }
fn default_anim_name() -> String { "Default".into() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Behavior {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct ObjectType {
    pub uid: i64,
    pub name: String,
    #[serde(default = "default_plugin")]
    pub plugin: String,
    #[serde(default)]
    pub is_global: bool,
    #[serde(default)]
    pub behaviors: Vec<Behavior>,
    #[serde(default)]
    pub instance_vars: Vec<Variable>,
    #[serde(default)]
    pub animations: Vec<Animation>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
    #[serde(default)]
    pub texture: String,
}
fn default_plugin() -> String { "sprite".into() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Instance {
    pub uid: i64,
    pub object: String,
    pub x: f64,
    pub y: f64,
    #[serde(default = "thirtytwo")]
    pub width: f64,
    #[serde(default = "thirtytwo")]
    pub height: f64,
    #[serde(default)]
    pub angle: f64,
    #[serde(default)]
    pub z: f64,
    #[serde(default = "one_hundred")]
    pub opacity: f64,
    #[serde(default)]
    pub vars: BTreeMap<String, Value>,
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}
fn thirtytwo() -> f64 { 32.0 }
fn one_hundred() -> f64 { 100.0 }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Layer {
    #[serde(default = "default_layer_name")]
    pub name: String,
    #[serde(default = "true_f")]
    pub visible: bool,
    #[serde(default = "one_hundred")]
    pub opacity: f64,
    #[serde(default = "one_hundred")]
    pub parallax_x: f64,
    #[serde(default = "one_hundred")]
    pub parallax_y: f64,
    #[serde(default)]
    pub instances: Vec<Instance>,
}
fn default_layer_name() -> String { "Layer 0".into() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Layout {
    pub uid: i64,
    pub name: String,
    pub width: u32,
    pub height: u32,
    #[serde(default = "default_bg")]
    pub background: String,
    #[serde(default)]
    pub layers: Vec<Layer>,
    #[serde(default)]
    pub event_sheet: String,
}
fn default_bg() -> String { "#ffffff".into() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Condition {
    pub id: String,
    #[serde(default = "system")]
    pub object: String,
    #[serde(default)]
    pub params: Vec<Value>,
}
fn system() -> String { "system".into() }

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Action {
    pub id: String,
    #[serde(default = "system")]
    pub object: String,
    #[serde(default)]
    pub params: Vec<Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Event {
    #[serde(default)]
    pub disabled: bool,
    #[serde(default)]
    pub is_group: bool,
    #[serde(default)]
    pub group_name: String,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    #[serde(default)]
    pub actions: Vec<Action>,
    #[serde(default)]
    pub children: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EventSheet {
    pub uid: i64,
    pub name: String,
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Asset {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub mime: String,
    #[serde(default)]
    pub data: String, // data URI
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Project {
    #[serde(default = "fmt")]
    pub format: String,
    #[serde(default = "default_project_name")]
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "one_dot_zero")]
    pub version: String,
    #[serde(default = "def_w")]
    pub width: u32,
    #[serde(default = "def_h")]
    pub height: u32,
    #[serde(default)]
    pub first_layout: String,
    #[serde(default)]
    pub object_types: Vec<ObjectType>,
    #[serde(default)]
    pub layouts: Vec<Layout>,
    #[serde(default)]
    pub event_sheets: Vec<EventSheet>,
    #[serde(default)]
    pub global_vars: Vec<Variable>,
    #[serde(default)]
    pub assets: Vec<Asset>,
}
fn fmt() -> String { FORMAT.into() }
fn default_project_name() -> String { "Nouveau projet".into() }
fn one_dot_zero() -> String { "1.0".into() }
fn def_w() -> u32 { 1280 }
fn def_h() -> u32 { 720 }

#[derive(Debug, Default)]
pub struct ImportResult {
    pub project: Project,
    pub warnings: Vec<String>,
}

// ------------------------------------------------------------------ helpers

fn jstr(v: &Value, keys: &[&str], def: &str) -> String {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            return s.to_string();
        }
        if let Some(n) = v.get(*k).and_then(|x| x.as_f64()) {
            return n.to_string();
        }
    }
    def.to_string()
}
fn jf64(v: &Value, keys: &[&str], def: f64) -> f64 {
    for k in keys {
        if let Some(x) = v.get(*k) {
            let n = num(x);
            if !n.is_nan() {
                return n;
            }
        }
    }
    def
}
fn jbool(v: &Value, keys: &[&str], def: bool) -> bool {
    for k in keys {
        if let Some(b) = v.get(*k).and_then(|x| x.as_bool()) {
            return b;
        }
        if let Some(n) = v.get(*k).and_then(|x| x.as_f64()) {
            return n != 0.0;
        }
    }
    def
}
fn num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}
fn varr(v: &Value, keys: &[&str]) -> Vec<Value> {
    for k in keys {
        if let Some(a) = v.get(*k).and_then(|x| x.as_array()) {
            return a.clone();
        }
    }
    Vec::new()
}
fn value_map(v: Option<&Value>) -> BTreeMap<String, Value> {
    let mut m = BTreeMap::new();
    if let Some(Value::Object(o)) = v {
        for (k, val) in o {
            m.insert(k.clone(), simplify(val));
        }
    }
    m
}
fn simplify(v: &Value) -> Value {
    match v {
        Value::Object(o) => {
            if let Some(x) = o.get("value") {
                simplify(x)
            } else {
                v.clone()
            }
        }
        Value::Array(a) => Value::Array(a.iter().map(simplify).collect()),
        _ => v.clone(),
    }
}
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

// --------------------------------------------------- fichiers importables

/// Détecte (kind, mime) d'un fichier par magie puis extension.
/// "n'importe quoi" : image, audio, vidéo, police, données, archive, autre.
pub fn detect_kind(name: &str, data: &[u8]) -> (&'static str, &'static str) {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) { return ("image", "image/png"); }
    if data.starts_with(&[0xFF, 0xD8]) { return ("image", "image/jpeg"); }
    if data.starts_with(b"GIF8") { return ("image", "image/gif"); }
    if data.len() > 12 && data.starts_with(b"RIFF") {
        if &data[8..12] == b"WEBP" { return ("image", "image/webp"); }
        if &data[8..12] == b"WAVE" { return ("audio", "audio/wav"); }
    }
    if data.starts_with(b"OggS") { return ("audio", "audio/ogg"); }
    if data.len() > 12 && &data[4..8] == b"ftyp" { return ("video", "video/mp4"); }
    if data.starts_with(b"ID3") || (!data.is_empty() && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0) {
        return ("audio", "audio/mpeg");
    }
    if data.starts_with(&[0x00, 0x01, 0x00, 0x00]) || data.starts_with(b"OTTO") || data.starts_with(b"wOFF") {
        return ("font", "font/ttf");
    }
    let lower = name.to_ascii_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "svg" => ("image", "image/svg+xml"),
        "bmp" => ("image", "image/bmp"),
        "ico" => ("image", "image/x-icon"),
        "wav" => ("audio", "audio/wav"),
        "ogg" | "opus" => ("audio", "audio/ogg"),
        "mp3" => ("audio", "audio/mpeg"),
        "m4a" | "aac" => ("audio", "audio/mp4"),
        "flac" => ("audio", "audio/flac"),
        "mid" | "midi" => ("audio", "audio/midi"),
        "mp4" | "m4v" => ("video", "video/mp4"),
        "webm" => ("video", "video/webm"),
        "mov" | "avi" | "mkv" => ("video", "video/x-generic"),
        "ttf" | "otf" | "woff" | "woff2" => ("font", "font/ttf"),
        "json" => ("data", "application/json"),
        "txt" | "md" | "csv" | "xml" | "yml" | "yaml" | "toml" | "log" | "ini" => ("data", "text/plain"),
        "c3p" | "capx" | "zip" => ("archive", "application/zip"),
        "js" | "ts" | "py" | "rs" | "c" | "cpp" | "h" | "lua" | "html" | "css" => ("data", "text/plain"),
        "pdf" => ("other", "application/pdf"),
        _ => ("other", "application/octet-stream"),
    }
}

/// Taille (l,h) pour PNG / JPEG / GIF.
pub fn image_size(data: &[u8]) -> Option<(i32, i32)> {
    if data.starts_with(&[0x89, b'P', b'N', b'G']) && data.len() >= 24 {
        let w = i32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = i32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((w, h));
    }
    if data.starts_with(&[0xFF, 0xD8]) {
        let mut i = 2usize;
        while i + 9 < data.len() {
            if data[i] != 0xFF { i += 1; continue; }
            let m = data[i + 1];
            if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) { i += 2; continue; }
            if i + 4 > data.len() { break; }
            let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
                let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as i32;
                let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as i32;
                return Some((w, h));
            }
            i += 2 + len;
        }
    }
    if data.starts_with(b"GIF8") && data.len() >= 10 {
        let w = data[6] as i32 | ((data[7] as i32) << 8);
        let h = data[8] as i32 | ((data[9] as i32) << 8);
        return Some((w, h));
    }
    None
}

pub fn data_uri(mime: &str, data: &[u8]) -> String {
    format!("data:{};base64,{}", mime, B64.encode(data))
}
pub fn decode_data_uri(s: &str) -> Option<Vec<u8>> {
    let rest = s.strip_prefix("data:")?;
    let b64 = rest.split(',').nth(1)?;
    B64.decode(b64.trim()).ok()
}

pub fn import_asset(id: String, name: &str, data: &[u8]) -> Asset {
    let (kind, mime) = detect_kind(name, data);
    let (w, h) = if kind == "image" { image_size(data).unwrap_or((0, 0)) } else { (0, 0) };
    Asset {
        id,
        name: name.to_string(),
        kind: kind.into(),
        mime: mime.into(),
        data: data_uri(mime, data),
        width: w,
        height: h,
    }
}

fn push_asset(assets: &mut Vec<Asset>, counter: &mut u64, name: &str, data: &[u8]) -> String {
    let (kind, mime) = detect_kind(name, data);
    let du = data_uri(mime, data);
    if let Some(a) = assets.iter().find(|a| a.data == du) {
        return a.id.clone();
    }
    *counter += 1;
    let id = format!("tx{}", counter);
    assets.push(Asset {
        id: id.clone(),
        name: name.to_string(),
        kind: kind.into(),
        mime: mime.into(),
        data: du,
        width: if kind == "image" { image_size(data).unwrap_or((0, 0)).0 } else { 0 },
        height: if kind == "image" { image_size(data).unwrap_or((0, 0)).1 } else { 0 },
    });
    id
}

fn find_first_data_uri(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::String(s) => {
            if s.starts_with("data:") { decode_data_uri(s) } else { None }
        }
        Value::Array(a) => a.iter().find_map(find_first_data_uri),
        Value::Object(o) => o.values().find_map(find_first_data_uri),
        _ => None,
    }
}

// ------------------------------------------------------- parse d'un .c3p

/// Ouvre un .c3p (ou .capx/.zip) et retourne le projet normalisé.
pub fn parse_c3p(bytes: &[u8]) -> Result<ImportResult, String> {
    let cur = Cursor::new(bytes);
    let mut ar = ZipArchive::new(cur).map_err(|e| format!("Archive illisible (zip attendu) : {}", e))?;
    let mut files: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut pj: Option<Vec<u8>> = None;
    for i in 0..ar.len() {
        let mut f = ar.by_index(i).map_err(|e| e.to_string())?;
        if f.is_dir() { continue; }
        let n = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        let ln = n.to_ascii_lowercase();
        if ln == "project.json" || ln.ends_with("/project.json") {
            pj = Some(buf);
        } else {
            files.insert(n, buf);
        }
    }
    let pj = pj.ok_or("project.json introuvable dans l'archive")?;
    let v: Value = serde_json::from_slice(&pj).map_err(|e| format!("JSON invalide : {}", e))?;
    if v.get("format").and_then(|x| x.as_str()) == Some(FORMAT) || v.get("object_types").is_some() {
        native_import(v)
    } else {
        Ok(c3_import(v, &files))
    }
}

fn native_import(v: Value) -> Result<ImportResult, String> {
    let mut p: Project = serde_json::from_value(v).map_err(|e| e.to_string())?;
    let mut warnings = Vec::new();
    if p.layouts.is_empty() {
        p.layouts.push(Layout {
            uid: 1,
            name: "Layout 1".into(),
            width: p.width,
            height: p.height,
            background: "#ffffff".into(),
            layers: vec![Layer::default()],
            event_sheet: "Event sheet 1".into(),
        });
        p.event_sheets.push(EventSheet { uid: 2, name: "Event sheet 1".into(), events: vec![] });
        p.first_layout = "Layout 1".into();
        warnings.push("Projet sans layout : un layout vide a été créé.".into());
    }
    if p.first_layout.is_empty() {
        p.first_layout = p.layouts[0].name.clone();
    }
    Ok(ImportResult { project: p, warnings })
}

fn normalize_plugin(raw: &str) -> String {
    let s = raw.to_lowercase().replace(['-', ' '], "");
    match s.as_str() {
        "tiledbackground" | "tiled" | "tiledbg" => "tiledbg".into(),
        "keyboard" => "keyboard".into(),
        "mouse" => "mouse".into(),
        "touch" => "touch".into(),
        "audio" => "audio".into(),
        "text" => "text".into(),
        "system" => "system".into(),
        "timer" => "timer".into(),
        "button" => "button".into(),
        "textbox" => "textbox".into(),
        _ => if s.is_empty() { "sprite".into() } else { s },
    }
}

fn normalize_behavior(raw: &str) -> String {
    let s = raw.to_lowercase().replace(['-', ' '], "_");
    match s.as_str() {
        "8direction" | "8_direction" => "eightdir".into(),
        "destroyoutside" | "destroy_outside" | "destroyoutsidepopup" => "destroyoutside".into(),
        "solid" | "platform" | "bullet" | "fade" | "sine" | "pin" | "wrap" | "scrollto" | "dragndrop" | "timer" => s,
        _ => s,
    }
}

fn normalize_vtype(raw: &str) -> String {
    let s = raw.to_lowercase();
    match s.as_str() {
        "string" | "text" => "string".into(),
        "boolean" | "bool" => "boolean".into(),
        _ => "number".into(),
    }
}

pub fn normalize_condition(raw: &str, warnings: &mut Vec<String>) -> String {
    let s = raw.trim().to_lowercase().replace([' ', '-'], "_");
    if s.is_empty() { return String::new(); }
    let has = |subs: &[&str]| subs.iter().all(|x| s.contains(x));
    if has(&["start"]) && has(&["layout"]) { return "on_start_of_layout".into(); }
    if s == "every_tick" || s == "tick" { return "every_tick".into(); }
    if has(&["every"]) && has(&["second"]) { return "every_x_seconds".into(); }
    if has(&["key"]) && has(&["press"]) { return "key_pressed".into(); }
    if has(&["key"]) && has(&["down"]) { return "key_down".into(); }
    if has(&["key"]) && has(&["releas"]) { return "key_released".into(); }
    if s.contains("collision") { return "on_collision_with".into(); }
    if s.contains("clicked") || s.contains("click") { return "on_object_clicked".into(); }
    if s.contains("created") { return "on_created".into(); }
    if s.contains("destroyed") { return "on_destroyed".into(); }
    if has(&["compare"]) && has(&["global"]) { return "compare_global_var".into(); }
    if has(&["compare"]) && (has(&["instance"]) || s.contains("variable") || s.ends_with("_var")) {
        return "compare_instance_var".into();
    }
    if has(&["compare"]) && s.ends_with("_x") { return "compare_x".into(); }
    if has(&["compare"]) && s.ends_with("_y") { return "compare_y".into(); }
    if has(&["on", "screen"]) || s.contains("is_on_screen") { return "is_on_screen".into(); }
    if s.contains("timer") { return "on_timer".into(); }
    if s.contains("chance") { return "random_chance".into(); }
    if s == "else" { return "else".into(); }
    let known = [
        "on_start_of_layout", "every_tick", "every_x_seconds", "key_pressed", "key_down",
        "key_released", "on_collision_with", "on_object_clicked", "on_created", "on_destroyed",
        "compare_global_var", "compare_instance_var", "compare_x", "compare_y", "is_on_screen",
        "on_timer", "random_chance", "else",
    ];
    if known.contains(&s.as_str()) { return s; }
    warnings.push(format!("Condition non reconnue : « {} » (ignorée à l'exécution)", raw));
    format!("__unknown:{}", s)
}

pub fn normalize_action(raw: &str, warnings: &mut Vec<String>) -> String {
    let s = raw.trim().to_lowercase().replace([' ', '-'], "_");
    if s.is_empty() { return String::new(); }
    let has = |subs: &[&str]| subs.iter().all(|x| s.contains(x));
    if has(&["create"]) { return "create_object".into(); }
    if s.contains("spawn") { return "spawn".into(); }
    if s == "destroy" || s.ends_with("destroy") { return "destroy".into(); }
    if has(&["set"]) {
        if s.ends_with("_x") { return "set_x".into(); }
        if s.ends_with("_y") && !s.contains("body") { return "set_y".into(); }
        if s.contains("angle") { return "set_angle".into(); }
        if s.contains("width") { return "set_width".into(); }
        if s.contains("height") { return "set_height".into(); }
        if s.contains("opacity") || s.contains("alpha") { return "set_opacity".into(); }
        if s.contains("z_order") || s == "set_z" { return "set_z".into(); }
        if has(&["font"]) { return "set_font_size".into(); }
        if has(&["animation"]) && has(&["frame"]) { return "set_animation_frame".into(); }
        if has(&["animation"]) { return "set_animation".into(); }
        if has(&["text"]) { return "set_text".into(); }
        if has(&["global"]) && has(&["variable", "var"]) { return "set_global_var".into(); }
        if has(&["instance"]) && has(&["variable", "var"]) { return "set_instance_var".into(); }
        if has(&["variable", "var"]) { return "set_instance_var".into(); }
    }
    if s.contains("goto") || (has(&["go"]) && has(&["layout"])) { return "go_to_layout".into(); }
    if s.contains("restart") { return "restart_layout".into(); }
    if s.contains("scroll") { return "scroll_to".into(); }
    if s.contains("sound") && (s.contains("play") || s == "play") { return "play_sound".into(); }
    if s.contains("velocity") { return "set_eightdir_velocity".into(); }
    if has(&["bullet"]) && has(&["speed"]) { return "set_bullet_speed".into(); }
    if s.contains("speed") { return "set_bullet_speed".into(); }
    if has(&["move"]) && has(&["forward"]) { return "move_forward".into(); }
    if s.contains("timer") { return "start_timer".into(); }
    if s.contains("pin") { return "pin_to".into(); }
    if s.contains("wait") { return "wait".into(); }
    let known = [
        "create_object", "spawn", "destroy", "set_x", "set_y", "set_angle", "set_width",
        "set_height", "set_opacity", "set_z", "set_text", "set_font_size", "set_animation",
        "set_animation_frame", "set_instance_var", "set_global_var", "go_to_layout",
        "restart_layout", "scroll_to", "play_sound", "set_eightdir_velocity",
        "set_bullet_speed", "move_forward", "start_timer", "pin_to", "wait", "set_position",
    ];
    if known.contains(&s.as_str()) { return s; }
    warnings.push(format!("Action non reconnue : « {} » (ignorée à l'exécution)", raw));
    format!("__unknown:{}", s)
}

fn frame_texture(
    fv: &Value,
    files: &BTreeMap<String, Vec<u8>>,
    assets: &mut Vec<Asset>,
    counter: &mut u64,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(t) = fv.get("texture") {
        if let Some(d) = t.get("data").and_then(|x| x.as_str()) {
            if let Some(b) = decode_data_uri(d) {
                return push_asset(assets, counter, "texture.png", &b);
            }
        }
        if let Some(s) = t.as_str() {
            if s.starts_with("data:") {
                if let Some(b) = decode_data_uri(s) {
                    return push_asset(assets, counter, "texture.png", &b);
                }
            } else if let Some(b) = files.get(s) {
                return push_asset(assets, counter, s, b);
            }
        }
        if let Some(b) = find_first_data_uri(t) {
            return push_asset(assets, counter, "texture.png", &b);
        }
    }
    if let Some(b) = find_first_data_uri(fv) {
        return push_asset(assets, counter, "texture.png", &b);
    }
    warnings.push("Texture de frame introuvable — frame vide.".into());
    String::new()
}

fn parse_instances(
    list: Vec<Value>,
    uid_to_type: &BTreeMap<i64, String>,
    files: &BTreeMap<String, Vec<u8>>,
    assets: &mut Vec<Asset>,
    counter: &mut u64,
    warnings: &mut Vec<String>,
) -> Vec<Instance> {
    let mut out = Vec::new();
    let _ = (files, assets, counter, warnings);
    for (ii, d) in list.iter().enumerate() {
        let obj = match d.get("object").or_else(|| d.get("object-class")).or_else(|| d.get("type")) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Number(n)) => uid_to_type
                .get(&(n.as_f64().unwrap_or(0.0) as i64))
                .cloned()
                .unwrap_or_default(),
            _ => String::new(),
        };
        if obj.is_empty() { continue; }
        let w = jf64(d, &["width"], 0.0);
        let h = jf64(d, &["height"], 0.0);
        let o = jf64(d, &["opacity"], 100.0);
        out.push(Instance {
            uid: jf64(d, &["uid", "instance-id"], ii as f64 + 1.0) as i64,
            object: obj,
            x: jf64(d, &["x"], 0.0),
            y: jf64(d, &["y"], 0.0),
            width: if w > 0.0 { w } else { 32.0 },
            height: if h > 0.0 { h } else { 32.0 },
            angle: jf64(d, &["angle"], 0.0),
            z: jf64(d, &["z-order", "zorder", "z"], 0.0),
            opacity: if o <= 1.0 && o > 0.0 { o * 100.0 } else if o == 0.0 { 0.0 } else { o },
            vars: value_map(d.get("variables")),
            properties: value_map(d.get("properties")),
        });
    }
    out
}

fn import_events(
    list: Vec<Value>,
    uid_to_type: &BTreeMap<i64, String>,
    warnings: &mut Vec<String>,
) -> Vec<Event> {
    let mut out = Vec::new();
    for e in list {
        let conds = varr(&e, &["conditions"])
            .into_iter()
            .map(|c| {
                let raw_id = jstr(&c, &["id", "condition-id", "type"], "");
                let obj = match c.get("object").or_else(|| c.get("object-class")) {
                    Some(Value::String(s)) => {
                        if s.eq_ignore_ascii_case("system") { "system".into() } else { s.clone() }
                    }
                    Some(Value::Number(n)) => uid_to_type
                        .get(&(n.as_f64().unwrap_or(0.0) as i64))
                        .cloned()
                        .unwrap_or_else(|| "system".into()),
                    _ => "system".into(),
                };
                let params = match c.get("parameters").or_else(|| c.get("params")) {
                    Some(Value::Array(a)) => a
                        .iter()
                        .map(|x| x.get("value").cloned().unwrap_or_else(|| x.clone()))
                        .collect(),
                    _ => vec![],
                };
                Condition {
                    id: normalize_condition(&raw_id, warnings),
                    object: obj,
                    params,
                }
            })
            .collect();
        let acts = varr(&e, &["actions"])
            .into_iter()
            .map(|a| {
                let raw_id = jstr(&a, &["id", "action-id", "type"], "");
                let obj = match a.get("object").or_else(|| a.get("object-class")) {
                    Some(Value::String(s)) => {
                        if s.eq_ignore_ascii_case("system") { "system".into() } else { s.clone() }
                    }
                    Some(Value::Number(n)) => uid_to_type
                        .get(&(n.as_f64().unwrap_or(0.0) as i64))
                        .cloned()
                        .unwrap_or_else(|| "system".into()),
                    _ => "system".into(),
                };
                let params = match a.get("parameters").or_else(|| a.get("params")) {
                    Some(Value::Array(a)) => a
                        .iter()
                        .map(|x| x.get("value").cloned().unwrap_or_else(|| x.clone()))
                        .collect(),
                    _ => vec![],
                };
                Action {
                    id: normalize_action(&raw_id, warnings),
                    object: obj,
                    params,
                }
            })
            .collect();
        let children = import_events(varr(&e, &["sub-events", "subEvents", "children"]), uid_to_type, warnings);
        out.push(Event {
            disabled: jbool(&e, &["disabled"], false),
            is_group: jbool(&e, &["is-group", "isGroup"], false)
                || e.get("group-name").is_some()
                || e.get("groupName").is_some(),
            group_name: jstr(&e, &["group-name", "groupName"], ""),
            conditions: conds,
            actions: acts,
            children,
        });
    }
    out
}

fn c3_import(v: Value, files: &BTreeMap<String, Vec<u8>>) -> ImportResult {
    let mut warnings: Vec<String> = Vec::new();
    let mut p = Project {
        format: FORMAT.into(),
        name: jstr(&v, &["projectName", "name"], "Projet importé"),
        author: jstr(&v, &["author", "projectAuthor"], ""),
        description: jstr(&v, &["description"], ""),
        version: "1.0".into(),
        width: jf64(&v, &["width"], 1280.0) as u32,
        height: jf64(&v, &["height"], 720.0) as u32,
        ..Default::default()
    };
    if let Some(ws) = v.get("windowSize") {
        let w = ws.get("width").or_else(|| ws.get(0));
        let h = ws.get("height").or_else(|| ws.get(1));
        if let Some(x) = w { let n = num(x); if !n.is_nan() { p.width = n as u32; } }
        if let Some(x) = h { let n = num(x); if !n.is_nan() { p.height = n as u32; } }
    }

    // ---- object types
    let mut uid_to_type: BTreeMap<i64, String> = BTreeMap::new();
    let mut aid: u64 = 0;
    let mut assets: Vec<Asset> = Vec::new();
    for o in varr(&v, &["object-types", "objectTypes"]) {
        let name = jstr(&o, &["name"], "Objet");
        let uid = jf64(&o, &["uid"], 0.0) as i64;
        uid_to_type.insert(uid, name.clone());
        let raw_plugin = jstr(&o, &["plugin-id", "pluginId", "plugin"], "sprite");
        let plugin = normalize_plugin(&raw_plugin);
        let mut behaviors = Vec::new();
        for b in varr(&o, &["behaviors"]) {
            let bid = normalize_behavior(&jstr(&b, &["behavior-id", "behaviorId", "id", "behavior", "name"], ""));
            let bname = jstr(&b, &["behavior-name", "name", "behavior"], "");
            behaviors.push(Behavior {
                id: bid.clone(),
                name: if bname.is_empty() { bid } else { bname },
                properties: value_map(b.get("properties")),
            });
        }
        let mut animations = Vec::new();
        for a in varr(&o, &["animations"]) {
            let mut frames = Vec::new();
            for f in varr(&a, &["frames"]) {
                let tx = frame_texture(&f, files, &mut assets, &mut aid, &mut warnings);
                frames.push(Frame { texture: tx, duration: jf64(&f, &["duration"], 1.0) });
            }
            animations.push(Animation {
                name: jstr(&a, &["name"], "Default"),
                speed: jf64(&a, &["speed"], 5.0),
                looping: jbool(&a, &["looping", "loop"], true),
                frames,
            });
        }
        let instance_vars = varr(&o, &["instance-variables", "instanceVariables"])
            .iter()
            .map(|iv| Variable {
                name: jstr(iv, &["name"], "var"),
                var_type: normalize_vtype(&jstr(iv, &["type", "variable-type", "variableType"], "number")),
                initial: iv
                    .get("initial")
                    .or_else(|| iv.get("default"))
                    .cloned()
                    .unwrap_or(Value::from(0)),
            })
            .collect();
        p.object_types.push(ObjectType {
            uid,
            name,
            plugin,
            is_global: jbool(&o, &["is-global", "isGlobal", "global"], false),
            behaviors,
            instance_vars,
            animations,
            properties: value_map(o.get("properties")),
            texture: String::new(),
        });
    }

    // ---- layouts
    let mut sheet_refs: Vec<(usize, Value)> = Vec::new();
    for (li, l) in varr(&v, &["layouts"]).iter().enumerate() {
        let name = jstr(l, &["name"], &format!("Layout {}", li + 1));
        let (mut lw, mut lh) = (p.width, p.height);
        if let Some(sz) = l.get("size") {
            let w = sz.get("width").or_else(|| sz.get(0));
            let h = sz.get("height").or_else(|| sz.get(1));
            if let Some(x) = w { let n = num(x); if !n.is_nan() { lw = n as u32; } }
            if let Some(x) = h { let n = num(x); if !n.is_nan() { lh = n as u32; } }
        }
        let bg = jstr(l, &["background-color", "backgroundColor", "background"], "#ffffff");
        let mut layers = Vec::new();
        let larr = varr(l, &["layers"]);
        if larr.is_empty() {
            let insts = parse_instances(varr(l, &["instances"]), &uid_to_type, files, &mut assets, &mut aid, &mut warnings);
            layers.push(Layer { name: "Layer 0".into(), instances: insts, ..Default::default() });
        } else {
            for ly in &larr {
                let insts = parse_instances(varr(ly, &["instances"]), &uid_to_type, files, &mut assets, &mut aid, &mut warnings);
                let (mut px, mut py) = (100.0, 100.0);
                if let Some(par) = ly.get("parallax").and_then(|x| x.as_array()) {
                    if par.len() >= 2 {
                        let a = num(&par[0]); let b = num(&par[1]);
                        if !a.is_nan() { px = a; }
                        if !b.is_nan() { py = b; }
                    }
                }
                let op = jf64(ly, &["opacity"], 100.0);
                layers.push(Layer {
                    name: jstr(ly, &["name"], "Layer"),
                    visible: jbool(ly, &["visible"], true),
                    opacity: if op <= 1.0 && op > 0.0 { op * 100.0 } else { op },
                    parallax_x: px,
                    parallax_y: py,
                    instances: insts,
                });
            }
        }
        let es_ref = l.get("event-sheet").or_else(|| l.get("eventSheet")).cloned();
        let layout = Layout {
            uid: jf64(l, &["uid"], li as f64 + 1.0) as i64,
            name: name.clone(),
            width: lw,
            height: lh,
            background: if bg.starts_with('#') { bg } else { "#ffffff".into() },
            layers,
            event_sheet: String::new(),
        };
        if let Some(r) = es_ref {
            sheet_refs.push((p.layouts.len(), r));
        }
        p.layouts.push(layout);
    }

    // ---- event sheets
    let mut sheet_uid_to_name: BTreeMap<i64, String> = BTreeMap::new();
    for (i, es) in varr(&v, &["event-sheets", "eventSheets"]).iter().enumerate() {
        let name = jstr(es, &["name"], &format!("Event sheet {}", i + 1));
        let uid = jf64(es, &["uid"], 0.0) as i64;
        sheet_uid_to_name.insert(uid, name.clone());
        let events = import_events(varr(es, &["events"]), &uid_to_type, &mut warnings);
        p.event_sheets.push(EventSheet { uid, name, events });
    }

    // résolution des références feuille d'événements des layouts
    for (idx, raw) in sheet_refs {
        let nm = match &raw {
            Value::Number(n) => sheet_uid_to_name.get(&(n.as_f64().unwrap_or(0.0) as i64)).cloned(),
            Value::String(s) => Some(s.clone()),
            _ => None,
        };
        if let Some(nm) = nm {
            if let Some(l) = p.layouts.get_mut(idx) {
                l.event_sheet = nm;
            }
        }
    }
    // layouts sans feuille → première feuille dispo
    let first_sheet = p.event_sheets.get(0).map(|s| s.name.clone()).unwrap_or_default();
    for l in &mut p.layouts {
        if l.event_sheet.is_empty() {
            l.event_sheet = first_sheet.clone();
        }
    }

    // ---- variables globales
    p.global_vars = varr(&v, &["global-variables", "globalVariables"])
        .iter()
        .map(|gv| Variable {
            name: jstr(gv, &["name"], "var"),
            var_type: normalize_vtype(&jstr(gv, &["type", "variable-type"], "number")),
            initial: gv.get("initial").cloned().unwrap_or(Value::from(0)),
        })
        .collect();

    p.assets = assets;
    p.first_layout = {
        let fl = jstr(&v, &["first-layout", "firstLayout"], "");
        if !fl.is_empty() && p.layouts.iter().any(|l| l.name == fl) {
            fl
        } else {
            p.layouts.get(0).map(|l| l.name.clone()).unwrap_or_default()
        }
    };
    // objet système implicite
    if !p.object_types.iter().any(|t| t.plugin == "system") {
        p.object_types.push(ObjectType {
            uid: -1,
            name: "System".into(),
            plugin: "system".into(),
            is_global: true,
            ..Default::default()
        });
    }
    warnings.insert(0, format!("Import C3 : {} objet(s), {} layout(s), {} feuille(s).", p.object_types.len(), p.layouts.len(), p.event_sheets.len()));
    ImportResult { project: p, warnings }
}

// ------------------------------------------------------------- exports

/// Sérialise le projet en .c3p (zip + project.json format natif c3r).
pub fn save_c3p(p: &Project) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec_pretty(p).map_err(|e| e.to_string())?;
    let buf = Cursor::new(Vec::new());
    let mut zw = ZipWriter::new(buf);
    zw.start_file("project.json", FileOptions::default()).map_err(|e| e.to_string())?;
    zw.write_all(&json).map_err(|e| e.to_string())?;
    let r = zw.finish().map_err(|e| e.to_string())?;
    Ok(r.into_inner())
}

/// Export HTML5 autonome : un seul fichier .html jouable hors-ligne.
pub fn export_html(p: &Project, player_js: &str) -> Result<Vec<u8>, String> {
    let mut json = serde_json::to_string(p).map_err(|e| e.to_string())?;
    json = json.replace("</", "<\\/");
    let title = html_escape(&p.name);
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"fr\"><head><meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\n<title>");
    html.push_str(&title);
    html.push_str("</title>\n<style>html,body{margin:0;height:100%;background:#111;display:flex;align-items:center;justify-content:center;overflow:hidden}canvas{max-width:100%;max-height:100%;box-shadow:0 0 40px #000;border-radius:4px}</style>\n</head><body>\n<script>\n");
    html.push_str(player_js);
    html.push_str("\nwindow.C3R_PROJECT = ");
    html.push_str(&json);
    html.push_str(";\nnew C3R_Runtime(window.C3R_PROJECT).start();\n</script>\n</body></html>\n");
    Ok(html.into_bytes())
}

/// Projet vierge avec objets globaux utiles.
pub fn default_project() -> Project {
    let mut p = Project::default();
    p.name = "Nouveau projet".into();
    p.width = 1280;
    p.height = 720;
    p.first_layout = "Layout 1".into();
    p.layouts.push(Layout {
        uid: 1,
        name: "Layout 1".into(),
        width: 1280,
        height: 720,
        background: "#ffffff".into(),
        layers: vec![Layer::default()],
        event_sheet: "Event sheet 1".into(),
    });
    p.event_sheets.push(EventSheet { uid: 2, name: "Event sheet 1".into(), events: vec![] });
    let globals = [("Keyboard", "keyboard"), ("Mouse", "mouse"), ("Audio", "audio")];
    for (i, (nm, plug)) in globals.iter().enumerate() {
        p.object_types.push(ObjectType {
            uid: 100 + i as i64,
            name: (*nm).into(),
            plugin: (*plug).into(),
            is_global: true,
            ..Default::default()
        });
    }
    p
}
