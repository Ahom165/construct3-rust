//! Zone centrale de l'éditeur : onglets (layout, événements, médias, aperçu),
//! canvas d'édition avec zoom/déplacement, rendu des sprites, aperçu jouable.

use crate::assets::{AssetKind};
use crate::editor::{EditorApp, Tab, Tool, COL_ACT, COL_ACCENT, COL_BG, COL_COND, COL_DIM, COL_ERR, COL_GRID, COL_SEL, COL_WARN};
use crate::expr::Val;
use crate::model::*;
use crate::runtime::render_frame;
use eframe::egui;
use egui::{Color32, ColorImage, Pos2, Rect, Sense, Shape, Stroke, TextureOptions, Vec2};
use std::collections::HashMap;

impl EditorApp {
    pub(crate) fn center(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // barre d'onglets
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if let Some(p) = self.project.as_ref() {
                for (i, l) in p.layouts.iter().enumerate() {
                    let sel = matches!(self.tab, Tab::Layout(j) if j == i);
                    if ui.selectable_label(sel, &l.name).clicked() {
                        self.tab = Tab::Layout(i);
                    }
                }
                for (i, s) in p.sheets.iter().enumerate() {
                    let sel = matches!(self.tab, Tab::Events(j) if j == i);
                    if ui.selectable_label(sel, format!("{} (events)", s.name)).clicked() {
                        self.tab = Tab::Events(i);
                    }
                }
            }
            if ui.selectable_label(matches!(self.tab, Tab::Media), "Médias").clicked() {
                self.tab = Tab::Media;
            }
            let prev_sel = matches!(self.tab, Tab::Preview);
            if ui.selectable_label(prev_sel, if self.preview.is_some() { "Aperçu (en cours)" } else { "Aperçu" }).clicked() {
                if self.preview.is_none() {
                    self.run_preview();
                } else {
                    self.tab = Tab::Preview;
                }
            }
        });
        ui.separator();
        match self.tab.clone() {
            Tab::Layout(i) => self.show_layout_tab(ui, i),
            Tab::Events(i) => self.show_events_tab(ui, i),
            Tab::Media => self.show_media_tab(ui),
            Tab::Preview => self.show_preview_tab(ui, ctx),
        }
    }

    // ---------------- layout ----------------

    fn show_layout_tab(&mut self, ui: &mut egui::Ui, li: usize) {
        let Some(p) = self.project.as_ref() else {
            ui.centered_and_justified(|ui| ui.label("Aucun projet. Créez-en un ou ouvrez un .c3p."));
            return;
        };
        if li >= p.layouts.len() {
            ui.centered_and_justified(|ui| ui.label("Layout introuvable."));
            return;
        }
        let lw = p.layouts[li].width;
        let lh = p.layouts[li].height;
        let bg = p.layouts[li]
            .background
            .or(Some(p.background))
            .map(|c| Color32::from_rgba_unmultiplied((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255));

        let avail = ui.available_size();
        let (rect, resp) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        let painter = ui.painter_at(rect);

        // vue : (zoom, centre monde)
        let entry = self.views.entry(li).or_insert_with(|| {
            let z = ((rect.width() / lw.max(1.0)).min(rect.height() / lh.max(1.0)) * 0.9).clamp(0.05, 4.0);
            (z, Pos2::new(lw / 2.0, lh / 2.0))
        });
        let (mut zoom, mut pan) = *entry;

        let to_screen = |world: Pos2, zoom: f32, pan: Pos2| -> Pos2 {
            Pos2::new(rect.left() + (world.x - pan.x) * zoom + rect.width() / 2.0, rect.top() + (world.y - pan.y) * zoom + rect.height() / 2.0)
        };
        let to_world = |screen: Pos2, zoom: f32, pan: Pos2| -> Pos2 {
            Pos2::new((screen.x - rect.left() - rect.width() / 2.0) / zoom + pan.x, (screen.y - rect.top() - rect.height() / 2.0) / zoom + pan.y)
        };

        // fond
        painter.rect_filled(rect, 0.0, COL_BG);
        if let Some(c) = bg {
            let a = to_screen(Pos2::ZERO, zoom, pan);
            let b = to_screen(Pos2::new(lw, lh), zoom, pan);
            painter.rect_filled(Rect::from_two_pos(a, b), 0.0, c);
        }

        // grille
        if zoom >= 0.18 {
            let step = 128.0;
            let w0 = to_world(rect.left_top(), zoom, pan);
            let w1 = to_world(rect.right_bottom(), zoom, pan);
            let mut gx = (w0.x / step).floor() * step;
            while gx <= w1.x {
                let s = to_screen(Pos2::new(gx, 0.0), zoom, pan);
                painter.line_segment([Pos2::new(s.x, rect.top()), Pos2::new(s.x, rect.bottom())], Stroke::new(1.0, COL_GRID));
                gx += step;
            }
            let mut gy = (w0.y / step).floor() * step;
            while gy <= w1.y {
                let s = to_screen(Pos2::new(0.0, gy), zoom, pan);
                painter.line_segment([Pos2::new(rect.left(), s.y), Pos2::new(rect.right(), s.y)], Stroke::new(1.0, COL_GRID));
                gy += step;
            }
        }

        // interactions
        let mut next_view = (zoom, pan);
        if resp.hovered() {
            let scroll = ui.input(|i| i.raw_scroll_delta);
            if scroll.y.abs() > 0.01 {
                let factor = (-scroll.y / 400.0).exp();
                let mouse = ui.input(|i| i.pointer.latest_pos().unwrap_or(rect.center()));
                let before = to_world(mouse, zoom, pan);
                let nz = (zoom * factor).clamp(0.05, 8.0);
                let after = to_world(mouse, nz, pan);
                pan.x += before.x - after.x;
                pan.y += before.y - after.y;
                zoom = nz;
                next_view = (zoom, pan);
            }
        }
        if resp.dragged_by(egui::PointerButton::Middle) || resp.dragged_by(egui::PointerButton::Secondary) {
            let d = resp.drag_delta();
            pan.x -= d.x / zoom;
            pan.y -= d.y / zoom;
            next_view = (zoom, pan);
        }

        // dessin des instances
        let mut hit: Option<usize> = None;
        let mut draw_errs: Vec<String> = Vec::new();
        {
            let ctx2 = ui.ctx().clone();
            for (lai, layer) in p.layouts[li].layers.iter().enumerate() {
                if !layer.visible {
                    continue;
                }
                for inst in &layer.instances {
                    let l = inst.x - inst.ox * inst.w;
                    let t = inst.y - inst.oy * inst.h;
                    let a = to_screen(Pos2::new(l, t), zoom, pan);
                    let b = to_screen(Pos2::new(l + inst.w, t + inst.h), zoom, pan);
                    let srect = Rect::from_two_pos(a, b);
                    if inst.uid == self.sel.unwrap_or(usize::MAX) {
                        hit = Some(inst.uid);
                    }
                    // clic : au survol, mémorise le candidat (ordre inverse géré par la boucle)
                    match p.type_of(&inst.type_name).map(|t| t.plugin) {
                        Some(Plugin::Sprite) | Some(Plugin::Other) => {
                            let key = p
                                .type_of(&inst.type_name)
                                .and_then(|t| t.anims.first())
                                .and_then(|an| an.frames.first().map(|f| f.image.clone()))
                                .unwrap_or_default();
                            match load_tex_cached(&mut self.tex, &ctx2, p, &key) {
                                Some(tex) => {
                                    let center = srect.center();
                                    add_rotated_quad(&painter, &tex, center, srect.width(), srect.height(), inst.angle, false);
                                }
                                None => {
                                    painter.rect_filled(srect, 2.0, color_from_name(&inst.type_name));
                                    painter.rect_stroke(srect, 2.0, Stroke::new(1.0, COL_DIM));
                                }
                            }
                        }
                        Some(Plugin::TiledBg) => {
                            let key = p
                                .type_of(&inst.type_name)
                                .and_then(|t| t.anims.first())
                                .and_then(|an| an.frames.first().map(|f| f.image.clone()))
                                .unwrap_or_default();
                            match load_tex_cached(&mut self.tex, &ctx2, p, &key) {
                                Some(tex) => {
                                    let ts = tex.size_vec2();
                                    if ts.x > 0.0 && ts.y > 0.0 {
                                        let mut y = srect.top();
                                        while y < srect.bottom() - 0.5 {
                                            let mut x = srect.left();
                                            while x < srect.right() - 0.5 {
                                                let tw = (ts.x * zoom).min(srect.right() - x);
                                                let th = (ts.y * zoom).min(srect.bottom() - y);
                                                add_quad_uv(&painter, &tex, Pos2::new(x, y), tw, th, [0.0, 0.0, (tw / (ts.x * zoom)).min(1.0) * ts.x, (th / (ts.y * zoom)).min(1.0) * ts.y]);
                                                x += ts.x * zoom;
                                            }
                                            y += ts.y * zoom;
                                        }
                                    }
                                }
                                None => {
                                    painter.rect_filled(srect, 0.0, color_from_name(&inst.type_name));
                                }
                            }
                        }
                        Some(Plugin::Text) => {
                            let text = inst
                                .props
                                .get("text")
                                .map(|v| v.text())
                                .unwrap_or_else(|| "Texte".into());
                            let size = inst
                                .props
                                .get("font-size")
                                .map(|v| v.num() as f32)
                                .unwrap_or(20.0);
                            let col = inst
                                .props
                                .get("color")
                                .and_then(|v| v.text().split(',').next().map(|s| s.to_string()))
                                .and_then(|s| parse_hex_color(&s))
                                .map(|c| Color32::from_rgb((c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8))
                                .unwrap_or(Color32::WHITE);
                            painter.text(
                                srect.left_top(),
                                egui::Align2::LEFT_TOP,
                                text,
                                egui::FontId::proportional((size * zoom).max(6.0)),
                                col,
                            );
                        }
                        _ => {
                            painter.rect_filled(srect, 2.0, color_from_name(&inst.type_name));
                        }
                    }
                    // bbox de sélection
                    if Some(inst.uid) == self.sel {
                        painter.rect_stroke(srect, 0.0, Stroke::new(1.5, COL_SEL));
                        let c = to_screen(Pos2::new(inst.x, inst.y), zoom, pan);
                        painter.line_segment([Pos2::new(c.x - 6.0, c.y), Pos2::new(c.x + 6.0, c.y)], Stroke::new(1.0, COL_ACCENT));
                        painter.line_segment([Pos2::new(c.x, c.y - 6.0), Pos2::new(c.x, c.y + 6.0)], Stroke::new(1.0, COL_ACCENT));
                    }
                    let _ = lai;
                }
            }
        }

        // sélection au clic (instance la plus proche du clic)
        if resp.clicked() {
            if let Some(mouse) = resp.interact_pointer_pos() {
                let w = to_world(mouse, zoom, pan);
                if let Tool::Place(name) = self.tool.clone() {
                    self.place_instance(li, &name, w.x, w.y);
                } else {
                    // topmost sous le curseur
                    let mut best: Option<usize> = None;
                    for layer in p.layouts[li].layers.iter().rev() {
                        for inst in layer.instances.iter().rev() {
                            let l = inst.x - inst.ox * inst.w;
                            let t = inst.y - inst.oy * inst.h;
                            if w.x >= l && w.x <= l + inst.w && w.y >= t && w.y <= t + inst.h {
                                best = Some(inst.uid);
                                break;
                            }
                        }
                        if best.is_some() {
                            break;
                        }
                    }
                    self.sel = best;
                }
            }
        }

        // déplacement de la sélection
        if resp.dragged_by(egui::PointerButton::Primary) {
            if let Some(uid) = self.sel {
                if matches!(self.tool, Tool::Select) {
                    let d = resp.drag_delta();
                    if let Some(p) = self.project.as_mut() {
                        'outer: for l in &mut p.layouts {
                            for layer in &mut l.layers {
                                for inst in &mut layer.instances {
                                    if inst.uid == uid {
                                        inst.x += d.x / zoom;
                                        inst.y += d.y / zoom;
                                        self.dirty = true;
                                        break 'outer;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // suppression
        if ui.input(|i| i.key_pressed(egui::Key::Delete)) && self.sel.is_some() {
            let uid = self.sel.unwrap();
            if let Some(p) = self.project.as_mut() {
                'outer: for l in &mut p.layouts {
                    for layer in &mut l.layers {
                        if let Some(k) = layer.instances.iter().position(|i| i.uid == uid) {
                            layer.instances.remove(k);
                            break 'outer;
                        }
                    }
                }
            }
            self.sel = None;
            self.dirty = true;
            self.say(0, "instance supprimee");
        }

        self.views.insert(li, next_view);
        let _ = hit;
        let _ = draw_errs;

        // infos d'aide
        painter.text(
            rect.left_bottom() + Vec2::new(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            "molette: zoom | clic-milieu/droit: deplacer | clic: selectionner/placer | Suppr: supprimer",
            egui::FontId::proportional(11.0),
            COL_DIM,
        );
    }

    fn place_instance(&mut self, li: usize, name: &str, wx: f32, wy: f32) {
        let Some(p) = self.project.as_mut() else { return };
        if li >= p.layouts.len() {
            return;
        }
        let t = match p.type_of(name) {
            Some(t) => t.clone(),
            None => return,
        };
        let (w, h) = t
            .anims
            .first()
            .and_then(|a| a.frames.first())
            .and_then(|f| p.images.get(&f.image))
            .map(|tex| (tex.w as f32, tex.h as f32))
            .unwrap_or((64.0, 64.0));
        let layer = self.place_layer.min(p.layouts[li].layers.len().saturating_sub(1));
        let mut props = std::collections::HashMap::new();
        if t.plugin == Plugin::Text {
            props.insert("text".to_string(), Val::Str("Texte".into()));
            props.insert("font-size".to_string(), Val::Num(20.0));
        }
        let uid = self.next_uid;
        self.next_uid += 1;
        p.layouts[li].layers[layer].instances.push(InstSpec {
            type_name: name.to_string(),
            x: wx,
            y: wy,
            w,
            h,
            angle: 0.0,
            ox: 0.5,
            oy: 0.5,
            opacity: 1.0,
            layer,
            vars: vec![],
            props,
            behaviors: vec![],
            uid,
        });
        self.sel = Some(uid);
        self.dirty = true;
        self.say(0, format!("« {name} » place ({wx:.0}, {wy:.0})"));
    }

    // ---------------- evenements ----------------

    fn show_events_tab(&mut self, ui: &mut egui::Ui, si: usize) {
        let Some(p) = self.project.as_ref() else {
            ui.centered_and_justified(|ui| ui.label("Aucun projet."));
            return;
        };
        if si >= p.sheets.len() {
            ui.centered_and_justified(|ui| ui.label("Feuille introuvable."));
            return;
        }
        ui.label(format!("Feuille « {} » — {} evenement(s) racine", p.sheets[si].name, p.sheets[si].events.len()));
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            for ev in &p.sheets[si].events {
                render_event(ui, ev, 0);
            }
        });
    }

    // ---------------- medias ----------------

    fn show_media_tab(&mut self, ui: &mut egui::Ui) {
        ui.label(format!("Bibliotheque de medias — {} fichier(s) importe(s). Tout type de fichier est accepte.", self.media.len()));
        ui.separator();
        let mut remove: Option<usize> = None;
        let mut to_sprite: Option<usize> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.media.is_empty() {
                ui.label("Aucun média. Glissez-déposez des fichiers dans la fenêtre, ou utilisez Importer…");
                ui.label("Images : PNG/JPEG/BMP/GIF/WebP -> objet sprite plaçable. Audio : WAV/OGG/MP3… Polices : TTF/OTF.");
            }
            for (i, a) in self.media.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.monospace(format!("[{}]", a.kind.label()));
                    ui.label(&a.name);
                    ui.label(format!("{:.1} Ko", a.size_kb()));
                    if let Some(tex) = &a.tex {
                        let _ = tex;
                        ui.label("sprite pret");
                    }
                    if a.kind == AssetKind::Image && ui.small_button("Ajouter comme objet").clicked() {
                        to_sprite = Some(i);
                    }
                    if ui.small_button("Retirer").clicked() {
                        remove = Some(i);
                    }
                });
                ui.separator();
            }
        });
        if let Some(i) = to_sprite {
            if let Some(a) = self.media.get(i) {
                if let Some(tex) = &a.tex {
                    let stem = a.name.rsplit('.').nth(1).unwrap_or(&a.name).to_string();
                    self.add_object_from_image(&stem, tex.clone());
                } else {
                    self.say(1, "texture non chargee pour ce media");
                }
            }
        }
        if let Some(i) = remove {
            self.media.remove(i);
        }
    }

    // ---------------- apercu ----------------

    fn show_preview_tab(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        if self.preview.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    if ui.add(egui::Button::new(egui::RichText::new("Lancer l'apercu (F5)").size(18.0))).clicked() {
                        self.run_preview();
                    }
                    ui.add_space(8.0);
                    ui.label("L'apercu execute le layout courant avec le moteur c3rs (comportements + evenements).");
                });
            });
            return;
        }
        self.build_preview_input(ctx);
        let fb = {
            let Some(eng) = self.preview.as_mut() else { return };
            eng.tick(1.0 / 60.0);
            render_frame(eng)
        };
        let (vw, vh) = (fb.w, fb.h);
        // conversion ARGB -> RGBA
        let mut rgba = Vec::with_capacity(fb.px.len() * 4);
        for &px in &fb.px {
            rgba.push(((px >> 16) & 0xFF) as u8);
            rgba.push(((px >> 8) & 0xFF) as u8);
            rgba.push((px & 0xFF) as u8);
            rgba.push(255);
        }
        let img = ColorImage::from_rgba_unmultiplied([vw.max(1), vh.max(1)], &rgba);
        let needs_recreate = match &self.preview_tex {
            Some(t) => t.size() != [vw.max(1), vh.max(1)],
            None => true,
        };
        let tex = if needs_recreate {
            let t = ctx.load_texture("preview", img, TextureOptions::LINEAR);
            self.preview_tex = Some(t.clone());
            t
        } else {
            let t = self.preview_tex.as_mut().unwrap();
            t.set(img, TextureOptions::LINEAR);
            t.clone()
        };
        let avail = ui.available_size();
        let (rect, _resp) = ui.allocate_exact_size(avail, Sense::hover());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, COL_BG);
        let scale = (rect.width() / vw as f32).min(rect.height() / vh as f32);
        let dw = vw as f32 * scale;
        let dh = vh as f32 * scale;
        let dst = Rect::from_min_size(Pos2::new(rect.center().x - dw / 2.0, rect.center().y - dh / 2.0), Vec2::new(dw, dh));
        let ts = tex.size_vec2();
        add_quad_uv(&painter, &tex, dst.min, dw, dh, [0.0, 0.0, ts.x, ts.y]);
        // coordonnées souris -> vue
        if let Some(mp) = ui.input(|i| i.pointer.latest_pos()) {
            if let Some(eng) = self.preview.as_mut() {
                let vx = ((mp.x - dst.min.x) / scale).max(0.0);
                let vy = ((mp.y - dst.min.y) / scale).max(0.0);
                eng.input.mouse = (vx, vy);
            }
        }
        painter.text(
            rect.left_top() + Vec2::new(8.0, 8.0),
            egui::Align2::LEFT_TOP,
            "Apercu — Echap ou F5 pour arreter",
            egui::FontId::proportional(11.0),
            COL_WARN,
        );
        let warn_n = self.preview.as_ref().map(|e| e.warnings.len()).unwrap_or(0);        if warn_n > 0 {
            painter.text(
                rect.left_top() + Vec2::new(8.0, 24.0),
                egui::Align2::LEFT_TOP,
                format!("{warn_n} avertissement(s) runtime (voir console)"),
                egui::FontId::proportional(11.0),
                COL_ERR,
            );
        }
        let _ = COL_SEL;
    }
}

