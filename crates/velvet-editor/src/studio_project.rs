//! Persist Studio layer tree + graph + paths to `velvet.studio.json`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::layers::{LayerEdge, LayerEdgeKind, LayerStack, ScreenLayer};

const STUDIO_FILE: &str = "velvet.studio.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioProjectFile {
    pub version: u32,
    pub active_layer: String,
    pub layers: Vec<StudioLayerSer>,
    pub edges: Vec<StudioEdgeSer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioLayerSer {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub z: i32,
    pub locked: bool,
    pub visible: bool,
    pub expanded: bool,
    pub width_px: u32,
    pub height_px: u32,
    pub document_path: Option<String>,
    pub graph_x: Option<f32>,
    pub graph_y: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioEdgeSer {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub kind: String,
}

impl StudioProjectFile {
    pub fn path(root: &Path) -> PathBuf {
        root.join(STUDIO_FILE)
    }

    pub fn from_stack(stack: &LayerStack) -> Self {
        Self {
            version: 1,
            active_layer: stack.active_id.clone(),
            layers: stack
                .layers
                .iter()
                .map(|l| StudioLayerSer {
                    id: l.id.clone(),
                    name: l.name.clone(),
                    parent: l.parent.clone(),
                    z: l.z,
                    locked: l.locked,
                    visible: l.visible,
                    expanded: l.expanded,
                    width_px: l.width_px,
                    height_px: l.height_px,
                    document_path: l
                        .document_path
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned()),
                    graph_x: l.graph_x,
                    graph_y: l.graph_y,
                })
                .collect(),
            edges: stack
                .edges
                .iter()
                .map(|e| StudioEdgeSer {
                    from: e.from.clone(),
                    to: e.to.clone(),
                    label: e.label.clone(),
                    kind: e.kind.as_str().to_string(),
                })
                .collect(),
        }
    }

    pub fn to_stack(&self) -> LayerStack {
        let mut layers = Vec::new();
        for l in &self.layers {
            let mut sl = if let Some(ref p) = l.parent {
                ScreenLayer::child(&l.id, &l.name, p, l.z, l.width_px, l.height_px)
            } else {
                ScreenLayer::root(&l.id, &l.name, l.z, l.width_px, l.height_px)
            };
            sl.locked = l.locked;
            sl.visible = l.visible;
            sl.expanded = l.expanded;
            sl.graph_x = l.graph_x;
            sl.graph_y = l.graph_y;
            if let Some(ref dp) = l.document_path {
                sl.document_path = Some(PathBuf::from(dp));
            }
            layers.push(sl);
        }
        let edges = self
            .edges
            .iter()
            .map(|e| LayerEdge {
                from: e.from.clone(),
                to: e.to.clone(),
                label: e.label.clone(),
                kind: match e.kind.as_str() {
                    "overlay" => LayerEdgeKind::Overlay,
                    "back" => LayerEdgeKind::Back,
                    _ => LayerEdgeKind::Transition,
                },
            })
            .collect();
        let active = if layers.iter().any(|l| l.id == self.active_layer) {
            self.active_layer.clone()
        } else {
            layers
                .first()
                .map(|l| l.id.clone())
                .unwrap_or_else(|| "main_menu".into())
        };
        LayerStack {
            layers,
            active_id: active,
            resize_anim: None,
            edges,
        }
    }

    pub fn load(root: &Path) -> Result<Option<Self>> {
        let p = Self::path(root);
        if !p.is_file() {
            return Ok(None);
        }
        let text = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
        let file: Self = serde_json::from_str(&text).with_context(|| "parse velvet.studio.json")?;
        Ok(Some(file))
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let p = Self::path(root);
        let text = serde_json::to_string_pretty(self).context("serialize studio project")?;
        fs::write(&p, text).with_context(|| format!("write {}", p.display()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_studio_json() {
        let dir = tempdir().unwrap();
        let stack = LayerStack::vn_tree();
        let file = StudioProjectFile::from_stack(&stack);
        file.save(dir.path()).unwrap();
        let loaded = StudioProjectFile::load(dir.path()).unwrap().unwrap();
        let s2 = loaded.to_stack();
        assert_eq!(s2.layers.len(), stack.layers.len());
        assert_eq!(s2.edges.len(), stack.edges.len());
        assert!(s2.get("menu_settings").is_some());
    }
}
