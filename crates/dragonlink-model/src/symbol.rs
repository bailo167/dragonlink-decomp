//! Symbols.

use serde::{Deserialize, Serialize};

/// Index of a symbol within one math model's symbol list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SymbolId(pub u16);

/// What role a symbol plays in evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// Pays on lines.
    Regular,
    /// Substitutes for regular symbols on lines.
    Wild,
    /// Pays anywhere in the window; never part of a line.
    Scatter,
    /// The Hold & Spin prize symbol; never part of a line.
    Orb,
}

/// One symbol of a math model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolDef {
    pub id: SymbolId,
    /// Short stable name used in traces and asset ids.
    pub key: String,
    pub kind: SymbolKind,
}