fn load_tex_cached(tex_cache: &mut HashMap<String, egui::TextureHandle>, ctx: &egui::Context, proj: &Project, key: &str) -> Option<egui::TextureHandle> {
    if let Some(t) = tex_cache.get(key) {
        return Some(t.clone());
    }
    let t = proj.images.get(key)?;
    let img = egui::ColorImage::from_rgba_unmultiplied([t.w.max(1) as usize, t.h.max(1) as usize], &t.rgba);
    let handle = ctx.load_texture(format!("img:{key}"), img, TextureOptions::LINEAR);
    tex_cache.insert(key.to_string(), handle.clone());
    Some(handle)
}

fn render_event(ui: &mut egui::Ui, ev: &Event, depth: usize) {
    match ev {
        Event::Block { conditions, actions, children, disabled, .. } => {
            let title = if conditions.is_empty() {
                "Toujours".to_string()
            } else {
                conditions.iter().map(cond_label).collect::<Vec<_>>().join("  ET  ")
            };
            row(ui, depth, if *disabled { COL_DIM } else { COL_COND }, &format!("QUAND  {title}"), if *disabled { "desactive" } else { "" });
            for a in actions {
                row(ui, depth + 1, COL_ACT, &format!("ALORS {}", act_label(a)), "");
            }
            for c in children {
                render_event(ui, c, depth + 2);
            }
        }
        Event::Else { conditions, actions, children, disabled, .. } => {
            let title = if conditions.is_empty() {
                String::new()
            } else {
                conditions.iter().map(cond_label).collect::<Vec<_>>().join("  ET  ")
            };
            row(ui, depth, if *disabled { COL_DIM } else { COL_WARN }, &format!("SINON {title}"), "");
            for a in actions {
                row(ui, depth + 1, COL_ACT, &format!("ALORS {}", act_label(a)), "");
            }
            for c in children {
                render_event(ui, c, depth + 2);
            }
        }
        Event::Group { title, children, disabled, .. } => {
            row(ui, depth, COL_RAISED0(), &format!("GROUPE {title}{}", if *disabled { " (inactif)" } else { "" }), "");
            for c in children {
                render_event(ui, c, depth + 1);
            }
        }
        Event::Comment => {}
        Event::Var { name, value, is_constant } => {
            row(ui, depth, COL_RAISED0(), &format!("VARIABLE {name} = {}{}", value.text(), if *is_constant { " (constante)" } else { "" }), "");
        }
        Event::Include { sheet } => {
            row(ui, depth, COL_RAISED0(), &format!("INCLUT la feuille « {sheet} »"), "");
        }
        Event::Function { name, children } => {
            row(ui, depth, COL_RAISED0(), &format!("FONCTION {name}"), "");
            for c in children {
                render_event(ui, c, depth + 1);
            }
        }
        Event::Unknown => {}
    }
}

