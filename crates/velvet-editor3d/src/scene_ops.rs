//! Edits on a `Scene3D` tree addressed by node path (`parent/child/...`, the same ids the
//! flattener gives each node): find, add, remove, duplicate, rename, move to the root.

use velvet_scene3d::{
    CameraDoc, EnvironmentDoc, LightDoc, MaterialDoc, MeshKind3D, Scene3D, SceneNode3D,
    TransformDoc,
};

pub fn node<'a>(nodes: &'a [SceneNode3D], path: &str) -> Option<&'a SceneNode3D> {
    let mut list = nodes;
    let mut found = None;
    for part in path.split('/') {
        let n = list.iter().find(|n| n.id == part)?;
        found = Some(n);
        list = &n.children;
    }
    found
}

pub fn node_mut<'a>(nodes: &'a mut [SceneNode3D], path: &str) -> Option<&'a mut SceneNode3D> {
    let mut parts = path.split('/');
    let first = parts.next()?;
    let mut cur = nodes.iter_mut().find(|n| n.id == first)?;
    for part in parts {
        cur = cur.children.iter_mut().find(|n| n.id == part)?;
    }
    Some(cur)
}

fn children_mut<'a>(
    scene: &'a mut Scene3D,
    parent: Option<&str>,
) -> Option<&'a mut Vec<SceneNode3D>> {
    match parent {
        None => Some(&mut scene.nodes),
        Some(p) => node_mut(&mut scene.nodes, p).map(|n| &mut n.children),
    }
}

pub fn parent_of(path: &str) -> Option<&str> {
    path.rfind('/').map(|i| &path[..i])
}

pub fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn join(parent: Option<&str>, id: &str) -> String {
    match parent {
        Some(p) => format!("{p}/{id}"),
        None => id.to_string(),
    }
}

fn clean_id(s: &str) -> String {
    let t: String = s
        .trim()
        .chars()
        .map(|c| if c == '/' { '_' } else { c })
        .collect();
    if t.is_empty() {
        "nodo".into()
    } else {
        t
    }
}

fn stem(s: &str) -> &str {
    if let Some(i) = s.rfind('_') {
        let tail = &s[i + 1..];
        if i > 0 && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            return &s[..i];
        }
    }
    s
}

/// `base`, or `base_2`, `base_3`… whichever no sibling uses yet.
pub fn unique_id(siblings: &[SceneNode3D], base: &str, skip: Option<usize>) -> String {
    let base = clean_id(base);
    let taken = |id: &str| {
        siblings
            .iter()
            .enumerate()
            .any(|(i, n)| Some(i) != skip && n.id == id)
    };
    if !taken(&base) {
        return base;
    }
    let s = stem(&base).to_string();
    (2..)
        .map(|i| format!("{s}_{i}"))
        .find(|c| !taken(c))
        .unwrap_or(base)
}

pub fn new_node(id: &str, mesh: MeshKind3D) -> SceneNode3D {
    SceneNode3D {
        id: id.into(),
        mesh,
        transform: TransformDoc::default(),
        material: MaterialDoc::default(),
        cast_shadow: true,
        collider: true,
        disabled: false,
        children: Vec::new(),
    }
}

/// Adds `node` under `parent` (the root when `None`) with an id free among its siblings; its path.
pub fn add_node(
    scene: &mut Scene3D,
    parent: Option<&str>,
    mut node: SceneNode3D,
) -> Option<String> {
    let list = children_mut(scene, parent)?;
    node.id = unique_id(list, &node.id, None);
    let id = node.id.clone();
    list.push(node);
    Some(join(parent, &id))
}

pub fn remove_node(scene: &mut Scene3D, path: &str) -> Option<SceneNode3D> {
    let parent = parent_of(path).map(str::to_string);
    let list = children_mut(scene, parent.as_deref())?;
    let i = list.iter().position(|n| n.id == leaf(path))?;
    Some(list.remove(i))
}

/// A copy next to the original, a metre to the side; the copy's path.
pub fn duplicate(scene: &mut Scene3D, path: &str) -> Option<String> {
    let mut copy = node(&scene.nodes, path)?.clone();
    let parent = parent_of(path).map(str::to_string);
    let list = children_mut(scene, parent.as_deref())?;
    copy.id = unique_id(list, &copy.id, None);
    copy.transform.position[0] += 1.0;
    let at = list
        .iter()
        .position(|n| n.id == leaf(path))
        .map_or(list.len(), |i| i + 1);
    let id = copy.id.clone();
    list.insert(at, copy);
    Some(join(parent.as_deref(), &id))
}

