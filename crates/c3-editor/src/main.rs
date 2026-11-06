//! c3-editor — serveur local de l'éditeur (style Construct 3).
//! Sert l'UI web embarquée et expose l'API : ouverture .c3p, import de
//! N'IMPORTE QUEL fichier, sauvegarde .c3p, export HTML5 autonome.

use c3_core::{
    default_project, detect_kind, decode_data_uri, export_html, import_asset, parse_c3p, save_c3p,
    Asset, Project,
};
use std::io::Read;
use std::sync::Mutex;
use tiny_http::{Header, Method, Response, Server};

static INDEX_HTML: &[u8] = include_bytes!("../../../web/index.html");
static EDITOR_CSS: &[u8] = include_bytes!("../../../web/editor.css");
static EDITOR_JS: &[u8] = include_bytes!("../../../web/editor.js");
static PLAYER_JS: &[u8] = include_bytes!("../../../web/player.js");

struct St {
    project: Project,
    warnings: Vec<String>,
    aid: u64,
}

fn hdr(k: &'static str, v: &'static str) -> Header {
    Header::from_bytes(k.as_bytes(), v.as_bytes()).unwrap()
}

fn send(req: tiny_http::Request, code: u16, ctype: &'static str, body: Vec<u8>, extra: Option<Header>) {
    let mut resp = Response::from_data(body).with_status_code(code);
    resp.add_header(hdr("Content-Type", ctype));
    resp.add_header(hdr("Cache-Control", "no-store"));
    if let Some(h) = extra {
        resp.add_header(h);
    }
    let _ = req.respond(resp);
}

fn send_json(req: tiny_http::Request, code: u16, v: serde_json::Value) {
    send(req, code, "application/json; charset=utf-8", v.to_string().into_bytes(), None);
}

fn percent_decode(s: &str) -> String {
    let s = s.replace('+', " ");
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() + 1 && i + 2 < b.len() {
            let hx = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok());
            if let Some(v) = hx {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn query_param<'a>(query: &'a str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            return Some(percent_decode(it.next().unwrap_or("")));
        }
    }
    None
}

fn ascii_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' { c } else { '_' })
        .collect()
}