fn COL_RAISED0() -> Color32 {
    Color32::from_rgb(0x35, 0x39, 0x40)
}

fn row(ui: &mut egui::Ui, depth: usize, color: Color32, text: &str, note: &str) {
    ui.horizontal(|ui| {
        ui.add_space((depth * 16) as f32);
        let label = if note.is_empty() { text.to_string() } else { format!("{text}   [{note}]") };
        egui::Frame::none()
            .fill(color)
            .rounding(3.0)
            .inner_margin(egui::Margin::symmetric(6.0, 3.0))
            .show(ui, |ui| {
                ui.label(egui::RichText::new(label).size(12.0));
            });
    });
}

fn cond_label(c: &Cond) -> String {
    let mut s = if c.object == "System" { String::new() } else { format!("{} ", c.object) };
    s.push_str(&humanize(&c.id));
    if !c.params.is_empty() {
        let ps: Vec<String> = c.params.iter().map(|(k, v)| format!("{k}={}", short(v, 24))).collect();
        s.push_str(&format!(" ({})", ps.join(", ")));
    }
    s
}

fn act_label(a: &Act) -> String {
    let mut s = if a.object == "System" { String::new() } else { format!("{} ", a.object) };
    s.push_str(&humanize(&a.id));
    if !a.params.is_empty() {
        let ps: Vec<String> = a.params.iter().map(|(k, v)| format!("{k}={}", short(v, 24))).collect();
        s.push_str(&format!(" ({})", ps.join(", ")));
    }
    s
}

