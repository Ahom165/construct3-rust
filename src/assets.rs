//! Import universel : n'importe quel fichier peut entrer dans l'éditeur.
//! - images      -> textures + nouvel objet Sprite
//! - .c3p / zip  -> ouverture de projet
//! - audio       -> bibliothèque média (jouable via le runtime cible)
//! - polices     -> police d'affichage de l'éditeur + média
//! - autres      -> bibliothèque média (données brutes conservées)

use crate::c3p;
use crate::model::Tex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetKind {
    Image,
    Audio,
    Font,
    Archive,
    Data,
    Other,
}

impl AssetKind {
    pub fn label(&self) -> &'static str {
        match self {
            AssetKind::Image => "image",
            AssetKind::Audio => "audio",
            AssetKind::Font => "police",
            AssetKind::Archive => "archive",
            AssetKind::Data => "données",
            AssetKind::Other => "fichier",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub kind: AssetKind,
    pub bytes: Vec<u8>,
    pub tex: Option<Tex>,
    pub tex_key: Option<String>,
}

impl Asset {
    pub fn size_kb(&self) -> f64 {
        (self.bytes.len() as f64 / 1024.0 * 10.0).round() / 10.0
    }
}

pub fn classify(name: &str) -> AssetKind {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" => AssetKind::Image,
        "wav" | "ogg" | "mp3" | "m4a" | "flac" | "mod" | "xm" | "it" | "s3m" | "mid" | "midi" => AssetKind::Audio,
        "ttf" | "otf" | "woff" | "woff2" => AssetKind::Font,
        "zip" | "c3p" | "7z" | "tar" | "gz" => AssetKind::Archive,
        "json" | "txt" | "md" | "csv" | "xml" | "ini" | "cfg" | "yaml" | "yml" | "lua" | "js" | "html" | "css" => AssetKind::Data,
        _ => AssetKind::Other,
    }
}

/// Décode une image (PNG/JPEG/BMP/GIF/WebP) en texture RGBA.
pub fn decode_image(bytes: &[u8]) -> Result<Tex, String> {
    let img = image::load_from_memory(bytes).map_err(|e| format!("image illisible : {e}"))?;
    let img = img.to_rgba8();
    let (w, h) = img.dimensions();
    Ok(Tex { w, h, rgba: img.into_raw() })
}

/// Nom de fichier -> clé d'image normalisée (minuscule, tirets).
pub fn image_key(stem: &str) -> String {
    let n = c3p::normalize(stem);
    if n.is_empty() { "image-importee".to_string() } else { n }
}

/// Dialogue de sélection de fichiers natif.
/// Essaie zenity, kdialog, PowerShell (Windows). Renvoie vide si indisponible :
/// l'éditeur affiche alors une saisie de chemin de secours.
pub fn native_pick_files() -> Vec<std::path::PathBuf> {
    if let Ok(out) = std::process::Command::new("zenity")
        .args(["--file-selection", "--multiple", "--separator=:", "--title", "Importer des fichiers"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            return s
                .split(':')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .map(std::path::PathBuf::from)
                .collect();
        }
    }
    if let Ok(out) = std::process::Command::new("kdialog")
        .args(["--getopenfilename", ".", "*/*", "--multiple", "--separate-output"])
        .output()
    {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).to_string();
            return s
                .lines()
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .map(std::path::PathBuf::from)
                .collect();
        }
    }
    #[cfg(target_os = "windows")]
    {
        let script = "Add-Type -AssemblyName System.Windows.Forms; \
            $f = New-Object System.Windows.Forms.OpenFileDialog; \
            $f.Title = 'Importer des fichiers'; $f.Multiselect = $true; \
            $f.Filter = 'Tous les fichiers (*.*)|*.*'; \
            if ($f.ShowDialog() -eq 'OK') { $f.FileNames -join '|' }";
        if let Ok(out) = std::process::Command::new("powershell")
            .args(["-sta", "-NoProfile", "-Command", script])
            .output()
        {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout).to_string();
                return s
                    .split('|')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .map(std::path::PathBuf::from)
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Lisible en texte court pour la bibliothèque.
pub fn summary(bytes: &[u8]) -> String {
    let head: Vec<u8> = bytes.iter().take(4).copied().collect();
    match head.as_slice() {
        [0x89, b'P', b'N', b'G'] => "PNG".into(),
        [0xFF, 0xD8, ..] => "JPEG".into(),
        [b'P', b'K', ..] => "ZIP".into(),
        [b'R', b'I', b'F', b'F'] => "RIFF/WAV".into(),
        [b'O', b'g', b'g', b'S'] => "OGG".into(),
        [0x00, 0x01, 0x00, 0x00] => "TTF".into(),
        [b'O', b'T', b'T', b'O'] => "OTF".into(),
        _ => {
            if bytes.iter().take(512).all(|b| (*b as char).is_ascii_graphic() || (*b as char).is_whitespace()) {
                "texte"
            } else {
                "binaire"
            }
            .into()
        }
    }
}
