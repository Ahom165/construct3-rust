//! c3rs — runtime de jeux Construct 3 (.c3p) en Rust.
//!
//! Modules :
//! - [`expr`] : évaluateur d'expressions compatible C3
//! - [`model`] : modèle de données + parsing JSON
//! - [`c3p`] : chargement des archives .c3p
//! - [`runtime`] : moteur (comportements, événements)
//! - [`render`] : rendu logiciel
//! - [`font`] : police bitmap

pub mod c3p;
pub mod expr;
pub mod font;
pub mod model;
pub mod render;
pub mod runtime;