fn humanize(id: &str) -> String {
    id.split('-').map(|w| capitalize(w)).collect::<Vec<_>>().join(" ")
}

fn capitalize(w: &str) -> String {
    let mut c = w.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{t}…")
    }
}

// ---------------- dessin mesh ----------------

fn add_rotated_quad(painter: &egui::Painter, tex: &egui::TextureHandle, center: Pos2, w: f32, h: f32, angle_deg: f32, mirror: bool) {
    let ts = tex.size_vec2();
    add_quad_uv_rot(painter, tex, center, w, h, angle_deg, mirror, [0.0, 0.0, ts.x, ts.y]);
}

fn add_quad_uv(painter: &egui::Painter, tex: &egui::TextureHandle, top_left: Pos2, w: f32, h: f32, uv: [f32; 4]) {
    add_quad_uv_rot(painter, tex, Pos2::new(top_left.x + w / 2.0, top_left.y + h / 2.0), w, h, 0.0, false, uv);
}

#[allow(clippy::too_many_arguments)]
fn add_quad_uv_rot(painter: &egui::Painter, tex: &egui::TextureHandle, center: Pos2, w: f32, h: f32, angle_deg: f32, mirror: bool, uv: [f32; 4]) {
    let mut mesh = egui::Mesh::with_texture(tex.id());
    let (hw, hh) = (w / 2.0, h / 2.0);
    let rad = angle_deg.to_radians();
    let (s, c) = rad.sin_cos();
    let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];
    let mut uvs = [(uv[0], uv[1]), (uv[2], uv[1]), (uv[2], uv[3]), (uv[0], uv[3])];
    if mirror {
        uvs = [(uv[2], uv[1]), (uv[0], uv[1]), (uv[0], uv[3]), (uv[2], uv[3])];
    }
    let base = mesh.vertices.len() as u32;
    for (i, (dx, dy)) in corners.iter().enumerate() {
        let px = center.x + dx * c - dy * s;
        let py = center.y + dx * s + dy * c;
        mesh.vertices.push(egui::epaint::Vertex { pos: Pos2::new(px, py), uv: Pos2::new(uvs[i].0, uvs[i].1), color: Color32::WHITE });
    }
    mesh.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    painter.add(Shape::Mesh(mesh));
}

fn color_from_name(s: &str) -> Color32 {
    let mut h: u32 = 2166136261;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    let r = 90 + (h & 0x7F) as u8;
    let g = 90 + ((h >> 8) & 0x7F) as u8;
    let b = 90 + ((h >> 16) & 0x7F) as u8;
    Color32::from_rgb(r, g, b)
}

fn parse_hex_color(s: &str) -> Option<[f32; 4]> {
    let s = s.trim().trim_start_matches('#');
    let cs: Vec<char> = s.chars().collect();
    if cs.len() < 6 {
        return None;
    }
    let hx = |cs: &[char]| -> Option<f32> {
        let a = u8::from_str_radix(&cs[0].to_string(), 16).ok()?;
        let b = u8::from_str_radix(&cs[1].to_string(), 16).ok()?;
        Some((a * 16 + b) as f32 / 255.0)
    };
    let r = hx(&cs[0..2])?;
    let g = hx(&cs[2..4])?;
    let b = hx(&cs[4..6])?;
    Some([r, g, b, 1.0])
}