fn default_headers() -> Vec<Header> {
    vec![
        hdr("Access-Control-Allow-Origin", "*"),
        hdr("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
        hdr("Access-Control-Allow-Headers", "Content-Type"),
    ]
}

fn main() {
    let port: u16 = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("C3R_PORT").ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr).expect("impossible d'écouter le port");
    println!("==============================================");
    println!("  C3R — Construct 3 Rust Edition  v0.1.0");
    println!("  Éditeur :  http://localhost:{}/", port);
    println!("  (Ctrl+C pour arrêter)");
    println!("==============================================");

    let state = Mutex::new(St {
        project: default_project(),
        warnings: Vec::new(),
        aid: 0,
    });

    for mut request in server.incoming_requests() {
        let method = request.method().clone();
        let url = request.url().to_string();
        let (path, query) = match url.split_once('?') {
            Some((p, q)) => (p.to_string(), q.to_string()),
            None => (url.clone(), String::new()),
        };

        if method == Method::Options {
            let mut resp = Response::from_data(Vec::new());
            for h in default_headers() {
                resp.add_header(h);
            }
            let _ = request.respond(resp);
            continue;
        }

        match (&method, path.as_str()) {
            (Method::Get, "/") | (Method::Get, "/index.html") => {
                send(request, 200, "text/html; charset=utf-8", INDEX_HTML.to_vec(), None);
            }
            (Method::Get, "/s/editor.css") => {
                send(request, 200, "text/css; charset=utf-8", EDITOR_CSS.to_vec(), None);
            }
            (Method::Get, "/s/editor.js") => {
                send(request, 200, "application/javascript; charset=utf-8", EDITOR_JS.to_vec(), None);
            }
            (Method::Get, "/s/player.js") => {
                send(request, 200, "application/javascript; charset=utf-8", PLAYER_JS.to_vec(), None);
            }
            (Method::Get, "/api/state") => {
                let st = state.lock().unwrap();
                send_json(
                    request,
                    200,
                    serde_json::json!({ "ok": true, "project": st.project, "warnings": st.warnings }),
                );
            }
            (Method::Post, "/api/new") => {
                let mut st = state.lock().unwrap();
                st.project = default_project();
                st.warnings = vec!["Nouveau projet créé.".into()];
                send_json(request, 200, serde_json::json!({ "ok": true, "project": st.project }));
            }
            (Method::Post, "/api/open") => {
                let mut body = Vec::new();
                let _ = request.as_reader().read_to_end(&mut body);
                match parse_c3p(&body) {
                    Ok(res) => {
                        let mut st = state.lock().unwrap();
                        st.project = res.project;
                        st.warnings = res.warnings;
                        let resp = serde_json::json!({ "ok": true, "project": st.project, "warnings": st.warnings });
                        send_json(request, 200, resp);
                    }
                    Err(e) => send_json(request, 400, serde_json::json!({ "ok": false, "error": e })),
                }
            }
            (Method::Post, "/api/import") => {
                let mut body = Vec::new();
                let _ = request.as_reader().read_to_end(&mut body);
                let name = query_param(&query, "name").unwrap_or_else(|| "fichier.bin".into());
                let mut st = state.lock().unwrap();
                st.aid += 1;
                let mut imported: Vec<Asset> = Vec::new();
                let (kind, _) = detect_kind(&name, &body);
                if kind == "archive" {
                    // Import d'un zip/.c3p : on extrait tous les médias dedans
                    if let Ok(res) = parse_c3p(&body) {
                        let n_assets = res.project.assets.len();
                        for a in res.project.assets {
                            imported.push(a);
                        }
                        st.warnings = vec![format!("Archive « {} » : {} ressource(s) extraite(s).", name, n_assets)];
                    }
                    if let Ok(mut za) = zip::ZipArchive::new(std::io::Cursor::new(&body)) {
                        for i in 0..za.len() {
                            if let Ok(mut f) = za.by_index(i) {
                                if f.is_dir() { continue; }
                                let fn_ = f.name().to_string();
                                let low = fn_.to_ascii_lowercase();
                                if low.ends_with(".json") || low.ends_with(".txt") || low.ends_with('/') { continue; }
                                let mut buf = Vec::new();
                                if f.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                                    let (k, _) = detect_kind(&fn_, &buf);
                                    if k == "image" || k == "audio" || k == "video" || k == "font" {
                                        st.aid += 1;
                                        imported.push(import_asset(format!("a{}", st.aid), &fn_, &buf));
                                    }
                                }
                            }
                        }
                    }
                    if imported.is_empty() {
                        st.warnings = vec![format!("« {} » : aucun média extractible trouvé.", name)];
                    }
                } else {
                    imported.push(import_asset(format!("a{}", st.aid), &name, &body));
                    st.warnings = Vec::new();
                }
                let mut ids = Vec::new();
                for a in imported {
                    st.project.assets.push(a.clone());
                    ids.push(a.id);
                }
                send_json(
                    request,
                    200,
                    serde_json::json!({ "ok": true, "ids": ids, "project": st.project, "warnings": st.warnings }),
                );
            }
            (Method::Post, "/api/project") => {
                let mut body = Vec::new();
                let _ = request.as_reader().read_to_end(&mut body);
                match serde_json::from_slice::<Project>(&body) {
                    Ok(p) => {
                        let mut st = state.lock().unwrap();
                        st.project = p;
                        send_json(request, 200, serde_json::json!({ "ok": true }));
                    }
                    Err(e) => send_json(request, 400, serde_json::json!({ "ok": false, "error": e.to_string() })),
                }
            }
            (Method::Post, "/api/save") => {
                let st = state.lock().unwrap();
                match save_c3p(&st.project) {
                    Ok(bytes) => {
                        let fname = format!("attachment; filename=\"{}.c3p\"", ascii_name(&st.project.name));
                        send(
                            request,
                            200,
                            "application/zip",
                            bytes,
                            Some(hdr("Content-Disposition", Box::leak(fname.into_boxed_str()))),
                        );
                    }
                    Err(e) => send_json(request, 500, serde_json::json!({ "ok": false, "error": e })),
                }
            }
            (Method::Post, "/api/export") => {
                let st = state.lock().unwrap();
                match export_html(&st.project, std::str::from_utf8(PLAYER_JS).unwrap()) {
                    Ok(bytes) => {
                        let fname = format!("attachment; filename=\"{}.html\"", ascii_name(&st.project.name));
                        send(
                            request,
                            200,
                            "text/html; charset=utf-8",
                            bytes,
                            Some(hdr("Content-Disposition", Box::leak(fname.into_boxed_str()))),
                        );
                    }
                    Err(e) => send_json(request, 500, serde_json::json!({ "ok": false, "error": e })),
                }
            }
            (Method::Get, p) if p.starts_with("/api/asset/") => {
                let id = &p["/api/asset/".len()..];
                let st = state.lock().unwrap();
                match st.project.assets.iter().find(|a| a.id == id) {
                    Some(a) => match decode_data_uri(&a.data) {
                        Some(bytes) => {
                            let ct: &'static str = Box::leak(a.mime.clone().into_boxed_str());
                            send(request, 200, ct, bytes, None)
                        }
                        None => send_json(request, 404, serde_json::json!({ "ok": false, "error": "données absentes" })),
                    },
                    None => send_json(request, 404, serde_json::json!({ "ok": false, "error": "asset inconnu" })),
                }
            }
            _ => {
                send_json(request, 404, serde_json::json!({ "ok": false, "error": "route inconnue" }));
            }
        }
    }
}
