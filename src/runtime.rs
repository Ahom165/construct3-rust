//! Moteur d'exécution : instances, comportements, interpréteur d'événements.

use crate::c3p::{placeholder_tex, prop_f};
use crate::expr::{eval, eval_str, parse, Env, Expr, Val};
use crate::model::*;
use crate::render::{FrameBuf, Rgba};
use std::collections::{HashMap, HashSet};

pub const KEY_LEFT: u32 = 37;
pub const KEY_UP: u32 = 38;
pub const KEY_RIGHT: u32 = 39;
pub const KEY_DOWN: u32 = 40;
pub const KEY_SPACE: u32 = 32;
pub const KEY_ENTER: u32 = 13;
pub const KEY_SHIFT: u32 = 16;
pub const KEY_CTRL: u32 = 17;
pub const KEY_ESC: u32 = 27;

pub fn parse_key(s: &str) -> u32 {
    let t = s.trim().to_lowercase();
    match t.as_str() {
        "left" | "arrowleft" => KEY_LEFT,
        "up" | "arrowup" => KEY_UP,
        "right" | "arrowright" => KEY_RIGHT,
        "down" | "arrowdown" => KEY_DOWN,
        "space" | "spacebar" => KEY_SPACE,
        "enter" | "return" => KEY_ENTER,
        "shift" => KEY_SHIFT,
        "control" | "ctrl" => KEY_CTRL,
        "escape" | "esc" => KEY_ESC,
        _ => {
            if let Ok(n) = t.parse::<u32>() {
                return n;
            }
            let mut it = t.chars();
            if let Some(c) = it.next() {
                let cu = c.to_ascii_uppercase();
                if cu.is_ascii_uppercase() {
                    return cu as u32;
                }
                if cu.is_ascii_digit() {
                    return cu as u32;
                }
            }
            0
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
    pub down: HashSet<u32>,
    pub pressed: HashSet<u32>,
    pub released: HashSet<u32>,
    pub mouse: (f32, f32),
    pub mouse_down: HashSet<u8>,
    pub mouse_pressed: HashSet<u8>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Ctrl {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
}

#[derive(Debug, Clone)]
pub enum Beh {
    Platform { vx: f32, vy: f32, on_floor: bool },
    EightDir { vx: f32, vy: f32 },
    Bullet { speed: f32 },
    Sine { base_x: f32, base_y: f32, t: f32 },
    ScrollTo,
    DestroyOutside,
    Fade { phase: u8, t: f32 },
    Timer,
    Wrap,
    Solid,
    Pin { target: Option<usize> },
    Other,
}

#[derive(Debug, Clone)]
pub struct BehState {
    pub name: String,
    pub kind: BKind,
    pub enabled: bool,
    pub props: HashMap<String, Val>,
    pub state: Beh,
}

#[derive(Debug, Clone)]
pub struct Inst {
    pub id: usize,
    pub uid: usize,
    pub type_name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub angle: f32,
    pub opacity: f32,
    pub layer: usize,
    pub visible: bool,
    pub ox: f32,
    pub oy: f32,
    pub z: f32,
    pub vars: HashMap<String, Val>,
    pub props: HashMap<String, Val>,
    pub behaviors: Vec<BehState>,
    pub anim_idx: usize,
    pub frame: usize,
    pub anim_t: f32,
    pub anim_speed: f32,
    pub mirrored: bool,
    pub destroyed: bool,
    pub tint: Option<Rgba>,
    pub prev_x: f32,
    pub prev_y: f32,
}

impl Inst {
    pub fn bbox(&self) -> (f32, f32, f32, f32) {
        let l = self.x - self.ox * self.w;
        let t = self.y - self.oy * self.h;
        (l, t, l + self.w, t + self.h)
    }
    pub fn cx(&self) -> f32 {
        let (l, _, r, _) = self.bbox();
        (l + r) / 2.0
    }
    pub fn cy(&self) -> f32 {
        let (_, t, _, b) = self.bbox();
        (t + b) / 2.0
    }
}

pub type Picked = HashMap<String, Vec<usize>>;

pub enum PendingNav {
    Restart,
    Layout(String),
}

pub struct Engine {
    pub project: Project,
    pub layout: Layout,
    pub instances: Vec<Inst>,
    pub next_id: usize,
    pub globals: HashMap<String, Val>,
    pub globals_init: HashMap<String, Val>,
    pub locals: HashMap<String, Val>,
    pub scroll: (f32, f32),
    pub view_w: f32,
    pub view_h: f32,
    pub time: f32,
    pub dt: f32,
    pub time_scale: f32,
    pub input: InputState,
    pub warnings: Vec<String>,
    warned: HashSet<String>,
    pub logs: Vec<String>,
    rng_state: u64,
    block_timers: HashMap<u64, f32>,
    last_collisions: HashSet<(usize, usize)>,
    current_pairs: HashSet<(usize, usize)>,
    pub landed: HashSet<usize>,
    pub anim_finished: HashSet<usize>,
    timer_fired: HashSet<(usize, String)>,
    pub timers: HashMap<(usize, String), f32>,
    pub loops: Vec<(String, f64)>,
    pub sim_ctrl: HashMap<usize, Ctrl>,
    pub start_flag: bool,
    pub pending: Option<PendingNav>,
    sheet_events: Vec<Event>,
    pub scroll_lock: bool,
}

fn normkey(s: &str) -> String {
    s.to_lowercase().replace(['-', '_', ' '], "")
}

impl Engine {
    pub fn new(project: Project, layout_name: &str) -> Result<Engine, String> {
        let layout_name = if layout_name.is_empty() { project.first_layout.clone() } else { layout_name.to_string() };
        let layout = project
            .layout(&layout_name)
            .cloned()
            .ok_or_else(|| format!("layout « {layout_name} » introuvable"))?;
        let mut eng = Engine {
            view_w: project.viewport_w,
            view_h: project.viewport_h,
            project,
            layout,
            instances: Vec::new(),
            next_id: 0,
            globals: HashMap::new(),
            globals_init: HashMap::new(),
            locals: HashMap::new(),
            scroll: (0.0, 0.0),
            time: 0.0,
            dt: 1.0 / 60.0,
            time_scale: 1.0,
            input: InputState::default(),
            warnings: Vec::new(),
            warned: HashSet::new(),
            logs: Vec::new(),
            rng_state: 0x2545F4914F6CDD1D,
            block_timers: HashMap::new(),
            last_collisions: HashSet::new(),
            current_pairs: HashSet::new(),
            landed: HashSet::new(),
            anim_finished: HashSet::new(),
            timer_fired: HashSet::new(),
            timers: HashMap::new(),
            loops: Vec::new(),
            sim_ctrl: HashMap::new(),
            start_flag: true,
            pending: None,
            sheet_events: Vec::new(),
            scroll_lock: false,
        };
        eng.seed_rng();
        eng.collect_globals();
        eng.build_instances();
        eng.load_locals();
        eng.resolve_sheet();
        Ok(eng)
    }

    fn seed_rng(&mut self) {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
            .unwrap_or(42);
        self.rng_state ^= t | 1;
    }

    pub fn rand01(&mut self) -> f64 {
        // xorshift64*
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        let v = x.wrapping_mul(0x2545F4914F6CDD1D);
        ((v >> 11) as f64) / ((1u64 << 53) as f64)
    }

    pub fn warn(&mut self, msg: &str) {
        if self.warnings.len() < 300 && self.warned.insert(msg.to_string()) {
            self.warnings.push(msg.to_string());
        }
    }

    fn collect_globals(&mut self) {
        // variables racine de toutes les feuilles = variables globales C3
        for sheet in &self.project.sheets {
            for ev in &sheet.events {
                if let Event::Var { name, value, is_constant } = ev {
                    self.globals.entry(name.clone()).or_insert(value.clone());
                    if *is_constant {
                        self.globals_init.entry(name.clone()).or_insert(value.clone());
                    }
                }
            }
        }
        self.globals_init = self.globals.clone();
    }

    fn build_instances(&mut self) {
        self.instances.clear();
        self.next_id = 0;
        let layers: Vec<Layer> = self.layout.layers.clone();
        for (li, layer) in layers.iter().enumerate() {
            for spec in &layer.instances {
                let tname = spec.type_name.clone();
                if let Some(t) = self.project.type_of(&tname) {
                    if matches!(t.plugin, Plugin::Keyboard | Plugin::Mouse) {
                        continue; // objets non-world
                    }
                }
                let inst = self.make_instance(spec, li);
                self.instances.push(inst);
                self.next_id += 1;
            }
        }
    }

    fn make_instance(&mut self, spec: &InstSpec, layer: usize) -> Inst {
        let tname = spec.type_name.clone();
        let tinfo = self.project.type_of(&tname).cloned();
        let mut vars: HashMap<String, Val> = HashMap::new();
        if let Some(t) = &tinfo {
            for (k, v) in &t.vars {
                vars.insert(k.clone(), v.clone());
            }
        }
        for (k, v) in &spec.vars {
            vars.insert(k.clone(), v.clone());
        }
        let mut props = spec.props.clone();
        // Taille par défaut depuis la texture de la première animation
        let mut w = spec.w;
        let mut h = spec.h;
        if w <= 0.0 || h <= 0.0 {
            if let Some(t) = &tinfo {
                if let Some(a) = t.anims.first() {
                    if let Some(frame_key) = a.frames.first() {
                        if let Some(tex) = self.project.images.get(frame_key) {
                            if w <= 0.0 {
                                w = tex.w as f32;
                            }
                            if h <= 0.0 {
                                h = tex.h as f32;
                            }
                        }
                    }
                }
            }
        }
        // comportements : défauts du type + overrides d'instance
        let mut behaviors: Vec<BehState> = Vec::new();
        let mut push_beh = |name: &str, kind: BKind, props: &HashMap<String, Val>, out: &mut Vec<BehState>| {
            let st = match kind {
                BKind::Platform => Beh::Platform { vx: 0.0, vy: 0.0, on_floor: false },
                BKind::EightDir => Beh::EightDir { vx: 0.0, vy: 0.0 },
                BKind::Bullet => Beh::Bullet { speed: prop_f(props, &["speed", "initial-speed"], 300.0) as f32 },
                BKind::Sine => Beh::Sine { base_x: 0.0, base_y: 0.0, t: 0.0 },
                BKind::ScrollTo => Beh::ScrollTo,
                BKind::DestroyOutside => Beh::DestroyOutside,
                BKind::Fade => Beh::Fade { phase: 0, t: 0.0 },
                BKind::Timer => Beh::Timer,
                BKind::Wrap => Beh::Wrap,
                BKind::Solid => Beh::Solid,
                BKind::Pin => Beh::Pin { target: None },
                BKind::Other => Beh::Other,
            };
            if let Some(b) = out.iter_mut().find(|b| normkey(&b.name) == normkey(name)) {
                b.props = props.clone();
                b.kind = kind;
            } else {
                out.push(BehState { name: name.to_string(), kind, enabled: true, props: props.clone(), state: st });
            }
        };
        if let Some(t) = &tinfo {
            for b in &t.behaviors {
                push_beh(&b.name, b.kind, &b.props, &mut behaviors);
            }
        }
        for b in &spec.behaviors {
            push_beh(&b.name, b.kind, &b.props, &mut behaviors);
        }
        let enabled = prop_f(&props, &["initially-visible", "visible"], 1.0);
        let anim_speed = tinfo.as_ref().and_then(|t| t.anims.first()).map(|a| a.speed).unwrap_or(0.0);
        let opacity = spec.opacity;
        let mut inst = Inst {
            id: self.next_id,
            uid: spec.uid,
            type_name: tname.clone(),
            x: spec.x,
            y: spec.y,
            w,
            h,
            angle: spec.angle,
            opacity: if opacity <= 0.0 { 1.0 } else { opacity },
            layer,
            visible: enabled != 0.0,
            ox: spec.ox,
            oy: spec.oy,
            z: 0.0,
            vars,
            props,
            behaviors,
            anim_idx: 0,
            frame: 0,
            anim_t: 0.0,
            anim_speed,
            mirrored: false,
            destroyed: false,
            tint: None,
            prev_x: spec.x,
            prev_y: spec.y,
        };
        for b in &mut inst.behaviors {
            if let Beh::Sine { base_x, base_y, .. } = &mut b.state {
                *base_x = inst.x;
                *base_y = inst.y;
            }
        }
        // couleur du texte par défaut
        if tinfo.map(|t| t.plugin == Plugin::Text).unwrap_or(false) && !inst.props.contains_key("color") {
            inst.props.insert("color".into(), Val::Str("#FFFFFFFF".into()));
        }
        inst
    }

    fn collect_vars(events: &[Event], root: bool, globals: &mut HashMap<String, Val>, locals: &mut Vec<(String, Val)>) {
        for ev in events {
            match ev {
                Event::Var { name, value, .. } => {
                    if root {
                        globals.entry(name.clone()).or_insert(value.clone());
                    } else {
                        locals.push((name.clone(), value.clone()));
                    }
                }
                Event::Block { children, .. } | Event::Group { children, .. } | Event::Else { children, .. }
                | Event::Function { children, .. } => {
                    Self::collect_vars(children, false, globals, locals);
                }
                _ => {}
            }
        }
    }

    fn load_locals(&mut self) {
        self.locals.clear();
        let mut g = HashMap::new();
        let mut l: Vec<(String, Val)> = Vec::new();
        Self::collect_vars(&self.sheet_events.clone(), true, &mut g, &mut l);
        let _ = g;
        for (k, v) in l {
            self.locals.insert(k, v);
        }
    }

    fn resolve_sheet(&mut self) {
        self.sheet_events.clear();
        let sheet_name = self.layout.event_sheet.clone().unwrap_or_default();
        let mut queue: Vec<String> = if sheet_name.is_empty() {
            vec![]
        } else {
            vec![sheet_name]
        };
        let mut visited: HashSet<String> = HashSet::new();
        while let Some(sname) = queue.pop() {
            if !visited.insert(sname.clone()) {
                continue;
            }
            if let Some(sheet) = self.project.sheet(&sname) {
                let mut copy = sheet.events.clone();
                // les includes sont ajoutés à la fin
                for ev in &copy.clone() {
                    if let Event::Include { sheet } = ev {
                        queue.push(sheet.clone());
                    }
                }
                copy.retain(|e| !matches!(e, Event::Include { .. }));
                self.sheet_events.extend(copy);
            } else if !sname.is_empty() {
                self.warn(&format!("feuille d'événements « {sname} » introuvable"));
            }
        }
    }

    pub fn inst(&self, id: usize) -> Option<&Inst> {
        self.instances.iter().find(|i| i.id == id)
    }
    pub fn inst_mut(&mut self, id: usize) -> Option<&mut Inst> {
        self.instances.iter_mut().find(|i| i.id == id)
    }

    pub fn instances_of(&self, type_name: &str) -> Vec<usize> {
        let members = self.project.concrete_members(type_name);
        self.instances
            .iter()
            .filter(|i| !i.destroyed && members.iter().any(|m| m == &i.type_name))
            .map(|i| i.id)
            .collect()
    }

    fn candidates(&self, picked: &Picked, name: &str) -> Vec<usize> {
        if let Some(list) = picked.get(name) {
            return list.clone();
        }
        // famille déjà résolue via un membre ?
        if self.project.is_family(name) {
            let members = self.project.concrete_members(name);
            let mut all: Vec<usize> = Vec::new();
            for m in &members {
                if let Some(l) = picked.get(m) {
                    all.extend(l.iter().cloned());
                }
            }
            if !all.is_empty() {
                return all;
            }
        }
        self.instances_of(name)
    }

    fn ensure_picked(&self, picked: &mut Picked, name: &str) {
        if !picked.contains_key(name) {
            picked.insert(name.to_string(), self.instances_of(name));
        }
    }

    // ---------------- tick ----------------

    pub fn tick(&mut self, dt: f32) {
        let dt = dt * self.time_scale;
        self.dt = dt;
        self.time += dt;
        self.sim_ctrl.clear();
        self.landed.clear();
        self.anim_finished.clear();
        self.timer_fired.clear();

        self.update_behaviors(dt);
        self.run_events();
        self.update_animations(dt);
        self.update_timers();
        self.cleanup();

        // navigation
        if let Some(nav) = self.pending.take() {
            match nav {
                PendingNav::Restart => self.reload_layout(self.layout.name.clone()),
                PendingNav::Layout(name) => self.reload_layout(name),
            }
        }
        self.update_camera();
        self.last_collisions = std::mem::take(&mut self.current_pairs);
        self.input.pressed.clear();
        self.input.released.clear();
        self.input.mouse_pressed.clear();
    }

    fn reload_layout(&mut self, name: String) {
        if let Some(l) = self.project.layout(&name).cloned() {
            self.layout = l;
        } else {
            self.warn(&format!("layout « {name} » introuvable"));
            return;
        }
        self.scroll = (0.0, 0.0);
        self.time = 0.0;
        self.time_scale = 1.0;
        self.block_timers.clear();
        self.last_collisions.clear();
        self.landed.clear();
        self.anim_finished.clear();
        self.timer_fired.clear();
        self.timers.clear();
        self.sim_ctrl.clear();
        self.start_flag = true;
        self.scroll_lock = false;
        self.build_instances();
        self.load_locals();
        self.resolve_sheet();
    }

    // ---------------- comportements ----------------

    fn ctrl_for(&self, id: usize, kind: BKind) -> Ctrl {
        let mut c = Ctrl::default();
        let d = &self.input.down;
        let (l, r, u, dn, j): (Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>, Vec<u32>) = match kind {
            BKind::Platform => (
                vec![KEY_LEFT, 65],
                vec![KEY_RIGHT, 68],
                vec![],
                vec![],
                vec![KEY_SPACE, KEY_UP, 87],
            ),
            BKind::EightDir => (
                vec![KEY_LEFT, 65],
                vec![KEY_RIGHT, 68],
                vec![KEY_UP, 87],
                vec![KEY_DOWN, 83],
                vec![],
            ),
            _ => (vec![], vec![], vec![], vec![], vec![]),
        };
        if l.iter().any(|k| d.contains(k)) {
            c.left = true;
        }
        if r.iter().any(|k| d.contains(k)) {
            c.right = true;
        }
        if u.iter().any(|k| d.contains(k)) {
            c.up = true;
        }
        if dn.iter().any(|k| d.contains(k)) {
            c.down = true;
        }
        if j.iter().any(|k| d.contains(k)) {
            c.jump = true;
        }
        if let Some(sim) = self.sim_ctrl.get(&id) {
            if sim.left {
                c.left = true;
            }
            if sim.right {
                c.right = true;
            }
            if sim.up {
                c.up = true;
            }
            if sim.down {
                c.down = true;
            }
            if sim.jump {
                c.jump = true;
            }
        }
        c
    }

    fn solids(&self) -> Vec<(f32, f32, f32, f32)> {
        self.instances
            .iter()
            .filter(|i| {
                !i.destroyed
                    && i.visible
                    && i.behaviors
                        .iter()
                        .any(|b| b.kind == BKind::Solid && b.enabled)
            })
            .map(|i| i.bbox())
            .collect()
    }

fn move_x(inst: &mut Inst, dx: f32, solids: &[(f32, f32, f32, f32)]) {
    if dx == 0.0 {
        return;
    }
    inst.x += dx;
    let (mut l, t, mut r, b) = inst.bbox();
    for &(sl, st, sr, sb) in solids {
        if l < sr && r > sl && t < sb && b > st {
            if dx > 0.0 {
                let new_l = sl - inst.w;
                inst.x = new_l + inst.ox * inst.w;
            } else {
                let new_l = sr;
                inst.x = new_l + inst.ox * inst.w;
            }
            let (nl, _, nr, _) = inst.bbox();
            l = nl;
            r = nr;
        }
    }
}

fn move_y(inst: &mut Inst, dy: f32, solids: &[(f32, f32, f32, f32)]) -> u8 {
    // retourne 1 = sol touché, 2 = plafond touché
    if dy == 0.0 {
        return 0;
    }
    inst.y += dy;
    let (l, mut t, r, mut b) = inst.bbox();
    let mut hit = 0u8;
    for &(sl, st, sr, sb) in solids {
        if l < sr && r > sl && t < sb && b > st {
            if dy > 0.0 {
                inst.y = st - inst.h * (1.0 - inst.oy);
                hit |= 1;
            } else {
                inst.y = sb + inst.oy * inst.h;
                hit |= 2;
            }
            let (_, nt, _, nb) = inst.bbox();
            t = nt;
            b = nb;
        }
    }
    hit
}

    fn update_behaviors(&mut self, dt: f32) {
        let ids: Vec<usize> = self.instances.iter().map(|i| i.id).collect();
        let solids = self.solids();
        for id in ids {
            let behs: Vec<(String, BKind)> = match self.inst(id) {
                Some(i) => i
                    .behaviors
                    .iter()
                    .filter(|b| b.enabled)
                    .map(|b| (b.name.clone(), b.kind))
                    .collect(),
                None => continue,
            };
            for (bname, kind) in behs {
                match kind {
                    BKind::Platform => self.update_platform(id, &bname, dt, &solids),
                    BKind::EightDir => self.update_eightdir(id, &bname, dt, &solids),
                    BKind::Bullet => self.update_bullet(id, &bname, dt, &solids),
                    BKind::Sine => self.update_sine(id, &bname, dt),
                    BKind::Fade => self.update_fade(id, &bname, dt),
                    BKind::Wrap => self.update_wrap(id, dt),
                    BKind::DestroyOutside => self.update_destroy_outside(id),
                    _ => {}
                }
            }
        }
        // pins après tous les déplacements
        let pins: Vec<usize> = self
            .instances
            .iter()
            .filter(|i| i.behaviors.iter().any(|b| matches!(b.state, Beh::Pin { .. }) && b.enabled))
            .map(|i| i.id)
            .collect();
        for id in pins {
            let (target, tx, ty) = match self.inst(id).and_then(|i| {
                i.behaviors.iter().find_map(|b| match &b.state {
                    Beh::Pin { target } => target.map(|t| (t, 0.0f32, 0.0f32)),
                    _ => None,
                })
            }) {
                Some((t, _, _)) => match self.inst(t) {
                    Some(ti) => (t, ti.x, ti.y),
                    None => continue,
                },
                None => continue,
            };
            let tpx = self.instances.iter().find(|i| i.id == target).map(|i| i.prev_x).unwrap_or(tx);
            let tpy = self.instances.iter().find(|i| i.id == target).map(|i| i.prev_y).unwrap_or(ty);
            let (dx, dy) = (tx - tpx, ty - tpy);
            if let Some(i) = self.inst_mut(id) {
                i.x += dx;
                i.y += dy;
            }
        }
        // mémorise les positions précédentes pour le tick suivant
        for i in &mut self.instances {
            i.prev_x = i.x;
            i.prev_y = i.y;
        }
    }

    fn update_platform(&mut self, id: usize, bname: &str, dt: f32, solids: &[(f32, f32, f32, f32)]) {
        let p = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => b.props.clone(),
            None => return,
        };
        let max_speed = prop_f(&p, &["max-speed", "maxspeed"], 330.0) as f32;
        let accel = prop_f(&p, &["acceleration"], 1500.0) as f32;
        let decel = prop_f(&p, &["deceleration"], 1500.0) as f32;
        let jump = prop_f(&p, &["jump-strength", "jumpstrength"], 1000.0) as f32;
        let gravity = prop_f(&p, &["gravity"], 1500.0) as f32;
        let max_fall = prop_f(&p, &["maximum-fall-speed", "maxfallspeed"], 1200.0) as f32;
        let use_default = prop_f(&p, &["default-controls", "defaultcontrols"], 1.0) != 0.0;
        let ctrl = if use_default { self.ctrl_for(id, BKind::Platform) } else { self.sim_ctrl.get(&id).copied().unwrap_or_default() };

        let dir = (ctrl.right as i32 - ctrl.left as i32) as f32;
        let target = dir * max_speed;
        let rate = if dir != 0.0 { accel } else { decel };
        let (on_floor, mut vx, mut vy) = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => match b.state {
                Beh::Platform { vx, vy, on_floor } => (on_floor, vx, vy),
                _ => return,
            },
            None => return,
        };
        if target > vx {
            vx = (vx + rate * dt).min(target);
        } else if target < vx {
            vx = (vx - rate * dt).max(target);
        }
        let jump_now = ctrl.jump && on_floor;
        vy += gravity * dt;
        vy = vy.min(max_fall);
        if jump_now {
            vy = -jump;
        }
        {
            let solids2 = solids;
            if let Some(mut i) = self.instances.iter_mut().find(|x| x.id == id) {
                Self::move_x(&mut i, vx * dt, solids2);
                let dy = vy * dt;
                let hit = Self::move_y(&mut i, dy, solids2);
                let on_floor_now = hit & 1 != 0;
                if hit & 2 != 0 {
                    vy = 0.0;
                }
                if on_floor_now {
                    if !on_floor {
                        self.landed.insert(id);
                    }
                }
                let final_vy = if on_floor_now { vy.min(0.0) } else { vy };
                if let Some(b) = i.behaviors.iter_mut().find(|b| b.name == bname) {
                    b.state = Beh::Platform { vx, vy: final_vy, on_floor: on_floor_now };
                }
            }
        }
    }

    fn update_eightdir(&mut self, id: usize, bname: &str, dt: f32, solids: &[(f32, f32, f32, f32)]) {
        let p = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => b.props.clone(),
            None => return,
        };
        let max_speed = prop_f(&p, &["max-speed", "maxspeed"], 200.0) as f32;
        let accel = prop_f(&p, &["acceleration"], 1200.0) as f32;
        let decel = prop_f(&p, &["deceleration"], 1200.0) as f32;
        let use_default = prop_f(&p, &["default-controls", "defaultcontrols"], 1.0) != 0.0;
        let ctrl = if use_default { self.ctrl_for(id, BKind::EightDir) } else { self.sim_ctrl.get(&id).copied().unwrap_or_default() };
        let dx = (ctrl.right as i32 - ctrl.left as i32) as f32;
        let dy = (ctrl.down as i32 - ctrl.up as i32) as f32;
        let (mut vx, mut vy) = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => match b.state {
                Beh::EightDir { vx, vy } => (vx, vy),
                _ => return,
            },
            None => return,
        };
        let (tx, ty) = (dx * max_speed, dy * max_speed);
        let rate = if dx != 0.0 { accel } else { decel };
        if tx > vx {
            vx = (vx + rate * dt).min(tx);
        } else if tx < vx {
            vx = (vx - rate * dt).max(tx);
        }
        let rate = if dy != 0.0 { accel } else { decel };
        if ty > vy {
            vy = (vy + rate * dt).min(ty);
        } else if ty < vy {
            vy = (vy - rate * dt).max(ty);
        }
        if let Some(mut i) = self.instances.iter_mut().find(|x| x.id == id) {
            Self::move_x(&mut i, vx * dt, solids);
            Self::move_y(&mut i, vy * dt, solids);
            if let Some(b) = i.behaviors.iter_mut().find(|b| b.name == bname) {
                b.state = Beh::EightDir { vx, vy };
            }
        }
    }

    fn update_bullet(&mut self, id: usize, bname: &str, dt: f32, solids: &[(f32, f32, f32, f32)]) {
        let p = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => b.props.clone(),
            None => return,
        };
        let accel = prop_f(&p, &["acceleration"], 0.0) as f32;
        let bounce = prop_f(&p, &["bounce-off-solids", "bounceoffsolids"], 0.0) != 0.0;
        let (angle, mut speed, x0, y0) = match self.inst(id) {
            Some(i) => match i.behaviors.iter().find(|b| b.name == bname) {
                Some(b) => match b.state {
                    Beh::Bullet { speed } => (i.angle, speed, i.x, i.y),
                    _ => return,
                },
                _ => return,
            },
            None => return,
        };
        speed += accel * dt;
        let rad = angle.to_radians();
        let (dx, dy) = (rad.cos() * speed * dt, rad.sin() * speed * dt);
        let mut new_angle = angle;
        let mut new_speed = Some(speed);
        if let Some(i) = self.inst_mut(id) {
            i.x += dx;
            i.y += dy;
            if bounce && !solids.is_empty() {
                let (l, t, r, b) = i.bbox();
                let overlap: Vec<(f32, f32, f32, f32)> = solids
                    .iter()
                    .filter(|(sl, st, sr, sb)| l < *sr && r > *sl && t < *sb && b > *st)
                    .copied()
                    .collect();
                if !overlap.is_empty() {
                    // test axe horizontal seul
                    i.x = x0;
                    let (l2, t2, r2, b2) = i.bbox();
                    let horiz_free = overlap.iter().all(|(sl, st, sr, sb)| !(l2 < *sr && r2 > *sl && t2 < *sb && b2 > *st));
                    if horiz_free {
                        new_angle = 180.0 - angle;
                        i.x = x0 + dx;
                    } else {
                        i.x = x0;
                        i.y = y0;
                        new_angle = -angle;
                        let rad = new_angle.to_radians();
                        i.x += rad.cos() * speed * dt;
                        i.y += rad.sin() * speed * dt;
                    }
                    let _ = (l, t, r, b);
                }
            }
            if let Some(b) = i.behaviors.iter_mut().find(|b| b.name == bname) {
                b.state = Beh::Bullet { speed };
            }
        }
        if let Some(i) = self.inst_mut(id) {
            i.angle = new_angle;
        }
        let _ = new_speed.take();
    }

    fn update_sine(&mut self, id: usize, bname: &str, dt: f32) {
        let p = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => b.props.clone(),
            None => return,
        };
        let period = prop_f(&p, &["period"], 1.0).max(0.01) as f32;
        let mag = prop_f(&p, &["magnitude"], 50.0) as f32;
        let movement = crate::c3p::prop_s(&p, &["movement"], "horizontal").to_lowercase();
        if let Some(i) = self.inst_mut(id) {
            if let Some(b) = i.behaviors.iter_mut().find(|b| b.name == bname) {
                if let Beh::Sine { base_x, base_y, t } = &mut b.state {
                    *t += dt;
                    let off = ((*t / period) * std::f32::consts::TAU).sin() * mag;
                    if movement.contains("vert") {
                        i.y = *base_y + off;
                    } else {
                        i.x = *base_x + off;
                    }
                }
            }
        }
    }

    fn update_fade(&mut self, id: usize, bname: &str, dt: f32) {
        let p = match self.inst(id).and_then(|i| i.behaviors.iter().find(|b| b.name == bname)) {
            Some(b) => b.props.clone(),
            None => return,
        };
        let fin = prop_f(&p, &["fade-in-time", "fadeintime"], 0.0) as f32;
        let wait = prop_f(&p, &["wait-time", "waittime"], 0.0) as f32;
        let fout = prop_f(&p, &["fade-out-time", "fadeouttime"], 1.0) as f32;
        let destroy = prop_f(&p, &["destroy-on-fade-out", "destroyonfadeout"], 1.0) != 0.0;
        let (phase, t, op) = {
            let inst = match self.inst(id) {
                Some(i) => i,
                None => return,
            };
            match inst.behaviors.iter().find(|b| b.name == bname).map(|b| &b.state) {
                Some(Beh::Fade { phase, t }) => (*phase, *t, inst.opacity),
                _ => return,
            }
        };
        let mut nphase = phase;
        let mut nt = t + dt;
        let mut nop = op;
        match phase {
            0 => {
                if fin > 0.0 {
                    nop = (nt / fin).min(1.0);
                    if nt >= fin {
                        nphase = 1;
                        nt = 0.0;
                    }
                } else {
                    nphase = 1;
                }
            }
            1 => {
                if nt >= wait {
                    nphase = 2;
                    nt = 0.0;
                }
            }
            _ => {
                if fout > 0.0 {
                    nop = (1.0 - nt / fout).max(0.0);
                    if nt >= fout {
                        if destroy {
                            if let Some(i) = self.inst_mut(id) {
                                i.destroyed = true;
                            }
                            return;
                        }
                        nphase = 1;
                        nt = 0.0;
                        nop = 1.0;
                    }
                }
            }
        }
        if let Some(i) = self.inst_mut(id) {
            i.opacity = nop;
            if let Some(b) = i.behaviors.iter_mut().find(|b| b.name == bname) {
                b.state = Beh::Fade { phase: nphase, t: nt };
            }
        }
    }

    fn update_wrap(&mut self, id: usize, _dt: f32) {
        let (l, t, r, b) = match self.inst(id) {
            Some(i) => i.bbox(),
            None => return,
        };
        let (lw, lh) = (self.layout.width, self.layout.height);
        let (ox, oy) = match self.inst(id) {
            Some(i) => (i.ox, i.oy),
            None => return,
        };
        if r < 0.0 {
            let x = lw + ox * (r - l);
            self.inst_mut(id).unwrap().x = x;
        } else if l > lw {
            let x = -ox * (r - l);
            self.inst_mut(id).unwrap().x = x;
        }
        if b < 0.0 {
            let y = lh + oy * (b - t);
            self.inst_mut(id).unwrap().y = y;
        } else if t > lh {
            let y = -oy * (b - t);
            self.inst_mut(id).unwrap().y = y;
        }
    }

    fn update_destroy_outside(&mut self, id: usize) {
        let margin = 100.0f32;
        if let Some(i) = self.inst(id) {
            let (l, t, r, b) = i.bbox();
            if r < -margin || l > self.layout.width + margin || b < -margin || t > self.layout.height + margin {
                self.inst_mut(id).unwrap().destroyed = true;
            }
        }
    }

    fn update_animations(&mut self, dt: f32) {
        for i in &mut self.instances {
            if i.destroyed || i.anim_speed <= 0.0 {
                continue;
            }
            let nframes = self
                .project
                .type_of(&i.type_name)
                .and_then(|t| t.anims.get(i.anim_idx))
                .map(|a| a.frames.len())
                .unwrap_or(1);
            if nframes <= 1 {
                continue;
            }
            let looping = self
                .project
                .type_of(&i.type_name)
                .and_then(|t| t.anims.get(i.anim_idx))
                .map(|a| a.looping)
                .unwrap_or(false);
            i.anim_t += i.anim_speed * dt;
            while i.anim_t >= 1.0 {
                i.anim_t -= 1.0;
                i.frame += 1;
                if i.frame >= nframes {
                    if looping {
                        i.frame = 0;
                    } else {
                        i.frame = nframes - 1;
                        i.anim_t = 0.0;
                        self.anim_finished.insert(i.id);
                        break;
                    }
                }
            }
        }
    }

    fn update_timers(&mut self) {
        let now = self.time;
        let mut fired: Vec<(usize, String)> = Vec::new();
        for ((id, tag), expiry) in self.timers.iter() {
            if *expiry <= now {
                fired.push((*id, tag.clone()));
            }
        }
        for k in fired {
            self.timers.remove(&k);
            self.timer_fired.insert(k);
        }
    }

    fn cleanup(&mut self) {
        self.instances.retain(|i| !i.destroyed);
        let alive: HashSet<usize> = self.instances.iter().map(|i| i.id).collect();
        self.last_collisions.retain(|(a, b)| alive.contains(a) && alive.contains(b));
        self.timers.retain(|(id, _), _| alive.contains(id));
    }

    fn update_camera(&mut self) {
        let target = self
            .instances
            .iter()
            .find(|i| i.behaviors.iter().any(|b| b.kind == BKind::ScrollTo && b.enabled));
        if let Some(t) = target {
            let (cx, cy) = (t.cx(), t.cy());
            let vw = self.view_w;
            let vh = self.view_h;
            let mut sx = cx - vw / 2.0;
            let mut sy = cy - vh / 2.0;
            let (lw, lh) = (self.layout.width, self.layout.height);
            if lw > vw {
                sx = sx.clamp(0.0, lw - vw);
            } else {
                sx = (lw - vw) / 2.0;
            }
            if lh > vh {
                sy = sy.clamp(0.0, lh - vh);
            } else {
                sy = (lh - vh) / 2.0;
            }
            self.scroll = (sx, sy);
        }
    }

    // ---------------- expressions ----------------

    pub fn eval_expr_with(&mut self, src: &str, picked: &Picked, self_id: Option<usize>) -> Val {
        match parse(src) {
            Ok(e) => {
                let mut ctx = Ctx { eng: self, picked, self_id };
                eval(&e, &mut ctx)
            }
            Err(err) => {
                self.warn(&format!("expression « {src} » invalide: {err}"));
                Val::Num(0.0)
            }
        }
    }

    pub fn eval_num(&mut self, src: &str, picked: &Picked, self_id: Option<usize>) -> f32 {
        self.eval_expr_with(src, picked, self_id).num() as f32
    }

    pub fn eval_str_with(&mut self, src: &str, picked: &Picked, self_id: Option<usize>) -> String {
        self.eval_expr_with(src, picked, self_id).text()
    }
}