/// Renames the node (kept unique among its siblings); its new path.
pub fn rename(scene: &mut Scene3D, path: &str, wanted: &str) -> Option<String> {
    let parent = parent_of(path).map(str::to_string);
    let list = children_mut(scene, parent.as_deref())?;
    let i = list.iter().position(|n| n.id == leaf(path))?;
    let id = unique_id(list, wanted, Some(i));
    list[i].id = id.clone();
    Some(join(parent.as_deref(), &id))
}

/// Moves a nested node to the root (keeping its local transform); its new path.
pub fn move_to_root(scene: &mut Scene3D, path: &str) -> Option<String> {
    parent_of(path)?;
    let mut n = remove_node(scene, path)?;
    n.id = unique_id(&scene.nodes, &n.id, None);
    let id = n.id.clone();
    scene.nodes.push(n);
    Some(id)
}

/// Hangs the node under `new_parent` (the root when `None`), keeping its local transform; its
/// new path. Refuses moves into itself or its own children.
pub fn reparent(scene: &mut Scene3D, path: &str, new_parent: Option<&str>) -> Option<String> {
    match new_parent {
        Some(np) => {
            if np == path || np.starts_with(&format!("{path}/")) || parent_of(path) == Some(np) {
                return None;
            }
            node(&scene.nodes, np)?;
        }
        None => {
            parent_of(path)?;
        }
    }
    let n = remove_node(scene, path)?;
    add_node(scene, new_parent, n)
}

pub fn count(nodes: &[SceneNode3D]) -> usize {
    nodes.iter().map(|n| 1 + count(&n.children)).sum()
}

/// Every node path, depth first.
pub fn all_paths(nodes: &[SceneNode3D], parent: Option<&str>, out: &mut Vec<String>) {
    for n in nodes {
        let p = join(parent, &n.id);
        out.push(p.clone());
        all_paths(&n.children, Some(&p), out);
    }
}

/// A new scene: a floor, a cube and a key light.
pub fn default_scene() -> Scene3D {
    let mut floor = new_node("suelo", MeshKind3D::Plane);
    floor.transform.scale = [20.0, 1.0, 20.0];
    floor.material.base_color = [0.32, 0.30, 0.36, 1.0];
    floor.collider = true;
    let mut cube = new_node("cubo", MeshKind3D::Cube);
    cube.transform.position = [0.0, 0.5, 0.0];
    cube.material.base_color = [0.62, 0.42, 0.95, 1.0];
    Scene3D {
        name: "nueva_escena".into(),
        nodes: vec![floor, cube],
        camera: CameraDoc::default(),
        lights: vec![LightDoc::Directional {
            direction: [-0.48, 0.82, 0.31],
            key_color: [1.05, 0.82, 0.62],
            ambient_color: [0.16, 0.12, 0.28],
            shadow_extent: 14.0,
        }],
        environment: EnvironmentDoc::default(),
    }
}

/// What kind of node this is, in a word, with its icon.
pub fn kind_label(mesh: &MeshKind3D) -> (&'static str, &'static str) {
    match mesh {
        MeshKind3D::Empty => ("◇", "Vacío"),
        MeshKind3D::Cube => ("■", "Cubo"),
        MeshKind3D::Sphere { .. } => ("●", "Esfera"),
        MeshKind3D::Plane => ("▬", "Plano"),
        MeshKind3D::Heightfield { .. } => ("⛰", "Terreno"),
        MeshKind3D::Gltf { .. } => ("◈", "Modelo"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_duplicate_rename_and_remove_keep_ids_unique() {
        let mut scene = default_scene();
        let a = add_node(&mut scene, None, new_node("cubo", MeshKind3D::Cube)).unwrap();
        assert_eq!(a, "cubo_2");
        let child = add_node(&mut scene, Some("cubo"), new_node("luz", MeshKind3D::Empty)).unwrap();
        assert_eq!(child, "cubo/luz");
        let dup = duplicate(&mut scene, "cubo/luz").unwrap();
        assert_eq!(dup, "cubo/luz_2");
        let renamed = rename(&mut scene, "cubo/luz_2", "luz").unwrap();
        assert_eq!(renamed, "cubo/luz_2", "a sibling already has that name");
        let moved = move_to_root(&mut scene, "cubo/luz_2").unwrap();
        assert_eq!(moved, "luz_2");
        assert!(remove_node(&mut scene, "cubo/luz").is_some());
        assert_eq!(count(&scene.nodes), 4);
        // Into a sibling, never into itself.
        let moved = reparent(&mut scene, "luz_2", Some("cubo")).unwrap();
        assert_eq!(moved, "cubo/luz_2");
        assert!(reparent(&mut scene, "cubo", Some("cubo/luz_2")).is_none());
        assert_eq!(reparent(&mut scene, "cubo/luz_2", None).unwrap(), "luz_2");
    }
}
