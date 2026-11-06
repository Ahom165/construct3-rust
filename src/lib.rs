//! c3rs — runtime + éditeur natif pour jeux Construct 3 (.c3p) en Rust.
//!
//! Modules :
//! - [`expr`] : évaluateur d'expressions compatible C3
//! - [`model`] : modèle de données + parsing JSON
//! - [`c3p`] : chargement des archives .c3p
//! - [`runtime`] : moteur (comportements, événements)
//! - [`render`] : rendu logiciel
//! - [`font`] : police bitmap
//! - [`save`] : écriture de projets .c3p
//! - [`assets`] : import universel de médias
//! - [`editor`] : éditeur natif (egui/eframe, GPU)

pub mod assets;
pub mod c3p;
pub mod editview;
pub mod editor;
pub mod expr;
pub mod font;
pub mod model;
pub mod render;
pub mod runtime;
pub mod save;