pub struct Ctx<'a> {
    pub eng: &'a mut Engine,
    pub picked: &'a Picked,
    pub self_id: Option<usize>,
}

impl Env for Ctx<'_> {
    fn var(&mut self, name: &str) -> Option<Val> {
        if let Some(v) = self.eng.locals.get(name) {
            return Some(v.clone());
        }
        if let Some(v) = self.eng.globals.get(name) {
            return Some(v.clone());
        }
        None
    }

    fn prop(&mut self, obj: Option<&str>, path: &[String]) -> Option<Val> {
        if path.is_empty() {
            return None;
        }
        let p0 = normkey(&path[0]);
        // propriétés système
        if obj.is_none() && self.self_id.is_none() {
            return match p0.as_str() {
                "layoutwidth" => Some(Val::Num(self.eng.layout.width as f64)),
                "layoutheight" => Some(Val::Num(self.eng.layout.height as f64)),
                "layoutname" => Some(Val::Str(self.eng.layout.name.clone())),
                "windowwidth" | "screenwidth" => Some(Val::Num(self.eng.view_w as f64)),
                "windowheight" | "screenheight" => Some(Val::Num(self.eng.view_h as f64)),
                "time" => Some(Val::Num(self.eng.time as f64)),
                "dt" => Some(Val::Num(self.eng.dt as f64)),
                "scrollx" => Some(Val::Num(self.eng.scroll.0 as f64)),
                "scrolly" => Some(Val::Num(self.eng.scroll.1 as f64)),
                "projectname" => Some(Val::Str(self.eng.project.name.clone())),
                _ => None,
            };
        }
        // résolution de la cible
        let target: Option<usize> = if let Some(obj) = obj {
            if obj.eq_ignore_ascii_case("system") {
                None
            } else {
                let members = self.eng.project.concrete_members(obj);
                let mut found = None;
                for m in &members {
                    if let Some(list) = self.picked.get(m) {
                        if let Some(&id) = list.first() {
                            found = Some(id);
                            break;
                        }
                    }
                }
                found.or_else(|| {
                    for m in &members {
                        let ids = self.eng.instances_of(m);
                        if let Some(&id) = ids.first() {
                            return Some(id);
                        }
                    }
                    None
                })
            }
        } else {
            self.self_id
        };
        let id = target?;
        let inst = self.eng.inst(id)?;
        match p0.as_str() {
            "x" => Some(Val::Num(inst.x as f64)),
            "y" => Some(Val::Num(inst.y as f64)),
            "cx" => Some(Val::Num(inst.cx() as f64)),
            "cy" => Some(Val::Num(inst.cy() as f64)),
            "width" => Some(Val::Num(inst.w as f64)),
            "height" => Some(Val::Num(inst.h as f64)),
            "angle" => Some(Val::Num(inst.angle as f64)),
            "opacity" => Some(Val::Num(inst.opacity as f64 * 100.0)),
            "animationframe" | "frame" => Some(Val::Num(inst.frame as f64)),
            "mirrored" => Some(Val::Bool(inst.mirrored)),
            "count" => {
                let members = self.eng.project.concrete_members(obj.unwrap_or(&inst.type_name));
                Some(Val::Num(
                    self.eng.instances.iter().filter(|i| members.contains(&i.type_name)).count() as f64,
                ))
            }
            "pickedcount" => {
                let objn = obj.unwrap_or(&inst.type_name);
                let n = self.eng.candidates(self.picked, objn).len();
                Some(Val::Num(n as f64))
            }
            _ => {
                // propriété de comportement ?
                if path.len() >= 2 {
                    let bkey = normkey(&path[0]);
                    let prop = normkey(&path[1]);
                    let b = inst.behaviors.iter().find(|b| normkey(&b.name) == bkey || kind_key(b.kind) == bkey)?;
                    return match b.kind {
                        BKind::Platform => match prop.as_str() {
                            "speed" => match &b.state {
                                Beh::Platform { vx, .. } => Some(Val::Num(vx.abs() as f64)),
                                _ => None,
                            },
                            "vx" | "vectorx" => match &b.state {
                                Beh::Platform { vx, .. } => Some(Val::Num(*vx as f64)),
                                _ => None,
                            },
                            "vy" | "vectory" => match &b.state {
                                Beh::Platform { vy, .. } => Some(Val::Num(*vy as f64)),
                                _ => None,
                            },
                            "isonfloor" => match &b.state {
                                Beh::Platform { on_floor, .. } => Some(Val::Bool(*on_floor)),
                                _ => None,
                            },
                            _ => b.props.iter().find(|(k, _)| normkey(k) == prop).map(|(_, v)| v.clone()),
                        },
                        BKind::Bullet => match prop.as_str() {
                            "speed" => match &b.state {
                                Beh::Bullet { speed } => Some(Val::Num(*speed as f64)),
                                _ => None,
                            },
                            _ => b.props.iter().find(|(k, _)| normkey(k) == prop).map(|(_, v)| v.clone()),
                        },
                        _ => b.props.iter().find(|(k, _)| normkey(k) == prop).map(|(_, v)| v.clone()),
                    };
                }
                // variable d'instance ?
                if let Some(v) = inst.vars.get(&path[0]) {
                    return Some(v.clone());
                }
                // propriété de plugin ?
                if let Some(v) = inst.props.get(&path[0]) {
                    return Some(v.clone());
                }
                None
            }
        }
    }

    fn call(&mut self, name: &str, args: &[Val]) -> Option<Val> {
        match name.to_lowercase().as_str() {
            "random" => {
                let r = self.eng.rand01();
                return Some(match args.len() {
                    0 => Val::Num(r),
                    1 => Val::Num(r * args[0].num()),
                    _ => Val::Num(args[0].num() + r * (args[1].num() - args[0].num())),
                });
            }
            "loopindex" => {
                let want = args.first().map(|v| normkey(&v.text()));
                for (n, i) in self.eng.loops.iter().rev() {
                    if want.as_ref().map(|w| normkey(n) == w.as_str()).unwrap_or(true) {
                        return Some(Val::Num(*i));
                    }
                }
                return Some(Val::Num(0.0));
            }
            _ => None,
        }
    }

    fn warn(&mut self, msg: &str) {
        self.eng.warn(msg);
    }
}

