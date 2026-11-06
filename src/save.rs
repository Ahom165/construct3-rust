//! Écriture d'un projet au format .c3p (ZIP multi-fichiers, format C3 moderne).
//! Produit : project.c3proj, objectTypes/, layouts/, eventSheets/, images/.

use crate::c3p::normalize;
use crate::c3p::placeholder_tex;
use crate::expr::Val;
use crate::model::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

fn val_to_json(v: &Val) -> Value {
    match v {
        Val::Num(n) => json!(n),
        Val::Str(s) => json!(s),
        Val::Bool(b) => json!(b),
    }
}

fn props_to_json(p: &HashMap<String, Val>) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in p {
        m.insert(k.clone(), val_to_json(v));
    }
    Value::Object(m)
}

fn strmap_to_json(p: &HashMap<String, String>) -> Value {
    let mut m = serde_json::Map::new();
    for (k, v) in p {
        m.insert(k.clone(), json!(v));
    }
    Value::Object(m)
}

fn cond_json(c: &Cond, sid: &mut u64) -> Value {
    *sid += 1;
    json!({
        "id": c.id,
        "objectClass": c.object,
        "parameters": strmap_to_json(&c.params),
        "sid": c.sid.max(*sid),
    })
}

fn act_json(a: &Act, sid: &mut u64) -> Value {
    *sid += 1;
    let mut o = json!({
        "id": a.id,
        "objectClass": a.object,
        "parameters": strmap_to_json(&a.params),
        "sid": a.sid.max(*sid),
    });
    if let Some(b) = &a.behavior {
        o["behavior"] = json!(b);
    }
    o
}

fn event_json(ev: &Event, sid: &mut u64) -> Value {
    match ev {
        Event::Block { conditions, actions, children, disabled, sid: s } => {
            *sid = (*sid).max(*s);
            json!({
                "eventType": "block",
                "disabled": disabled,
                "sid": s,
                "conditions": conditions.iter().map(|c| cond_json(c, sid)).collect::<Vec<_>>(),
                "actions": actions.iter().map(|a| act_json(a, sid)).collect::<Vec<_>>(),
                "children": children.iter().map(|e| event_json(e, sid)).collect::<Vec<_>>(),
            })
        }
        Event::Else { conditions, actions, children, disabled, sid: s } => {
            *sid = (*sid).max(*s);
            json!({
                "eventType": "else",
                "disabled": disabled,
                "sid": s,
                "conditions": conditions.iter().map(|c| cond_json(c, sid)).collect::<Vec<_>>(),
                "actions": actions.iter().map(|a| act_json(a, sid)).collect::<Vec<_>>(),
                "children": children.iter().map(|e| event_json(e, sid)).collect::<Vec<_>>(),
            })
        }
        Event::Group { title, active, children, disabled } => json!({
            "eventType": "group",
            "title": title,
            "isActiveOnStart": active,
            "disabled": disabled,
            "children": children.iter().map(|e| event_json(e, sid)).collect::<Vec<_>>(),
        }),
        Event::Comment => json!({ "eventType": "comment" }),
        Event::Var { name, value, is_constant } => {
            let (t, raw) = match value {
                Val::Num(n) => ("number", json!(n)),
                Val::Str(s) => ("string", json!(format!("\"{s}\""))),
                Val::Bool(b) => ("boolean", json!(if *b { 1 } else { 0 })),
            };
            json!({
                "eventType": "variable",
                "name": name,
                "type": t,
                "initialValue": raw,
                "isConstant": is_constant,
            })
        }
        Event::Include { sheet } => json!({ "eventType": "include", "includeSheet": sheet }),
        Event::Function { name, children } => json!({
            "eventType": "function-block",
            "name": name,
            "children": children.iter().map(|e| event_json(e, sid)).collect::<Vec<_>>(),
        }),
        Event::Unknown => json!({ "eventType": "comment" }),
    }
}

fn sheet_json(s: &Sheet, sid: &mut u64) -> Value {
    json!({
        "name": s.name,
        "events": s.events.iter().map(|e| event_json(e, sid)).collect::<Vec<_>>(),
    })
}

fn inst_json(i: &InstSpec) -> Value {
    let mut vars = serde_json::Map::new();
    for (k, v) in &i.vars {
        vars.insert(k.clone(), val_to_json(v));
    }
    let mut behs = serde_json::Map::new();
    for b in &i.behaviors {
        behs.insert(b.name.clone(), json!({ "properties": props_to_json(&b.props) }));
    }
    json!({
        "type": i.type_name,
        "uid": i.uid,
        "world": {
            "x": i.x,
            "y": i.y,
            "width": i.w,
            "height": i.h,
            "angle": i.angle.to_radians(),
            "originX": i.ox,
            "originY": i.oy,
            "color": [1.0, 1.0, 1.0, i.opacity],
        },
        "instanceVariables": Value::Object(vars),
        "properties": props_to_json(&i.props),
        "behaviors": Value::Object(behs),
    })
}

