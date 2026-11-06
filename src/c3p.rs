//! Ouverture d'un fichier .c3p (archive ZIP) et assemblage du projet.

use crate::expr::Val;
use crate::model::*;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

pub fn load(path: &Path) -> Result<(Project, Vec<String>), String> {
    let bytes = std::fs::read(path).map_err(|e| format!("impossible d'ouvrir {}: {e}", path.display()))?;
    load_bytes(&bytes)
}

/// Charge un projet .c3p depuis des octets en mémoire (drag & drop, import).
pub fn load_bytes(bytes: &[u8]) -> Result<(Project, Vec<String>), String> {
    let rd = std::io::Cursor::new(bytes.to_vec());
    let mut zip = zip::ZipArchive::new(rd).map_err(|e| format!("archive ZIP invalide: {e}"))?;

    // lecture de toutes les entrées (backslashes -> slashes)
    let mut entries: HashMap<String, Vec<u8>> = HashMap::new();
    for i in 0..zip.len() {
        let mut zf = zip.by_index(i).map_err(|e| e.to_string())?;
        if zf.is_dir() {
            continue;
        }
        let raw_name = zf.name().to_string();
        let name = raw_name.replace('\\', "/");
        let mut data = Vec::with_capacity(zf.size() as usize);
        zf.read_to_end(&mut data).map_err(|e| e.to_string())?;
        entries.insert(name, data);
    }

    let mut warnings: Vec<String> = Vec::new();

    // --- fichier projet ---
    let proj_key = ["project.c3proj", "project.json"]
        .iter()
        .find(|k| entries.contains_key(**k))
        .ok_or("project.c3proj introuvable dans l'archive (.c3p invalide ?)")?
        .to_string();
    let proj_bytes = entries.get(&proj_key).unwrap();
    let proj_doc: serde_json::Value = serde_json::from_slice(proj_bytes)
        .map_err(|e| format!("{proj_key} illisible: {e}"))?;
    let (name, vw, vh, first, bg) = parse_project_json(&proj_doc, &mut warnings);

    // --- object types ---
    let mut types: Vec<ObjType> = Vec::new();
    let mut type_docs: Vec<(String, serde_json::Value)> = Vec::new();
    for (k, v) in &entries {
        if k.starts_with("objectTypes/") && k.ends_with(".json") && !k.contains(".uistate") {
            match serde_json::from_slice(v) {
                Ok(doc) => type_docs.push((k.clone(), doc)),
                Err(e) => warnings.push(format!("{k}: JSON invalide ({e})")),
            }
        }
    }
    type_docs.sort_by(|a, b| a.0.cmp(&b.0));
    for (k, doc) in &type_docs {
        let mut t = parse_object_type(doc, &mut warnings);
        // nom de secours depuis le chemin
        if t.name.is_empty() || t.name == "Objet" {
            t.name = k.rsplit('/').next().unwrap_or(k).trim_end_matches(".json").to_string();
        }
        types.push(t);
    }

    // --- familles ---
    for (k, v) in &entries {
        if k.starts_with("families/") && k.ends_with(".json") && !k.contains(".uistate") {
            if let Ok(doc) = serde_json::from_slice::<serde_json::Value>(v) {
                let mut t = parse_object_type(&doc, &mut warnings);
                if t.name.is_empty() || t.name == "Objet" {
                    t.name = k.rsplit('/').next().unwrap_or(k).trim_end_matches(".json").to_string();
                }
                t.is_family = true;
                types.push(t);
            }
        }
    }

    // --- layouts ---
    let mut layouts: Vec<Layout> = Vec::new();
    let mut layout_keys: Vec<&String> = entries
        .keys()
        .filter(|k| k.starts_with("layouts/") && k.ends_with(".json") && !k.contains(".uistate"))
        .collect();
    layout_keys.sort();
    for k in layout_keys {
        let doc: serde_json::Value = serde_json::from_slice(&entries[k])
            .map_err(|e| format!("{k} illisible: {e}"))?;
        layouts.push(parse_layout(&doc, &mut warnings));
    }

    // --- event sheets ---
    let mut sheets: Vec<Sheet> = Vec::new();
    let mut sheet_keys: Vec<&String> = entries
        .keys()
        .filter(|k| {
            (k.starts_with("eventSheets/") || k.starts_with("eventtables/") || k.starts_with("eventTables/"))
                && k.ends_with(".json")
                && !k.contains(".uistate")
        })
        .collect();
    sheet_keys.sort();
    for k in sheet_keys {
        let doc: serde_json::Value = serde_json::from_slice(&entries[k])
            .map_err(|e| format!("{k} illisible: {e}"))?;
        let sheet_name = doc
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                k.rsplit('/')
                    .next()
                    .unwrap_or(k)
                    .trim_end_matches(".json")
                    .to_string()
            });
        sheets.push(parse_sheet(&sheet_name, &doc, &mut warnings));
    }

    // --- images ---
    let mut images: HashMap<String, Tex> = HashMap::new();
    for (k, v) in &entries {
        if k.starts_with("images/") && k.to_lowercase().ends_with(".png") {
            match decode_png(v) {
                Ok(tex) => {
                    let base = k.rsplit('/').next().unwrap_or(k).to_lowercase();
                    images.insert(base, tex);
                }
                Err(e) => warnings.push(format!("{k}: PNG illisible ({e})")),
            }
        }
    }

    // résolution des frames -> images par heuristique
    let img_keys: Vec<String> = images.keys().cloned().collect();
    for t in &mut types {
        if t.is_family {
            continue;
        }
        let obj = normalize(&t.name);
        for a in &mut t.anims {
            let anim = normalize(&a.name);
            let n = a.frames.len();
            for i in 0..n {
                let mut candidates: Vec<String> = Vec::new();
                candidates.push(format!("{obj}-{anim}-{i:03}.png"));
                candidates.push(format!("{obj}-{anim}-{i}.png"));
                if n > 1 {
                    candidates.push(format!("{obj}-{anim}-{i:03}.png"));
                    candidates.push(format!("{obj}-{anim}-{i}.png"));
                }
                if anim == "default" {
                    candidates.push(format!("{obj}-{i:03}.png"));
                    candidates.push(format!("{obj}.png"));
                } else {
                    candidates.push(format!("{obj}-{anim}.png"));
                }
                candidates.push(format!("{obj}-{i:03}.png"));
                candidates.push(format!("{obj}.png"));
                let found = candidates.iter().find(|c| images.contains_key(*c)).cloned()
                    .or_else(|| {
                        // recherche floue: basename normalisé
                        let want = candidates[0].clone();
                        img_keys.iter().find(|k| normalize(k) == normalize(&want)).cloned()
                    });
                let final_key = match found {
                    Some(key) => Some(key),
                    None if i == 0 => {
                        warnings.push(format!("image manquante pour {}/{}", t.name, a.name));
                        None
                    }
                    None => a.frames.first().cloned(), // réutilise la frame 0
                };
                if let Some(key) = final_key {
                    a.frames[i] = key;
                }
            }
        }
    }

    // première disposition
    let first_layout = if !first.is_empty() && layouts.iter().any(|l| l.name == first) {
        first
    } else {
        layouts.first().map(|l| l.name.clone()).unwrap_or_default()
    };

    let project = Project { name, viewport_w: vw, viewport_h: vh, first_layout, background: bg, layouts, sheets, types, images };
    Ok((project, warnings))
}

pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn decode_png(data: &[u8]) -> Result<Tex, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    let (w, h) = (info.width, info.height);
    let ct = info.color_type;
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    match ct {
        png::ColorType::Rgba => rgba = buf[..(w * h * 4) as usize].to_vec(),
        png::ColorType::Rgb => {
            for px in buf[..(w * h * 3) as usize].chunks(3) {
                rgba.extend_from_slice(&[px[0], px[1], px[2], 255]);
            }
        }
        png::ColorType::Grayscale => {
            for px in buf[..(w * h) as usize].iter() {
                rgba.extend_from_slice(&[*px, *px, *px, 255]);
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for px in buf[..(w * h * 2) as usize].chunks(2) {
                rgba.extend_from_slice(&[px[0], px[0], px[0], px[1]]);
            }
        }
        png::ColorType::Indexed => return Err("PNG indexé non supporté".into()),
    }
    Ok(Tex { w, h, rgba })
}

/// Génère une texture de remplacement déterministe (damier coloré).
pub fn placeholder_tex(seed: u64) -> Tex {
    let w = 32u32;
    let h = 32u32;
    let mut hash = seed.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(0x1234_5678);
    let r = (hash >> 8) as f32 / 255.0;
    hash = hash.rotate_left(11);
    let g = (hash >> 8) as f32 / 255.0;
    hash = hash.rotate_left(7);
    let b = (hash >> 8) as f32 / 255.0;
    let base = [((r * 0.6 + 0.25) * 255.0) as u8, ((g * 0.6 + 0.25) * 255.0) as u8, ((b * 0.6 + 0.25) * 255.0) as u8];
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let border = x < 2 || y < 2 || x >= w - 2 || y >= h - 2;
            let f = if border { 0.7 } else { 1.0 };
            rgba.extend_from_slice(&[
                (base[0] as f32 * f) as u8,
                (base[1] as f32 * f) as u8,
                (base[2] as f32 * f) as u8,
                255,
            ]);
        }
    }
    Tex { w, h, rgba }
}

/// Valeur par défaut de propriété numérique
pub fn prop_f(props: &HashMap<String, Val>, keys: &[&str], def: f64) -> f64 {
    for k in keys {
        if let Some(v) = props.get(*k) {
            return v.num();
        }
    }
    def
}

pub fn prop_s(props: &HashMap<String, Val>, keys: &[&str], def: &str) -> String {
    for k in keys {
        if let Some(v) = props.get(*k) {
            return v.text();
        }
    }
    def.to_string()
}