fn kind_key(k: BKind) -> &'static str {
    match k {
        BKind::Platform => "platform",
        BKind::Solid => "solid",
        BKind::EightDir => "8direction",
        BKind::Bullet => "bullet",
        BKind::Sine => "sine",
        BKind::ScrollTo => "scrollto",
        BKind::DestroyOutside => "destroyoutsidelayout",
        BKind::Fade => "fade",
        BKind::Timer => "timer",
        BKind::Wrap => "wrap",
        BKind::Pin => "pin",
        BKind::Other => "other",
    }
}

// ---------------- fin helpers ----------------

fn param<'a>(c: &'a Cond, keys: &[&str]) -> Option<&'a String> {
    for k in keys {
        if let Some(v) = c.params.get(*k) {
            return Some(v);
        }
    }
    None
}

fn aparam<'a>(a: &'a Act, keys: &[&str]) -> Option<&'a String> {
    for k in keys {
        if let Some(v) = a.params.get(*k) {
            return Some(v);
        }
    }
    None
}

pub fn unquote(s: &str) -> String {
    let t = s.trim();
    if t.len() >= 2
        && ((t.starts_with('"') && t.ends_with('"')) || (t.starts_with('\'') && t.ends_with('\'')))
    {
        t[1..t.len() - 1].to_string()
    } else {
        t.to_string()
    }
}

