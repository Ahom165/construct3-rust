//! Modèle de données d'un projet .c3p + parsing JSON tolérant.
//! Format réel observé : `project.c3proj` / `project.json`, dossiers
//! `layouts\`, `eventSheets\`, `objectTypes\`, `families\`, `images\`.

use crate::expr::{eval_str, json_to_val, Env, Val};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plugin {
    Sprite,
    TiledBg,
    Text,
    Keyboard,
    Mouse,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BKind {
    Platform,
    Solid,
    EightDir,
    Bullet,
    Sine,
    ScrollTo,
    DestroyOutside,
    Fade,
    Timer,
    Wrap,
    Pin,
    Other,
}

impl BKind {
    pub fn from_name(s: &str) -> BKind {
        let l = s.to_lowercase();
        if l.contains("platform") {
            BKind::Platform
        } else if l == "solid" || l.contains("solid") {
            BKind::Solid
        } else if l.contains("8direction") || l.contains("eightdir") {
            BKind::EightDir
        } else if l.contains("bullet") {
            BKind::Bullet
        } else if l.contains("sine") {
            BKind::Sine
        } else if l.contains("scroll") {
            BKind::ScrollTo
        } else if l.contains("destroyoutside") {
            BKind::DestroyOutside
        } else if l.contains("fade") {
            BKind::Fade
        } else if l.contains("timer") {
            BKind::Timer
        } else if l.contains("wrap") {
            BKind::Wrap
        } else if l.contains("pin") {
            BKind::Pin
        } else {
            BKind::Other
        }
    }
    pub fn name(&self) -> &'static str {
        match self {
            BKind::Platform => "Platform",
            BKind::Solid => "Solid",
            BKind::EightDir => "8Direction",
            BKind::Bullet => "Bullet",
            BKind::Sine => "Sine",
            BKind::ScrollTo => "ScrollTo",
            BKind::DestroyOutside => "DestroyOutsideLayout",
            BKind::Fade => "Fade",
            BKind::Timer => "Timer",
            BKind::Wrap => "Wrap",
            BKind::Pin => "Pin",
            BKind::Other => "Other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tex {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BehSpec {
    pub name: String,
    pub kind: BKind,
    pub props: HashMap<String, Val>,
}

#[derive(Debug, Clone)]
pub struct Anim {
    pub name: String,
    pub frames: Vec<String>, // clés d'images (normalisées)
    pub speed: f32,
    pub looping: bool,
}

#[derive(Debug, Clone)]
pub struct ObjType {
    pub name: String,
    pub plugin: Plugin,
    pub is_family: bool,
    pub members: Vec<String>, // pour les familles
    pub vars: Vec<(String, Val)>,
    pub behaviors: Vec<BehSpec>,
    pub anims: Vec<Anim>,
    pub is_global: bool,
}

#[derive(Debug, Clone)]
pub struct InstSpec {
    pub type_name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub angle: f32,
    pub ox: f32,
    pub oy: f32,
    pub opacity: f32,
    pub layer: usize,
    pub vars: Vec<(String, Val)>,
    pub props: HashMap<String, Val>,
    pub behaviors: Vec<BehSpec>,
    pub uid: usize,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub parallax: (f32, f32),
    pub visible: bool,
    pub opacity: f32,
    pub instances: Vec<InstSpec>,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub background: Option<[f32; 4]>,
    pub layers: Vec<Layer>,
    pub event_sheet: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Cond {
    pub id: String,
    pub object: String, // "System" ou nom d'objet/famille
    pub params: HashMap<String, String>,
    pub sid: u64,
}

#[derive(Debug, Clone)]
pub struct Act {
    pub id: String,
    pub object: String,
    pub behavior: Option<String>,
    pub params: HashMap<String, String>,
    pub sid: u64,
}

#[derive(Debug, Clone)]
pub enum Event {
    Block {
        conditions: Vec<Cond>,
        actions: Vec<Act>,
        children: Vec<Event>,
        disabled: bool,
        sid: u64,
    },
    Else {
        conditions: Vec<Cond>,
        actions: Vec<Act>,
        children: Vec<Event>,
        disabled: bool,
        sid: u64,
    },
    Group {
        title: String,
        active: bool,
        children: Vec<Event>,
        disabled: bool,
    },
    Comment,
    Var {
        name: String,
        value: Val,
        is_constant: bool,
    },
    Include {
        sheet: String,
    },
    Function {
        name: String,
        children: Vec<Event>,
    },
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Sheet {
    pub name: String,
    pub events: Vec<Event>,
}

#[derive(Debug, Clone)]
pub struct Project {
    pub name: String,
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub first_layout: String,
    pub background: [f32; 4],
    pub layouts: Vec<Layout>,
    pub sheets: Vec<Sheet>,
    pub types: Vec<ObjType>,
    pub images: HashMap<String, Tex>,
}

impl Project {
    pub fn layout(&self, name: &str) -> Option<&Layout> {
        self.layouts.iter().find(|l| l.name == name)
    }
    pub fn type_of(&self, name: &str) -> Option<&ObjType> {
        self.types.iter().find(|t| t.name == name)
    }
    pub fn sheet(&self, name: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|s| s.name == name)
    }
    /// Membres concrets d'un type (famille -> membres, sinon lui-même)
    pub fn concrete_members(&self, name: &str) -> Vec<String> {
        match self.type_of(name) {
            Some(t) if t.is_family => t.members.clone(),
            _ => vec![name.to_string()],
        }
    }
    pub fn is_family(&self, name: &str) -> bool {
        self.type_of(name).map(|t| t.is_family).unwrap_or(false)
    }
}

// ---------------- helpers JSON ----------------

fn as_f64(v: &serde_json::Value, d: f64) -> f64 {
    match v {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(d),
        serde_json::Value::String(s) => s.trim().parse().unwrap_or(d),
        serde_json::Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => d,
    }
}

fn get<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<&'a serde_json::Value> {
    for k in keys {
        if let Some(o) = v.as_object() {
            if let Some(x) = o.get(*k) {
                if !x.is_null() {
                    return Some(x);
                }
            }
        }
    }
    None
}

fn parse_color4(v: &serde_json::Value) -> Option<[f32; 4]> {
    if let Some(s) = v.as_str() {
        return parse_hex_color(s);
    }
    if let Some(arr) = v.as_array() {
        if arr.len() >= 3 {
            let c: Vec<f32> = arr.iter().map(|x| as_f64(x, 1.0) as f32).collect();
            // format [r,g,b,a] en floats 0..1
            if c.iter().all(|x| *x <= 1.001) {
                return Some([c[0], c[1], c[2], *c.get(3).unwrap_or(&1.0)]);
            }
            // 0..255
            return Some([c[0] / 255.0, c[1] / 255.0, c[2] / 255.0, c.get(3).unwrap_or(&255.0) / 255.0]);
        }
    }
    None
}

pub fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let cs: Vec<char> = s.chars().collect();
    if cs.len() < 6 {
        return None;
    }
    let hx = |cs: &[char]| -> Option<f32> {
        if cs.len() < 2 {
            return None;
        }
        let a = u8::from_str_radix(&cs[0].to_string(), 16).ok()?;
        let b = u8::from_str_radix(&cs[1].to_string(), 16).ok()?;
        Some((a * 16 + b) as f32 / 255.0)
    };
    let r = hx(&cs[0..2])?;
    let g = hx(&cs[2..4])?;
    let b = hx(&cs[4..6])?;
    let a = if cs.len() >= 8 { hx(&cs[6..8])? } else { 1.0 };
    Some([r, g, b, a])
}

fn parse_plugin(s: &str) -> Plugin {
    match s.to_lowercase().as_str() {
        "sprite" => Plugin::Sprite,
        "tiledbg" | "tiledbackground" => Plugin::TiledBg,
        "text" | "spritefont2" | "spritefont" => Plugin::Text,
        "keyboard" => Plugin::Keyboard,
        "mouse" => Plugin::Mouse,
        _ => Plugin::Other,
    }
}

// ---------------- parsing projet ----------------

pub struct LoadWarn {
    pub warnings: Vec<String>,
}

pub fn parse_project_json(doc: &serde_json::Value, warnings: &mut Vec<String>) -> (String, f32, f32, String, [f32; 4]) {
    let name = get(doc, &["name", "applicationName", "projectName"])
        .and_then(|v| v.as_str())
        .unwrap_or("Projet")
        .to_string();
    let vw = get(doc, &["viewportWidth", "windowWidth", "width"]).map(|v| as_f64(v, 1280.0)).unwrap_or(1280.0) as f32;
    let vh = get(doc, &["viewportHeight", "windowHeight", "height"]).map(|v| as_f64(v, 720.0)).unwrap_or(720.0) as f32;
    let first = get(doc, &["firstLayout", "first-layout", "first_layout"])
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let bg = get(doc, &["properties"])
        .and_then(|p| p.as_object())
        .and_then(|p| p.get("backgroundColor"))
        .and_then(parse_color4)
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    (name, vw, vh, first, bg)
}

pub fn parse_layout(doc: &serde_json::Value, warnings: &mut Vec<String>) -> Layout {
    let name = get(doc, &["name"]).and_then(|v| v.as_str()).unwrap_or("Layout").to_string();
    let width = get(doc, &["width"]).map(|v| as_f64(v, 1920.0)).unwrap_or(1920.0) as f32;
    let height = get(doc, &["height"]).map(|v| as_f64(v, 1080.0)).unwrap_or(1080.0) as f32;
    let background = get(doc, &["backgroundColor", "background"]).and_then(parse_color4);
    let event_sheet = get(doc, &["eventSheet", "eventTable", "event_sheet", "event-sheet"])
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let mut layers = Vec::new();
    if let Some(ls) = get(doc, &["layers"]).and_then(|v| v.as_array()) {
        for l in ls {
            layers.push(parse_layer(l, warnings));
            // sous-couches aplaties (BFS)
            let mut queue: Vec<&serde_json::Value> = get(l, &["subLayers"])
                .and_then(|v| v.as_array())
                .map(|a| a.iter().collect())
                .unwrap_or_default();
            while !queue.is_empty() {
                let sl = queue.remove(0);
                layers.push(parse_layer(sl, warnings));
                if let Some(subs) = get(sl, &["subLayers"]).and_then(|v| v.as_array()) {
                    queue.extend(subs.iter());
                }
            }
        }
    }
    // chaque instance connaît l'index réel de sa couche
    for (li, layer) in layers.iter_mut().enumerate() {
        for inst in &mut layer.instances {
            inst.layer = li;
        }
    }
    Layout { name, width, height, background, layers, event_sheet }
}

fn parse_layer(doc: &serde_json::Value, warnings: &mut Vec<String>) -> Layer {
    let name = get(doc, &["name"]).and_then(|v| v.as_str()).unwrap_or("Couche").to_string();
    let parallax = (
        get(doc, &["parallaxX", "parallax-x", "parallax-x-rate"]).map(|v| as_f64(v, 100.0)).unwrap_or(100.0) as f32,
        get(doc, &["parallaxY", "parallax-y", "parallax-y-rate"]).map(|v| as_f64(v, 100.0)).unwrap_or(100.0) as f32,
    );
    let visible = get(doc, &["visible", "initially-visible"]).map(|v| as_f64(v, 1.0) != 0.0).unwrap_or(true);
    let opacity = get(doc, &["opacity", "layer-opacity"]).map(|v| as_f64(v, 100.0)).unwrap_or(100.0) as f32 / 100.0;
    let mut instances = Vec::new();
    if let Some(is) = get(doc, &["instances"]).and_then(|v| v.as_array()) {
        for inst in is.iter() {
            instances.push(parse_instance(inst, 0, warnings));
        }
    }
    Layer { name, parallax, visible, opacity, instances }
}

fn parse_instance(doc: &serde_json::Value, layer: usize, warnings: &mut Vec<String>) -> InstSpec {
    // type : chaîne directe, ou objet {name} / {id}
    let type_name = get(doc, &["type", "objectType", "object"])
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => get(other, &["name", "id"])
                .and_then(|n| n.as_str())
                .unwrap_or("Inconnu")
                .to_string(),
        })
        .unwrap_or_else(|| {
            warnings.push("instance sans type".into());
            "Inconnu".to_string()
        });
    let world = get(doc, &["world"]).cloned().unwrap_or(serde_json::json!({}));
    let x = get(&world, &["x"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let y = get(&world, &["y"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let w = get(&world, &["width", "w"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let h = get(&world, &["height", "h"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let angle = get(&world, &["angle"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let ox = get(&world, &["originX"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let oy = get(&world, &["originY"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
    let mut opacity = 1.0;
    if let Some(c) = get(&world, &["color"]).and_then(parse_color4) {
        opacity = c[3];
    }
    // fallback sans "world"
    let x = if get(&world, &["x"]).is_some() { x } else { get(doc, &["x"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32 };
    let y = if get(&world, &["y"]).is_some() { y } else { get(doc, &["y"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32 };
    let w = if w > 0.0 { w } else { get(doc, &["width"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32 };
    let h = if h > 0.0 { h } else { get(doc, &["height"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32 };
    let angle = if get(&world, &["angle"]).is_some() { angle } else { get(doc, &["angle"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32 };

    let mut vars = Vec::new();
    if let Some(m) = get(doc, &["instanceVariables", "variables"]).and_then(|v| v.as_object()) {
        for (k, v) in m {
            vars.push((k.clone(), json_to_val(v)));
        }
    }
    let mut props = HashMap::new();
    if let Some(m) = get(doc, &["properties", "props"]).and_then(|v| v.as_object()) {
        for (k, v) in m {
            let val = match v {
                serde_json::Value::String(s) => Val::Str(s.clone()),
                serde_json::Value::Bool(b) => Val::Bool(*b),
                serde_json::Value::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
                _ => continue,
            };
            props.insert(k.clone(), val);
        }
    }
    let mut behaviors = Vec::new();
    if let Some(m) = get(doc, &["behaviors"]).and_then(|v| v.as_object()) {
        for (k, v) in m {
            let kind = BKind::from_name(k);
            let mut bp = HashMap::new();
            if let Some(p) = get(v, &["properties"]).and_then(|x| x.as_object()) {
                for (pk, pv) in p {
                    let val = match pv {
                        serde_json::Value::String(s) => Val::Str(s.clone()),
                        serde_json::Value::Bool(b) => Val::Bool(*b),
                        serde_json::Value::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
                        _ => continue,
                    };
                    bp.insert(pk.clone(), val);
                }
            }
            behaviors.push(BehSpec { name: k.clone(), kind, props: bp });
        }
    } else if let Some(arr) = get(doc, &["behaviors"]).and_then(|v| v.as_array()) {
        for b in arr {
            let name = get(b, &["name", "behavior-plugin-id", "behaviorId", "id"])
                .and_then(|v| v.as_str())
                .unwrap_or("Inconnu")
                .to_string();
            let kind = BKind::from_name(&name);
            let mut bp = HashMap::new();
            if let Some(p) = get(b, &["properties"]).and_then(|x| x.as_object()) {
                for (pk, pv) in p {
                    let val = match pv {
                        serde_json::Value::String(s) => Val::Str(s.clone()),
                        serde_json::Value::Bool(b2) => Val::Bool(*b2),
                        serde_json::Value::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
                        _ => continue,
                    };
                    bp.insert(pk.clone(), val);
                }
            } else {
                // champs plats numériques
                if let Some(o) = b.as_object() {
                    for (pk, pv) in o {
                        if let Some(n) = pv.as_f64() {
                            bp.insert(pk.clone(), Val::Num(n));
                        }
                    }
                }
            }
            behaviors.push(BehSpec { name, kind, props: bp });
        }
    }
    let uid = get(doc, &["uid"]).and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    InstSpec { type_name, x, y, w, h, angle, ox, oy, opacity, layer, vars, props, behaviors, uid }
}

pub fn parse_object_type(doc: &serde_json::Value, warnings: &mut Vec<String>) -> ObjType {
    let name = get(doc, &["name"]).and_then(|v| v.as_str()).unwrap_or("Objet").to_string();
    let plugin_id = get(doc, &["plugin-id", "pluginId", "plugin"])
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => get(other, &["id", "name"])
                .and_then(|n| n.as_str())
                .unwrap_or("Sprite")
                .to_string(),
        })
        .unwrap_or_else(|| "Sprite".to_string());
    let is_family = get(doc, &["isFamily"]).and_then(|v| v.as_bool()).unwrap_or(false)
        || plugin_id.eq_ignore_ascii_case("family");
    let mut members = Vec::new();
    if let Some(ml) = get(doc, &["members"]).and_then(|v| v.as_array()) {
        for m in ml {
            match m {
                serde_json::Value::String(s) => members.push(s.clone()),
                other => {
                    if let Some(n) = get(other, &["name"]).and_then(|n| n.as_str()) {
                        members.push(n.to_string());
                    }
                }
            }
        }
    }
    let mut vars = Vec::new();
    if let Some(arr) = get(doc, &["instanceVariables", "variables"]).and_then(|v| v.as_array()) {
        for v in arr {
            let vname = get(v, &["name", "variable"]).and_then(|x| x.as_str()).unwrap_or("var").to_string();
            let vtype = get(v, &["type"]).and_then(|x| x.as_str()).unwrap_or("number").to_lowercase();
            let raw = get(v, &["initialValue", "value", "default"]).cloned().unwrap_or(serde_json::json!(0));
            let val = match vtype.as_str() {
                "string" => Val::Str(match &raw {
                    serde_json::Value::String(s) => s.clone(),
                    other => json_to_val(other).text(),
                }),
                "boolean" | "bool" => Val::Bool(match &raw {
                    serde_json::Value::Bool(b) => *b,
                    other => json_to_val(other).num() != 0.0,
                }),
                _ => Val::Num(match &raw {
                    serde_json::Value::String(s) => s.trim().parse().unwrap_or(0.0),
                    other => json_to_val(other).num(),
                }),
            };
            vars.push((vname, val));
        }
    }
    let mut behaviors = Vec::new();
    if let Some(arr) = get(doc, &["behaviorTypes", "behaviors"]).and_then(|v| v.as_array()) {
        for b in arr {
            let bname = get(b, &["name", "behavior-plugin-id", "behaviorId", "id"])
                .and_then(|v| v.as_str())
                .unwrap_or("Inconnu")
                .to_string();
            let kind = BKind::from_name(&bname);
            let mut bp = HashMap::new();
            if let Some(p) = get(b, &["properties"]).and_then(|x| x.as_object()) {
                for (pk, pv) in p {
                    let val = match pv {
                        serde_json::Value::String(s) => Val::Str(s.clone()),
                        serde_json::Value::Bool(b2) => Val::Bool(*b2),
                        serde_json::Value::Number(n) => Val::Num(n.as_f64().unwrap_or(0.0)),
                        _ => continue,
                    };
                    bp.insert(pk.clone(), val);
                }
            } else if let Some(o) = b.as_object() {
                for (pk, pv) in o {
                    if let Some(n) = pv.as_f64() {
                        bp.insert(pk.clone(), Val::Num(n));
                    }
                }
            }
            behaviors.push(BehSpec { name: bname, kind, props: bp });
        }
    }
    let mut anims = Vec::new();
    if let Some(animroot) = get(doc, &["animations"]) {
        let items = get(animroot, &["items", "animations"])
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_else(|| {
                if animroot.is_array() {
                    animroot.as_array().unwrap().clone()
                } else {
                    Vec::new()
                }
            });
        for a in &items {
            let aname = get(a, &["name"]).and_then(|v| v.as_str()).unwrap_or("Default").to_string();
            let speed = get(a, &["speed"]).map(|v| as_f64(v, 0.0)).unwrap_or(0.0) as f32;
            let looping = get(a, &["isLooping", "looping", "loop"]).map(|v| as_f64(v, 0.0) != 0.0).unwrap_or(false);
            let mut frames = Vec::new();
            if let Some(fs) = get(a, &["frames"]).and_then(|v| v.as_array()) {
                for f in fs {
                    let key = get(f, &["texture", "image", "file"])
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    frames.push(key); // résolu ensuite vers les images connues
                }
            }
            anims.push(Anim { name: aname, frames, speed, looping });
        }
    }
    let is_global = get(doc, &["isGlobal"]).and_then(|v| v.as_bool()).unwrap_or(false);
    let plugin = parse_plugin(&plugin_id);
    let mut anims = anims;
    if matches!(plugin, Plugin::TiledBg) && anims.is_empty() {
        // le TiledBg stocke sa texture dans properties/texture plutôt qu'en animation
        let tx = get(doc, &["texture"])
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        anims.push(Anim { name: "Default".into(), frames: vec![tx], speed: 0.0, looping: true });
    }
    ObjType { name, plugin, is_family, members, vars, behaviors, anims, is_global }
}

// ---------------- parsing événements ----------------

/// Normalise les ids de conditions/actions : snake_case (format c3r) -> kebab-case
/// (ids style C3 utilisés par le moteur natif), avec alias courts.
pub(crate) fn norm_event_id(raw: &str) -> String {
    let s = raw.trim().to_lowercase().replace('_', "-");
    match s.as_str() {
        "start-of-layout" | "on-layout-start" => "on-start-of-layout".into(),
        "tick" => "every-tick".into(),
        "every-x-sec" | "every-seconds" => "every-x-seconds".into(),
        "key-pressed" | "on-key-press" => "on-key-pressed".into(),
        "key-down" | "is-key-down" => "key-is-down".into(),
        "key-released" | "on-key-release" => "on-key-released".into(),
        "on-collision-with" | "collision-with" | "on-collision" => "on-collision-with-another-object".into(),
        "on-object-click" => "on-object-clicked".into(),
        "compare-instance-var" | "compare-instance-variable" => "compare-instance-variable".into(),
        "compare-global-var" | "compare-global-variable" | "compare-eventvar" => "compare-eventvar".into(),
        "is-overlapping" | "on-overlap" => "is-overlapping".into(),
        "is-on-screen" => "is-on-screen".into(),
        "set-instance-var" | "set-instvar" | "set-instance-variable" => "set-instvar-value".into(),
        "set-global-var" | "set-global-variable" | "set-eventvar" => "set-eventvar-value".into(),
        "add-to-global-var" | "add-to-eventvar" => "add-to-eventvar".into(),
        "subtract-from-global-var" => "subtract-from-eventvar".into(),
        "spawn" | "spawn-object" => "spawn-another-object".into(),
        "scroll-to" | "scrollto" => "scroll-to-object".into(),
        "go-to-layout" | "goto-layout" | "goto" => "go-to-layout".into(),
        "pin" | "pin-to" => "pin".into(),
        "set-animation-frame" | "set-frame" => "set-animation-frame".into(),
        "set-animation" => "set-animation".into(),
        "start-timer" => "start-timer".into(),
        "stop-timer" => "stop-timer".into(),
        "on-timer" => "on-timer".into(),
        "wait" => "wait-for-previous-actions".into(),
        other => other.into(),
    }
}

fn params_map(doc: &serde_json::Value, warnings: &mut Vec<String>) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let mut push = |out: &mut HashMap<String, String>, k: String, v: &serde_json::Value| match v {
        serde_json::Value::String(s) => {
            out.insert(k, s.clone());
        }
        serde_json::Value::Bool(b) => {
            out.insert(k, if *b { "true".into() } else { "false".into() });
        }
        serde_json::Value::Number(n) => {
            out.insert(k, n.to_string());
        }
        serde_json::Value::Null => {}
        other => {
            warnings.push(format!("paramètre complexe ignoré: {k}"));
            let _ = other;
        }
    };
    if let Some(m) = get(doc, &["parameters", "params"]).and_then(|v| v.as_object()) {
        for (k, v) in m {
            push(&mut out, k.clone(), v);
        }
    }
    // format positionnel historique
    if let Some(arr) = get(doc, &["parameters", "params"]).and_then(|v| v.as_array()) {
        for (i, v) in arr.iter().enumerate() {
            push(&mut out, format!("{i}"), v);
        }
    }
    out
}

fn parse_cond(doc: &serde_json::Value, warnings: &mut Vec<String>) -> Option<Cond> {
    let id = get(doc, &["id", "type", "name"])
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => get(other, &["name", "id"])
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
        })?;
    let id = norm_event_id(&id);
    if id.is_empty() {
        return None;
    }
    let object = get(doc, &["objectClass", "object", "objectType"])
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => get(other, &["name", "id"])
                .and_then(|n| n.as_str())
                .unwrap_or("System")
                .to_string(),
        })
        .unwrap_or_else(|| "System".to_string());
    let sid = get(doc, &["sid"]).and_then(|v| v.as_u64()).unwrap_or(0);
    Some(Cond { id, object, params: params_map(doc, warnings), sid })
}

fn parse_act(doc: &serde_json::Value, warnings: &mut Vec<String>) -> Option<Act> {
    let id = get(doc, &["id", "type", "name"])
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => get(other, &["name", "id"])
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string(),
        })?;
    let id = norm_event_id(&id);
    if id.is_empty() {
        return None;
    }
    let object = get(doc, &["objectClass", "object", "objectType"])
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => get(other, &["name", "id"])
                .and_then(|n| n.as_str())
                .unwrap_or("System")
                .to_string(),
        })
        .unwrap_or_else(|| "System".to_string());
    let behavior = get(doc, &["behavior"])
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let sid = get(doc, &["sid"]).and_then(|v| v.as_u64()).unwrap_or(0);
    Some(Act { id, object, behavior, params: params_map(doc, warnings), sid })
}

pub fn parse_event(doc: &serde_json::Value, warnings: &mut Vec<String>) -> Event {
    let et = get(doc, &["eventType", "type"])
        .and_then(|v| v.as_str())
        .unwrap_or("block")
        .to_lowercase();
    let disabled = get(doc, &["disabled"]).and_then(|v| v.as_bool()).unwrap_or(false);
    let sid = get(doc, &["sid"]).and_then(|v| v.as_u64()).unwrap_or(0);
    let children_v: Vec<Event> = get(doc, &["children", "subEvents", "sub-events"])
        .and_then(|v| v.as_array())
        .map(|a| a.iter().map(|e| parse_event(e, warnings)).collect())
        .unwrap_or_default();
    let conditions_v: Vec<Cond> = get(doc, &["conditions"])
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| parse_cond(c, warnings)).collect())
        .unwrap_or_default();
    let actions_v: Vec<Act> = get(doc, &["actions"])
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|c| parse_act(c, warnings)).collect())
        .unwrap_or_default();
    match et.as_str() {
        "block" => Event::Block { conditions: conditions_v, actions: actions_v, children: children_v, disabled, sid },
        "else" => Event::Else { conditions: conditions_v, actions: actions_v, children: children_v, disabled, sid },
        "group" => {
            let title = get(doc, &["title", "name"]).and_then(|v| v.as_str()).unwrap_or("Groupe").to_string();
            let active = get(doc, &["isActiveOnStart", "active"]).map(|v| as_f64(v, 1.0) != 0.0).unwrap_or(true);
            Event::Group { title, active, children: children_v, disabled }
        }
        "comment" => Event::Comment,
        "variable" => {
            let name = get(doc, &["name"]).and_then(|v| v.as_str()).unwrap_or("var").to_string();
            let vtype = get(doc, &["type"]).and_then(|v| v.as_str()).unwrap_or("number").to_lowercase();
            let raw = get(doc, &["initialValue", "value"]).and_then(|v| v.as_str()).unwrap_or("0").to_string();
            let is_constant = get(doc, &["isConstant"]).and_then(|v| v.as_bool()).unwrap_or(false);
            let val = match vtype.as_str() {
                "string" => {
                    // les chaînes sont stockées avec guillemets dans C3
                    let t = raw.trim();
                    if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
                        || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
                    {
                        Val::Str(t[1..t.len() - 1].to_string())
                    } else {
                        Val::Str(raw.clone())
                    }
                }
                "boolean" | "bool" => Val::Bool(raw.trim().parse::<f64>().map(|n| n != 0.0).unwrap_or(false)),
                _ => {
                    // valeur numérique = expression
                    struct NullEnv;
                    impl Env for NullEnv {}
                    let v = eval_str(&raw, &mut NullEnv);
                    Val::Num(v.num())
                }
            };
            Event::Var { name, value: val, is_constant }
        }
        "include" => {
            let sheet = get(doc, &["includeSheet", "sheet", "eventSheet"])
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Event::Include { sheet }
        }
        "function-block" | "function" => {
            let fname = get(doc, &["name", "functionName", "title"])
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            Event::Function { name: fname, children: children_v }
        }
        _ => {
            warnings.push(format!("type d'événement ignoré: {et}"));
            Event::Unknown
        }
    }
}

pub fn parse_sheet(name: &str, doc: &serde_json::Value, warnings: &mut Vec<String>) -> Sheet {
    let mut events = Vec::new();
    if let Some(arr) = get(doc, &["events"]).and_then(|v| v.as_array()) {
        for e in arr {
            events.push(parse_event(e, warnings));
        }
    }
    Sheet { name: name.to_string(), events }
}