fn type_json(t: &ObjType) -> Value {
    let mut vars = Vec::new();
    for (k, v) in &t.vars {
        let (vt, init) = match v {
            Val::Num(n) => ("number", json!(n)),
            Val::Str(s) => ("string", json!(format!("\"{s}\""))),
            Val::Bool(b) => ("boolean", json!(if *b { 1 } else { 0 })),
        };
        vars.push(json!({ "name": k, "type": vt, "initialValue": init }));
    }
    let behs: Vec<Value> = t
        .behaviors
        .iter()
        .map(|b| json!({ "name": b.name, "properties": props_to_json(&b.props) }))
        .collect();
    let anims: Vec<Value> = t
        .anims
        .iter()
        .map(|a| {
            json!({
                "name": a.name,
                "speed": a.speed,
                "isLooping": a.looping,
                "frames": (0..a.frames.len()).map(|_| json!({})).collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "name": t.name,
        "plugin-id": match t.plugin {
            Plugin::Sprite => "sprite",
            Plugin::TiledBg => "tiledbg",
            Plugin::Text => "text",
            Plugin::Keyboard => "keyboard",
            Plugin::Mouse => "mouse",
            Plugin::Other => "sprite",
        },
        "isFamily": t.is_family,
        "members": t.members,
        "instanceVariables": vars,
        "behaviorTypes": behs,
        "animations": { "items": anims },
        "isGlobal": t.is_global,
    })
}

fn layout_json(l: &Layout) -> Value {
    let layers: Vec<Value> = l
        .layers
        .iter()
        .map(|la| {
            json!({
                "name": la.name,
                "parallaxX": la.parallax.0,
                "parallaxY": la.parallax.1,
                "visible": la.visible,
                "opacity": (la.opacity * 100.0).round(),
                "instances": la.instances.iter().map(inst_json).collect::<Vec<_>>(),
            })
        })
        .collect();
    let mut o = json!({
        "name": l.name,
        "width": l.width,
        "height": l.height,
        "layers": layers,
    });
    if let Some(bg) = l.background {
        o["backgroundColor"] = json!(bg);
    }
    if let Some(es) = &l.event_sheet {
        o["eventSheet"] = json!(es);
    }
    o
}

fn encode_png(tex: &Tex) -> Result<Vec<u8>, String> {
    let mut out: Vec<u8> = Vec::new();
    {
        let mut buf = std::io::Cursor::new(&mut out);
        let mut enc = png::Encoder::new(&mut buf, tex.w.max(1), tex.h.max(1));
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&tex.rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Sauvegarde le projet dans une archive .c3p lisible à la fois par c3rs
/// et (structurellement) proche du format projet de Construct 3.
pub fn save_c3p(project: &Project, path: &Path) -> Result<(), String> {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();

    let proj = json!({
        "name": project.name,
        "viewportWidth": project.viewport_w,
        "viewportHeight": project.viewport_h,
        "firstLayout": project.first_layout,
        "properties": { "backgroundColor": project.background },
    });
    files.push(("project.c3proj".into(), serde_json::to_vec_pretty(&proj).map_err(|e| e.to_string())?));

    for t in &project.types {
        files.push((
            format!("objectTypes/{}.json", t.name),
            serde_json::to_vec_pretty(&type_json(t)).map_err(|e| e.to_string())?,
        ));
    }
    for l in &project.layouts {
        files.push((
            format!("layouts/{}.json", l.name),
            serde_json::to_vec_pretty(&layout_json(l)).map_err(|e| e.to_string())?,
        ));
    }
    let mut sid = 1_000_000u64;
    for s in &project.sheets {
        files.push((
            format!("eventSheets/{}.json", s.name),
            serde_json::to_vec_pretty(&sheet_json(s, &mut sid)).map_err(|e| e.to_string())?,
        ));
    }

    // images : images/<obj>-<anim>-<i3>.png (heuristique de chargement compatible)
    let mut written: HashMap<String, ()> = HashMap::new();
    for t in &project.types {
        if t.is_family {
            continue;
        }
        let obj = normalize(&t.name);
        for a in &t.anims {
            let anim = normalize(&a.name);
            for (i, key) in a.frames.iter().enumerate() {
                let tex = match project.images.get(key) {
                    Some(t) => t.clone(),
                    None => placeholder_tex((obj.len() as u64) * 131 + i as u64 + 7),
                };
                let name = format!("images/{obj}-{anim}-{i:03}.png");
                if written.insert(name.clone(), ()).is_none() {
                    let data = encode_png(&tex)?;
                    files.push((name, data));
                }
            }
        }
    }

    // écriture zip
    let file = std::fs::File::create(path).map_err(|e| format!("création impossible : {e}"))?;
    let mut zw = zip::ZipWriter::new(file);
    for (name, data) in &files {
        zw.start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .map_err(|e| format!("entrée ZIP « {name} » : {e}"))?;
        zw.write_all(data).map_err(|e| e.to_string())?;
    }
    zw.finish().map_err(|e| e.to_string())?;
    Ok(())
}