fn cmp_ok(code: i64, l: &Val, r: &Val) -> bool {
    match code {
        0 => match (l, r) {
            (Val::Str(x), Val::Str(y)) => x == y,
            _ => (l.num() - r.num()).abs() < 1e-9,
        },
        1 => match (l, r) {
            (Val::Str(x), Val::Str(y)) => x != y,
            _ => (l.num() - r.num()).abs() >= 1e-9,
        },
        2 => l.num() < r.num(),
        3 => l.num() <= r.num(),
        4 => l.num() > r.num(),
        5 => l.num() >= r.num(),
        _ => (l.num() - r.num()).abs() < 1e-9,
    }
}

fn comp_code(c: &Cond) -> i64 {
    param(c, &["comparison", "comparator"])
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

// ---------------- interpréteur d'événements ----------------

impl Engine {
    pub fn run_events(&mut self) {
        let events = std::mem::take(&mut self.sheet_events);
        let empty = Picked::new();
        self.run_sequence(&events, &empty);
        self.sheet_events = events;
        self.start_flag = false;
    }

    fn run_sequence(&mut self, events: &[Event], base: &Picked) -> bool {
        let mut last_ok = false;
        for ev in events {
            match ev {
                Event::Block { conditions, actions, children, disabled, sid } => {
                    if *disabled {
                        last_ok = false;
                        continue;
                    }
                    last_ok = self.run_block(conditions, actions, children, base, *sid);
                }
                Event::Else { conditions, actions, children, disabled, sid } => {
                    if *disabled {
                        last_ok = false;
                        continue;
                    }
                    if last_ok {
                        last_ok = false;
                        continue;
                    }
                    last_ok = self.run_block(conditions, actions, children, base, *sid);
                }
                Event::Group { active, children, disabled, .. } => {
                    if *disabled || !*active {
                        last_ok = false;
                        continue;
                    }
                    self.run_sequence(children, base);
                    last_ok = false;
                }
                Event::Function { children, .. } => {
                    // fonctions C3 non supportées : exécution ignorée
                    let _ = children;
                }
                _ => last_ok = false,
            }
        }
        last_ok
    }

    fn run_block(
        &mut self,
        conds: &[Cond],
        acts: &[Act],
        children: &[Event],
        base: &Picked,
        sid: u64,
    ) -> bool {
        let loop_pos = conds.iter().position(|c| c.id == "for-each" || c.id == "for");
        let (pre, special, post): (&[Cond], Option<&Cond>, &[Cond]) = match loop_pos {
            Some(i) => (&conds[..i], Some(&conds[i]), &conds[i + 1..]),
            None => (conds, None, &[]),
        };
        let mut picked: Picked = base.clone();
        if !self.eval_conds(pre, &mut picked, sid) {
            return false;
        }
        match special {
            None => {
                if !post.is_empty() && !self.eval_conds(post, &mut picked, sid) {
                    return false;
                }
                self.run_actions(acts, &picked);
                let empty = Picked::new();
                let child_base = if children.is_empty() { &empty } else { &picked };
                self.run_sequence(children, child_base);
                true
            }
            Some(c) if c.id == "for-each" => {
                let obj = param(c, &["object", "object-to-iterate"])
                    .map(|s| unquote(s))
                    .unwrap_or_default();
                let ids = self.candidates(&picked, &obj);
                let mut any = false;
                for id in ids {
                    let tn = match self.inst(id) {
                        Some(i) => i.type_name.clone(),
                        None => continue,
                    };
                    let mut p2 = picked.clone();
                    p2.insert(obj.clone(), vec![id]);
                    p2.insert(tn, vec![id]);
                    if !post.is_empty() && !self.eval_conds(post, &mut p2, sid) {
                        continue;
                    }
                    any = true;
                    self.run_actions(acts, &p2);
                    self.run_sequence(children, &p2);
                    // détruits pendant la boucle : ok, la liste est un snapshot
                }
                any
            }
            Some(c) => {
                // boucle "for"
                let name = param(c, &["name"]).map(|s| unquote(s)).unwrap_or_default();
                let start = self.eval_num(param(c, &["start-index"]).map(|s| s.as_str()).unwrap_or("0"), &picked, None);
                let end = self.eval_num(param(c, &["end-index"]).map(|s| s.as_str()).unwrap_or("0"), &picked, None);
                let mut any = false;
                let mut i = start as i64;
                while i <= end as i64 {
                    self.loops.push((name.clone(), i as f64));
                    let ok = if post.is_empty() { true } else { self.eval_conds(post, &mut picked, sid) };
                    if ok {
                        any = true;
                        self.run_actions(acts, &picked);
                        self.run_sequence(children, &picked);
                    }
                    self.loops.pop();
                    i += 1;
                }
                any
            }
        }
    }

    fn eval_conds(&mut self, conds: &[Cond], picked: &mut Picked, sid: u64) -> bool {
        for c in conds {
            if c.id == "for" || c.id == "for-each" {
                continue;
            }
            if !self.eval_cond(c, picked, sid) {
                return false;
            }
        }
        true
    }

    fn eval_cond(&mut self, c: &Cond, picked: &mut Picked, sid: u64) -> bool {
        match c.id.as_str() {
            "every-tick" | "always" => true,
            "on-start-of-layout" | "on-layout-start" => self.start_flag,
            "every-x-seconds" => {
                let interval = self
                    .eval_num(param(c, &["interval-seconds", "seconds", "0"]).map(|s| s.as_str()).unwrap_or("1"), picked, None)
                    .max(0.01);
                let now = self.time;
                let next = self.block_timers.entry(sid).or_insert(now + interval);
                if now >= *next {
                    *next = now + interval;
                    true
                } else {
                    false
                }
            }
            "key-is-down" => {
                let k = self.key_of(c, picked);
                self.input.down.contains(&k)
            }
            "on-key-pressed" => {
                let k = self.key_of(c, picked);
                self.input.pressed.contains(&k)
            }
            "on-key-released" => {
                let k = self.key_of(c, picked);
                self.input.released.contains(&k)
            }
            "on-collision-with-another-object" => self.cond_collision(c, picked),
            "is-overlapping" | "is-overlapping-at-offset" => self.cond_overlap(c, picked),
            "compare-instance-variable" => self.cond_cmp_instvar(c, picked),
            "compare-eventvar" => {
                let name = param(c, &["variable"]).map(|s| unquote(s)).unwrap_or_default();
                let cur = self
                    .locals
                    .get(&name)
                    .cloned()
                    .or_else(|| self.globals.get(&name).cloned())
                    .unwrap_or(Val::Num(0.0));
                let val = self.eval_expr_with(
                    param(c, &["value"]).map(|s| s.as_str()).unwrap_or("0"),
                    picked,
                    None,
                );
                cmp_ok(comp_code(c), &cur, &val)
            }
            "compare-two-values" => {
                let a = self.eval_expr_with(param(c, &["first-value"]).map(|s| s.as_str()).unwrap_or("0"), picked, None);
                let b = self.eval_expr_with(param(c, &["second-value"]).map(|s| s.as_str()).unwrap_or("0"), picked, None);
                cmp_ok(comp_code(c), &a, &b)
            }
            "compare-x" | "compare-y" => {
                self.ensure_picked(picked, &c.object);
                let ids = self.candidates(picked, &c.object);
                let target = self.eval_expr_with(
                    param(c, &["x-co-ordinate", "y-co-ordinate", "coordinate"]).map(|s| s.as_str()).unwrap_or("0"),
                    picked,
                    None,
                );
                let code = comp_code(c);
                let isx = c.id == "compare-x";
                let keep: Vec<usize> = ids
                    .into_iter()
                    .filter(|id| {
                        if let Some(i) = self.inst(*id) {
                            let cur = if isx { i.x } else { i.y } as f64;
                            cmp_ok(code, &Val::Num(cur), &target)
                        } else {
                            false
                        }
                    })
                    .collect();
                picked.insert(c.object.clone(), keep.clone());
                !keep.is_empty()
            }
            "is-on-floor" => self.filter_platform(c, picked, |p| p.on_floor),
            "is-jumping" => self.filter_platform(c, picked, |p| p.vy < 0.0 && !p.on_floor),
            "is-falling" => self.filter_platform(c, picked, |p| p.vy > 0.0 && !p.on_floor),
            "is-moving" => self.filter_platform(c, picked, |p| p.vx != 0.0 || p.vy != 0.0),
            "on-landed" => {
                self.ensure_picked(picked, &c.object);
                let ids = self.candidates(picked, &c.object);
                let keep: Vec<usize> = ids.into_iter().filter(|id| self.landed.contains(id)).collect();
                picked.insert(c.object.clone(), keep.clone());
                !keep.is_empty()
            }
            "is-animation-playing" => {
                self.ensure_picked(picked, &c.object);
                let want = param(c, &["animation"]).map(|s| unquote(s)).unwrap_or_default();
                let ids = self.candidates(picked, &c.object);
                let keep: Vec<usize> = ids
                    .into_iter()
                    .filter(|id| match self.inst(*id) {
                        Some(i) => self
                            .project
                            .type_of(&i.type_name)
                            .and_then(|t| t.anims.get(i.anim_idx))
                            .map(|a| a.name.eq_ignore_ascii_case(&want))
                            .unwrap_or(false),
                        None => false,
                    })
                    .collect();
                picked.insert(c.object.clone(), keep.clone());
                !keep.is_empty()
            }
            "on-animation-finished" => {
                self.ensure_picked(picked, &c.object);
                let ids = self.candidates(picked, &c.object);
                let keep: Vec<usize> = ids.into_iter().filter(|id| self.anim_finished.contains(id)).collect();
                picked.insert(c.object.clone(), keep.clone());
                !keep.is_empty()
            }
            "on-timer" => {
                let tag = param(c, &["tag"]).map(|s| unquote(s)).unwrap_or_default();
                self.ensure_picked(picked, &c.object);
                let ids = self.candidates(picked, &c.object);
                let keep: Vec<usize> = ids.into_iter().filter(|id| self.timer_fired.contains(&(*id, tag.clone()))).collect();
                picked.insert(c.object.clone(), keep.clone());
                !keep.is_empty()
            }
            "is-timer-running" => {
                let tag = param(c, &["tag"]).map(|s| unquote(s)).unwrap_or_default();
                self.ensure_picked(picked, &c.object);
                let ids = self.candidates(picked, &c.object);
                let keep: Vec<usize> = ids.into_iter().filter(|id| self.timers.contains_key(&(*id, tag.clone()))).collect();
                picked.insert(c.object.clone(), keep.clone());
                !keep.is_empty()
            }
            "layer-is-visible" => {
                let lname = param(c, &["layer"]).map(|s| unquote(s)).unwrap_or_default();
                self.layout
                    .layers
                    .iter()
                    .find(|l| l.name.eq_ignore_ascii_case(&lname))
                    .map(|l| l.visible)
                    .unwrap_or(true)
            }
            "on-mouse-button-down" => {
                let b = param(c, &["mouse-button", "button"]).map(|s| mouse_btn(s)).unwrap_or(0);
                self.input.mouse_pressed.contains(&b)
            }
            "is-mouse-button-down" => {
                let b = param(c, &["mouse-button", "button"]).map(|s| mouse_btn(s)).unwrap_or(0);
                self.input.mouse_down.contains(&b)
            }
            _ => {
                self.warn(&format!("condition « {} » non supportée (ignorée comme fausse)", c.id));
                false
            }
        }
    }

    fn key_of(&mut self, c: &Cond, picked: &Picked) -> u32 {
        let raw = param(c, &["key"]).cloned().unwrap_or_default();
        let t = raw.trim();
        if !t.is_empty() && t.chars().all(|ch| ch.is_ascii_digit()) {
            t.parse().unwrap_or(0)
        } else {
            // expression ou nom symbolique
            let v = self.eval_expr_with(&raw, picked, None).num() as u32;
            if v == 0 {
                parse_key(&raw)
            } else {
                v
            }
        }
    }

    fn filter_platform(&mut self, c: &Cond, picked: &mut Picked, f: impl Fn(&BehPlatformView) -> bool) -> bool {
        self.ensure_picked(picked, &c.object);
        let ids = self.candidates(picked, &c.object);
        let keep: Vec<usize> = ids
            .into_iter()
            .filter(|id| match self.inst(*id).and_then(|i| i.behaviors.iter().find(|b| b.kind == BKind::Platform)) {
                Some(b) => match &b.state {
                    Beh::Platform { vx, vy, on_floor } => f(&BehPlatformView { vx: *vx, vy: *vy, on_floor: *on_floor }),
                    _ => false,
                },
                None => false,
            })
            .collect();
        picked.insert(c.object.clone(), keep.clone());
        !keep.is_empty()
    }

    fn cond_collision(&mut self, c: &Cond, picked: &mut Picked) -> bool {
        let other = param(c, &["object"]).map(|s| unquote(s)).unwrap_or_default();
        self.ensure_picked(picked, &c.object);
        let a_ids = self.candidates(picked, &c.object);
        let b_ids = self.candidates(picked, &other);
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for &a in &a_ids {
            for &b in &b_ids {
                if a == b {
                    continue;
                }
                if !self.overlap(a, b) {
                    continue;
                }
                let key = if a < b { (a, b) } else { (b, a) };
                if !self.last_collisions.contains(&key) {
                    hits.push((a, b));
                }
            }
        }
        if hits.is_empty() {
            return false;
        }
        for (a, b) in hits {
            self.current_pairs.insert(if a < b { (a, b) } else { (b, a) });
            for id in [a, b] {
                if let Some(i) = self.inst(id) {
                    let tn = i.type_name.clone();
                    let entry = picked.entry(tn).or_default();
                    if !entry.contains(&id) {
                        entry.push(id);
                    }
                }
            }
        }
        true
    }

    fn cond_overlap(&mut self, c: &Cond, picked: &mut Picked) -> bool {
        let other = param(c, &["object"]).map(|s| unquote(s)).unwrap_or_default();
        let offx = self.eval_num(param(c, &["offset-x"]).map(|s| s.as_str()).unwrap_or("0"), picked, None);
        let offy = self.eval_num(param(c, &["offset-y"]).map(|s| s.as_str()).unwrap_or("0"), picked, None);
        self.ensure_picked(picked, &c.object);
        let a_ids = self.candidates(picked, &c.object);
        let b_ids = self.candidates(picked, &other);
        let mut keep: Vec<usize> = Vec::new();
        let mut hit_b: Vec<usize> = Vec::new();
        for &a in &a_ids {
            let (al, at, ar, ab) = match self.inst(a) {
                Some(i) => i.bbox(),
                None => continue,
            };
            let (al, at, ar, ab) = (al + offx, at + offy, ar + offx, ab + offy);
            let mut found = false;
            for &b in &b_ids {
                if a == b {
                    continue;
                }
                if let Some((bl, bt, br, bb)) = self.inst(b).map(|i| i.bbox()) {
                    if al < br && ar > bl && at < bb && ab > bt {
                        found = true;
                        if !hit_b.contains(&b) {
                            hit_b.push(b);
                        }
                    }
                }
            }
            if found {
                keep.push(a);
            }
        }
        if keep.is_empty() {
            return false;
        }
        picked.insert(c.object.clone(), keep);
        if let Some(bi) = picked.get_mut(&other) {
            for b in hit_b {
                if !bi.contains(&b) {
                    bi.push(b);
                }
            }
        }
        true
    }

    fn cond_cmp_instvar(&mut self, c: &Cond, picked: &mut Picked) -> bool {
        self.ensure_picked(picked, &c.object);
        let ids = self.candidates(picked, &c.object);
        let varname = param(c, &["instance-variable", "variable"]).map(|s| unquote(s)).unwrap_or_default();
        let code = comp_code(c);
        let vexpr = param(c, &["value"]).map(|s| s.as_str()).unwrap_or("0").to_string();
        let mut keep: Vec<usize> = Vec::new();
        for id in ids {
            let val = self.eval_expr_with(&vexpr, picked, Some(id));
            let cur = self.inst(id).and_then(|i| i.vars.get(&varname)).cloned().unwrap_or(Val::Num(0.0));
            if cmp_ok(code, &cur, &val) {
                keep.push(id);
            }
        }
        picked.insert(c.object.clone(), keep.clone());
        !keep.is_empty()
    }

    fn overlap(&self, a: usize, b: usize) -> bool {
        match (self.inst(a), self.inst(b)) {
            (Some(x), Some(y)) => {
                let (l1, t1, r1, b1) = x.bbox();
                let (l2, t2, r2, b2) = y.bbox();
                l1 < r2 && r1 > l2 && t1 < b2 && b1 > t2
            }
            _ => false,
        }
    }

    // ---------------- actions ----------------

    fn run_actions(&mut self, acts: &[Act], picked: &Picked) {
        for a in acts {
            self.run_action(a, picked);
        }
    }

    fn targets(&self, a: &Act, picked: &Picked) -> Vec<usize> {
        self.candidates(picked, &a.object)
    }

    fn run_action(&mut self, a: &Act, picked: &Picked) {
        match a.id.as_str() {
            "go-to-layout" => {
                let name = self.eval_str_with(aparam(a, &["layout"]).map(|s| s.as_str()).unwrap_or(""), picked, None);
                self.pending = Some(PendingNav::Layout(unquote(&name)));
            }
            "restart-layout" => self.pending = Some(PendingNav::Restart),
            "next-layout" | "previous-layout" => {
                let cur = self.layout.name.clone();
                if let Some(idx) = self.project.layouts.iter().position(|l| l.name == cur) {
                    let n = self.project.layouts.len();
                    let delta = if a.id == "next-layout" { 1 } else { n - 1 };
                    let next = self.project.layouts[(idx + delta) % n].name.clone();
                    self.pending = Some(PendingNav::Layout(next));
                }
            }
            "create-object" => {
                let obj = aparam(a, &["object-to-create", "object"]).map(|s| unquote(s)).unwrap_or_default();
                let x = self.eval_num(aparam(a, &["x"]).map(|s| s.as_str()).unwrap_or("0"), picked, None);
                let y = self.eval_num(aparam(a, &["y"]).map(|s| s.as_str()).unwrap_or("0"), picked, None);
                let layer_name = aparam(a, &["layer"]).map(|s| unquote(s)).unwrap_or_default();
                self.spawn_instance(&obj, x, y, &layer_name);
            }
            "spawn-another-object" => {
                let obj = aparam(a, &["object"]).map(|s| unquote(s)).unwrap_or_default();
                let (x, y) = match picked
                    .get(&a.object)
                    .and_then(|l| l.first().copied())
                    .and_then(|id| self.inst(id).map(|i| (i.cx(), i.cy())))
                {
                    Some((cx, cy)) => (cx, cy),
                    None => (0.0, 0.0),
                };
                let layer_name = aparam(a, &["layer"]).map(|s| unquote(s)).unwrap_or_default();
                self.spawn_instance(&obj, x, y, &layer_name);
            }
            "destroy" => {
                for id in self.targets(a, picked) {
                    if let Some(i) = self.inst_mut(id) {
                        i.destroyed = true;
                    }
                }
            }
            "set-instvar-value" => {
                let name = aparam(a, &["instance-variable", "variable"]).map(|s| unquote(s)).unwrap_or_default();
                let expr = aparam(a, &["value"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_expr_with(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.vars.insert(name.clone(), v);
                    }
                }
            }
            "set-eventvar-value" | "add-to-eventvar" | "subtract-from-eventvar" => {
                let name = aparam(a, &["variable"]).map(|s| unquote(s)).unwrap_or_default();
                let expr = aparam(a, &["value"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                let v = self.eval_expr_with(&expr, picked, None);
                let key = name.clone();
                if self.locals.contains_key(&key) {
                    let cur = self.locals.get(&key).cloned().unwrap_or(Val::Num(0.0));
                    let nv = match a.id.as_str() {
                        "add-to-eventvar" => Val::Num(cur.num() + v.num()),
                        "subtract-from-eventvar" => Val::Num(cur.num() - v.num()),
                        _ => v,
                    };
                    self.locals.insert(key, nv);
                } else {
                    let cur = self.globals.get(&key).cloned().unwrap_or(Val::Num(0.0));
                    let nv = match a.id.as_str() {
                        "add-to-eventvar" => Val::Num(cur.num() + v.num()),
                        "subtract-from-eventvar" => Val::Num(cur.num() - v.num()),
                        _ => v,
                    };
                    self.globals.insert(key, nv);
                }
            }
            "set-text" | "append-text" => {
                let expr = aparam(a, &["text"]).map(|s| s.as_str()).unwrap_or("").to_string();
                for id in self.targets(a, picked) {
                    let t = self.eval_expr_with(&expr, picked, Some(id)).text();
                    if let Some(i) = self.inst_mut(id) {
                        let cur = i.props.get("text").map(|v| v.text()).unwrap_or_default();
                        let nv = if a.id == "append-text" { format!("{cur}{t}") } else { t };
                        i.props.insert("text".into(), Val::Str(nv));
                    }
                }
            }
            "set-animation" => {
                let aname = aparam(a, &["animation"]).map(|s| unquote(s)).unwrap_or_default();
                let from = aparam(a, &["from"]).map(|s| unquote(s)).unwrap_or_default();
                for id in self.targets(a, picked) {
                    let idx = self
                        .project
                        .type_of(&self.inst(id).map(|i| i.type_name.clone()).unwrap_or_default())
                        .and_then(|t| t.anims.iter().position(|an| an.name.eq_ignore_ascii_case(&aname)));
                    if let (Some(i), Some(idx)) = (self.inst_mut(id), idx) {
                        i.anim_idx = idx;
                        if from.eq_ignore_ascii_case("beginning") {
                            i.frame = 0;
                            i.anim_t = 0.0;
                        }
                    }
                }
            }
            "set-animation-speed" => {
                let expr = aparam(a, &["speed"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.anim_speed = v;
                    }
                }
            }
            "set-animation-frame" => {
                let expr = aparam(a, &["frame"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.frame = v.max(0.0) as usize;
                        i.anim_t = 0.0;
                    }
                }
            }
            "set-x" => {
                let expr = aparam(a, &["x"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.x = v;
                    }
                }
            }
            "set-y" => {
                let expr = aparam(a, &["y"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.y = v;
                    }
                }
            }
            "set-position" | "set-position-to" => {
                let ex = aparam(a, &["x"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                let ey = aparam(a, &["y"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let vx = self.eval_num(&ex, picked, Some(id));
                    let vy = self.eval_num(&ey, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.x = vx;
                        i.y = vy;
                    }
                }
            }
            "set-angle" => {
                let expr = aparam(a, &["angle"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.angle = v;
                    }
                }
            }
            "set-width" => {
                let expr = aparam(a, &["width"]).map(|s| s.as_str()).unwrap_or("32").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.w = v;
                    }
                }
            }
            "set-height" => {
                let expr = aparam(a, &["height"]).map(|s| s.as_str()).unwrap_or("32").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.h = v;
                    }
                }
            }
            "set-opacity" => {
                let expr = aparam(a, &["opacity"]).map(|s| s.as_str()).unwrap_or("100").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        i.opacity = (v / 100.0).clamp(0.0, 1.0);
                    }
                }
            }
            "move-forward" => {
                let expr = aparam(a, &["distance"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let d = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        let rad = i.angle.to_radians();
                        i.x += rad.cos() * d;
                        i.y += rad.sin() * d;
                    }
                }
            }
            "set-vector-x" | "set-vector-y" => {
                let isx = a.id == "set-vector-x";
                let key = if isx { "vector-x" } else { "vector-y" };
                let expr = aparam(a, &[key]).map(|s| s.as_str()).unwrap_or("0").to_string();
                for id in self.targets(a, picked) {
                    let v = self.eval_num(&expr, picked, Some(id));
                    if let Some(i) = self.inst_mut(id) {
                        if let Some(b) = i.behaviors.iter_mut().find(|b| matches!(b.kind, BKind::Platform | BKind::EightDir)) {
                            match &mut b.state {
                                Beh::Platform { vx, vy, .. } => {
                                    if isx {
                                        *vx = v;
                                    } else {
                                        *vy = v;
                                    }
                                }
                                Beh::EightDir { vx, vy } => {
                                    if isx {
                                        *vx = v;
                                    } else {
                                        *vy = v;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            "simulate-control" => {
                let ctl = aparam(a, &["control"]).map(|s| unquote(s)).unwrap_or_default().to_lowercase();
                for id in self.targets(a, picked) {
                    let e = self.sim_ctrl.entry(id).or_default();
                    match ctl.as_str() {
                        "left" => e.left = true,
                        "right" => e.right = true,
                        "up" => e.up = true,
                        "down" => e.down = true,
                        "jump" => e.jump = true,
                        _ => {}
                    }
                }
            }
            "set-mirrored" => {
                let st = aparam(a, &["state"]).map(|s| unquote(s)).unwrap_or_default().to_lowercase();
                let m = st.contains("mirrored") && !st.contains("not");
                for id in self.targets(a, picked) {
                    if let Some(i) = self.inst_mut(id) {
                        i.mirrored = m;
                    }
                }
            }
            "set-enabled" => {
                let st = aparam(a, &["state"]).map(|s| unquote(s)).unwrap_or_default().to_lowercase();
                let en = st.contains("enabled") && !st.contains("dis");
                for id in self.targets(a, picked) {
                    if let Some(i) = self.inst_mut(id) {
                        let want = a.behavior.clone();
                        for b in &mut i.behaviors {
                            if want.as_ref().map(|w| b.name.eq_ignore_ascii_case(w)).unwrap_or(true) {
                                b.enabled = en;
                            }
                        }
                    }
                }
            }
            "set-layer-visible" => {
                let lname = aparam(a, &["layer"]).map(|s| unquote(s)).unwrap_or_default();
                let vis = aparam(a, &["visibility"]).map(|s| unquote(s)).unwrap_or_default().to_lowercase();
                let v = vis.contains("visible") && !vis.contains("invis");
                if let Some(l) = self.layout.layers.iter_mut().find(|l| l.name.eq_ignore_ascii_case(&lname)) {
                    l.visible = v;
                }
            }
            "set-time-scale" => {
                let expr = aparam(a, &["time-scale"]).map(|s| s.as_str()).unwrap_or("1").to_string();
                self.time_scale = self.eval_num(&expr, picked, None) as f32;
            }
            "start-timer" => {
                let tag = aparam(a, &["tag"]).map(|s| unquote(s)).unwrap_or_default();
                let expr = aparam(a, &["duration"]).map(|s| s.as_str()).unwrap_or("1").to_string();
                for id in self.targets(a, picked) {
                    let d = self.eval_num(&expr, picked, Some(id));
                    self.timers.insert((id, tag.clone()), self.time + d);
                }
            }
            "stop-timer" => {
                let tag = aparam(a, &["tag"]).map(|s| unquote(s)).unwrap_or_default();
                for id in self.targets(a, picked) {
                    self.timers.remove(&(id, tag.clone()));
                }
            }
            "scroll-to-object" => {
                if let Some(&id) = self.targets(a, picked).first() {
                    if let Some(i) = self.inst(id) {
                        let (cx, cy) = (i.cx(), i.cy());
                        let vw = self.view_w;
                        let vh = self.view_h;
                        let (lw, lh) = (self.layout.width, self.layout.height);
                        let sx = if lw > vw { (cx - vw / 2.0).clamp(0.0, lw - vw) } else { (lw - vw) / 2.0 };
                        let sy = if lh > vh { (cy - vh / 2.0).clamp(0.0, lh - vh) } else { (lh - vh) / 2.0 };
                        self.scroll = (sx, sy);
                    }
                }
            }
            "set-default-color" | "set-color" | "set-effect-enabled" => {
                if a.id != "set-effect-enabled" {
                    let expr = aparam(a, &["color"]).map(|s| s.as_str()).unwrap_or("0").to_string();
                    let v = self.eval_expr_with(&expr, picked, None).num() as u32;
                    let c = Rgba::new(
                        ((v >> 16) & 0xFF) as f32 / 255.0,
                        ((v >> 8) & 0xFF) as f32 / 255.0,
                        (v & 0xFF) as f32 / 255.0,
                        1.0,
                    );
                    for id in self.targets(a, picked) {
                        if let Some(i) = self.inst_mut(id) {
                            i.tint = Some(c);
                        }
                    }
                }
            }
            "reset-global-variables" => self.globals = self.globals_init.clone(),
            "wait-for-previous-actions" => {}
            "log" => {
                let msg = self.eval_str_with(aparam(a, &["message"]).map(|s| s.as_str()).unwrap_or(""), picked, None);
                self.logs.push(msg);
            }
            _ => self.warn(&format!("action « {} » non supportée (ignorée)", a.id)),
        }
    }

    fn spawn_instance(&mut self, obj_name: &str, x: f32, y: f32, layer_name: &str) {
        let t = match self.project.type_of(obj_name) {
            Some(t) => t.clone(),
            None => {
                self.warn(&format!("création impossible : objet « {obj_name} » inconnu"));
                return;
            }
        };
        if matches!(t.plugin, Plugin::Keyboard | Plugin::Mouse) {
            return;
        }
        let layer = self
            .layout
            .layers
            .iter()
            .position(|l| l.name.eq_ignore_ascii_case(layer_name))
            .unwrap_or(0);
        let (w, h) = t
            .anims
            .first()
            .and_then(|a| a.frames.first())
            .and_then(|k| self.project.images.get(k))
            .map(|tex| (tex.w as f32, tex.h as f32))
            .unwrap_or((32.0, 32.0));
        let mut vars = HashMap::new();
        for (k, v) in &t.vars {
            vars.insert(k.clone(), v.clone());
        }
        let mut behaviors: Vec<BehState> = Vec::new();
        for b in &t.behaviors {
            let st = match b.kind {
                BKind::Platform => Beh::Platform { vx: 0.0, vy: 0.0, on_floor: false },
                BKind::EightDir => Beh::EightDir { vx: 0.0, vy: 0.0 },
                BKind::Bullet => Beh::Bullet { speed: prop_f(&b.props, &["speed", "initial-speed"], 300.0) as f32 },
                BKind::Sine => Beh::Sine { base_x: x, base_y: y, t: 0.0 },
                BKind::Fade => Beh::Fade { phase: 0, t: 0.0 },
                BKind::Pin => Beh::Pin { target: None },
                _ => Beh::Other,
            };
            behaviors.push(BehState { name: b.name.clone(), kind: b.kind, enabled: true, props: b.props.clone(), state: st });
        }
        let inst = Inst {
            id: self.next_id,
            uid: 0,
            type_name: t.name.clone(),
            x,
            y,
            w,
            h,
            angle: 0.0,
            opacity: 1.0,
            layer,
            visible: true,
            ox: 0.5,
            oy: 0.5,
            z: 0.0,
            vars,
            props: if t.plugin == Plugin::Text {
                let mut p = HashMap::new();
                p.insert("text".into(), Val::Str(String::new()));
                p.insert("color".into(), Val::Str("#FFFFFFFF".into()));
                p
            } else {
                HashMap::new()
            },
            behaviors,
            anim_idx: 0,
            frame: 0,
            anim_t: 0.0,
            anim_speed: t.anims.first().map(|a| a.speed).unwrap_or(0.0),
            mirrored: false,
            destroyed: false,
            tint: None,
            prev_x: x,
            prev_y: y,
        };
        self.instances.push(inst);
        self.next_id += 1;
    }
}

pub struct BehPlatformView {
    pub vx: f32,
    pub vy: f32,
    pub on_floor: bool,
}

fn mouse_btn(s: &str) -> u8 {
    let t = s.to_lowercase();
    if t.contains("right") {
        2
    } else if t.contains("middle") {
        1
    } else {
        0
    }
}

// ---------------- rendu de scène ----------------

fn texture_for(eng: &Engine, inst: &Inst) -> Option<crate::model::Tex> {
    let t = eng.project.type_of(&inst.type_name)?;
    let anim = t.anims.get(inst.anim_idx)?;
    let key = anim.frames.get(inst.frame).or_else(|| anim.frames.first())?;
    eng.project.images.get(key).cloned()
}

pub fn render_frame(eng: &Engine) -> FrameBuf {
    let w = (eng.view_w.max(1.0)) as usize;
    let h = (eng.view_h.max(1.0)) as usize;
    let mut fb = FrameBuf::new(w, h);
    let bg = eng
        .layout
        .background
        .map(|c| Rgba::new(c[0], c[1], c[2], 1.0))
        .unwrap_or_else(|| {
            let b = eng.project.background;
            Rgba::new(b[0], b[1], b[2], 1.0)
        });
    fb.clear(bg);
    for (li, layer) in eng.layout.layers.iter().enumerate() {
        if !layer.visible {
            continue;
        }
        let px = eng.scroll.0 * (layer.parallax.0 / 100.0);
        let py = eng.scroll.1 * (layer.parallax.1 / 100.0);
        for inst in eng.instances.iter().filter(|i| i.layer == li && i.visible && !i.destroyed) {
            let (l, t, _, _) = inst.bbox();
            let sx = l - px;
            let sy = t - py;
            let opacity = (inst.opacity * layer.opacity).clamp(0.0, 1.0);
            let plugin = eng.project.type_of(&inst.type_name).map(|t| t.plugin);
            match plugin {
                Some(Plugin::Sprite) => {
                    if inst.angle == 0.0 && (sx > w as f32 || sy > h as f32 || sx + inst.w < 0.0 || sy + inst.h < 0.0) {
                        continue;
                    }
                    let tex = match texture_for(eng, inst) {
                        Some(t) => t,
                        None => {
                            let mut hsh: u64 = 1469598103934665603;
                            for b in inst.type_name.bytes() {
                                hsh ^= b as u64;
                                hsh = hsh.wrapping_mul(1099511628211);
                            }
                            placeholder_tex(hsh)
                        }
                    };
                    fb.draw_tex(&tex, sx, sy, inst.w, inst.h, inst.angle, inst.ox, inst.oy, opacity, inst.tint, inst.mirrored);
                }
                Some(Plugin::TiledBg) => {
                    let tex = match texture_for(eng, inst) {
                        Some(t) => t,
                        None => placeholder_tex(7),
                    };
                    fb.tile_tex(&tex, sx, sy, inst.w, inst.h, opacity);
                }
                Some(Plugin::Text) => {
                    let text = crate::c3p::prop_s(&inst.props, &["text"], "");
                    let size = crate::c3p::prop_f(&inst.props, &["font-size", "size", "font-height", "fontHeight"], 20.0) as f32;
                    let color = Rgba::hex(&crate::c3p::prop_s(&inst.props, &["color", "font-color"], "#FFFFFFFF"));
                    fb.draw_text(&text, sx, sy, size, color, opacity);
                }
                _ => {}
            }
        }
    }
    fb
}
