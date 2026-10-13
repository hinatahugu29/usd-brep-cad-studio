use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use pyo3::Bound;
use std::ffi::CString;
use std::sync::Once;

static INIT_PYTHON: Once = Once::new();

pub fn ensure_python_init() {
    INIT_PYTHON.call_once(|| {
        if std::env::var_os("PYTHONHOME").is_none() {
            unsafe {
                std::env::set_var(
                    "PYTHONHOME",
                    "C:\\Users\\T03000\\AppData\\Local\\Programs\\Python\\Python312",
                );
            }
        }
    });
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Box,
    Cylinder,
    Sphere,
    Cone,
    Torus,
}

impl PrimitiveKind {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Cylinder => "Cylinder",
            Self::Sphere => "Sphere",
            Self::Cone => "Cone",
            Self::Torus => "Torus",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeTreatment {
    None,
    Fillet,
    Chamfer,
}

impl EdgeTreatment {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Fillet => "Fillet (round)",
            Self::Chamfer => "Chamfer (bevel)",
        }
    }
}

/// A standalone Fillet/Chamfer modifier feature. A sequence of these is applied, in
/// order, to the solid produced by the boolean history — each one sees the edges of
/// the shape as it stood after the *previous* feature, exactly like a real parametric
/// CAD feature tree. `edge_indices` empty means "all edges of the shape at that point".
#[derive(Debug, Clone)]
pub struct EdgeFeature {
    pub id: u64,
    pub name: String,
    pub treatment: EdgeTreatment,
    pub amount: f64,
    pub edge_indices: Vec<usize>,
    pub suppressed: bool,
}

impl EdgeFeature {
    pub fn new(id: u64, treatment: EdgeTreatment, amount: f64, edge_indices: Vec<usize>) -> Self {
        let scope = if edge_indices.is_empty() {
            "All Edges".to_string()
        } else {
            format!("{} edge(s)", edge_indices.len())
        };
        Self {
            id,
            name: format!("{} — {scope}", treatment.display_name()),
            treatment,
            amount,
            edge_indices,
            suppressed: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    Base,       // First / standalone shape
    Union,      // Union (+)
    Cut,        // Difference (-)
    Intersect,  // Intersection (&)
}

impl BoolOp {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Base => "Base",
            Self::Union => "Union (+)",
            Self::Cut => "Cut (-)",
            Self::Intersect => "Intersect (&)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ShapeDef {
    pub id: u64,
    pub name: String,
    pub kind: PrimitiveKind,
    pub params: [f64; 3],
    pub position: [f64; 3],
    pub rotation: [f64; 3], // degrees: [rx, ry, rz]
    pub op: BoolOp,
    pub visible: bool,
    pub color: [f32; 3],
}

impl ShapeDef {
    pub fn new_box(id: u64, name: &str, w: f64, l: f64, h: f64) -> Self {
        Self {
            id,
            name: name.to_string(),
            kind: PrimitiveKind::Box,
            params: [w, l, h],
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            op: BoolOp::Base,
            visible: true,
            color: [0.52, 0.65, 0.82],
        }
    }

    pub fn new_cylinder(id: u64, name: &str, r: f64, h: f64) -> Self {
        Self {
            id,
            name: name.to_string(),
            kind: PrimitiveKind::Cylinder,
            params: [r, h, 0.0],
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            op: BoolOp::Cut,
            visible: true,
            color: [0.52, 0.65, 0.82],
        }
    }

    pub fn new_sphere(id: u64, name: &str, r: f64) -> Self {
        Self {
            id,
            name: name.to_string(),
            kind: PrimitiveKind::Sphere,
            params: [r, 0.0, 0.0],
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            op: BoolOp::Union,
            visible: true,
            color: [0.52, 0.65, 0.82],
        }
    }

    pub fn new_cone(id: u64, name: &str, r_bottom: f64, r_top: f64, h: f64) -> Self {
        Self {
            id,
            name: name.to_string(),
            kind: PrimitiveKind::Cone,
            params: [r_bottom, r_top, h],
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            op: BoolOp::Union,
            visible: true,
            color: [0.52, 0.65, 0.82],
        }
    }

    pub fn new_torus(id: u64, name: &str, r_major: f64, r_minor: f64) -> Self {
        Self {
            id,
            name: name.to_string(),
            kind: PrimitiveKind::Torus,
            params: [r_major, r_minor, 0.0],
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            op: BoolOp::Union,
            visible: true,
            color: [0.52, 0.65, 0.82],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EvaluationResult {
    pub vertex_buffer: Vec<u8>, // flattened f32: [x, y, z, nx, ny, nz]
    pub line_buffer: Vec<u8>,   // flattened f32: [x, y, z] pairs
    pub triangle_count: usize,
    pub line_count: usize,
    pub volume: f64,
    pub surface_area: f64,
    pub bbox_min: [f32; 3],
    pub bbox_max: [f32; 3],
    /// Flattened f32 [start.xyz, end.xyz] per topological edge of the
    /// pre-edge-treatment composite solid. Used for viewport edge picking.
    pub edge_points: Vec<u8>,
    pub edge_count: usize,
    pub error: Option<String>,
    pub warning: Option<String>,
}

const EVALUATOR_PYTHON_SCRIPT: &str = r#"
import numpy as np
import smlibpy as sm

def build_shape(kind, params, pos, rot):
    p0, p1, p2 = params
    if kind == 'Box':
        s = sm.Box(max(0.001, p0), max(0.001, p1), max(0.001, p2))
    elif kind == 'Cylinder':
        s = sm.Cylinder(max(0.001, p0), max(0.001, p1))
    elif kind == 'Sphere':
        s = sm.Sphere(max(0.001, p0))
    elif kind == 'Cone':
        s = sm.Cone(max(0.0, p0), max(0.0, p1), max(0.001, p2))
    elif kind == 'Torus':
        r_maj = max(0.001, p0)
        r_min = max(0.001, min(p1, r_maj * 0.95))
        s = sm.Torus(r_maj, r_min)
    else:
        raise ValueError(f"Unknown kind {kind}")

    rx, ry, rz = rot
    if abs(rx) > 1e-4:
        s = s.rotate(sm.Axis.X, rx)
    if abs(ry) > 1e-4:
        s = s.rotate(sm.Axis.Y, ry)
    if abs(rz) > 1e-4:
        s = s.rotate(sm.Axis.Z, rz)

    tx, ty, tz = pos
    if abs(tx) > 1e-4 or abs(ty) > 1e-4 or abs(tz) > 1e-4:
        s = s.translate(tx, ty, tz)

    return s

def select_edges(edges_list, indices):
    """Resolves stored edge indices against the CURRENT edge list. Indices are only
    valid for the shape topology they were captured against — if the solid changed
    since (new/edited primitives upstream), some or all of them may no longer refer
    to a real edge. Never silently fall back to "all edges": that would apply the
    feature far more broadly than the user ever selected, with no indication anything
    was wrong."""
    if not indices:
        return edges_list, None
    n = len(edges_list)
    valid = [i for i in indices if 0 <= i < n]
    if not valid:
        raise ValueError(
            f"all {len(indices)} selected edge(s) no longer exist on this shape "
            f"(it now has {n} edge(s) — geometry upstream likely changed)"
        )
    sel = [edges_list[i] for i in valid]
    dropped = len(indices) - len(valid)
    note = f"{dropped} of {len(indices)} selected edge(s) no longer exist and were skipped" if dropped else None
    return sm.ShapeList(sel), note


def apply_edge_treatment(composite, treatment, amount, edges_list, indices):
    if treatment == 'None' or amount <= 1e-6:
        return composite, None
    try:
        target, note = select_edges(edges_list, indices)
        if treatment == 'Fillet':
            result = sm.fillet(target, amount)
        elif treatment == 'Chamfer':
            result = sm.chamfer(target, amount)
        else:
            return composite, None
        return result, note
    except Exception as e:
        return composite, f"{treatment} skipped: {e}"


def edge_points_bytes(edges_list):
    vals = []
    for e in edges_list:
        s, en = e.start, e.end
        vals.extend([s.x, s.y, s.z, en.x, en.y, en.z])
    buf = np.array(vals, dtype=np.float32).tobytes() if vals else b''
    return buf, len(edges_list)


def apply_edge_features(composite, features):
    """Applies a sequence of {treatment, amount, indices} dicts in order, each one
    seeing the edges of the shape as it stood after the previous feature. Returns
    (composite, warnings, edges_before_first_uncommitted_feature_bytes/count) where the
    last pair is the pickable edge list for composing the *next* feature (i.e. the
    state after all COMMITTED features — see `committed_count` below)."""
    warnings = []
    for feat in features:
        composite, warn = apply_edge_treatment(
            composite, feat.get('treatment', 'None'), feat.get('amount', 0.0),
            composite.edges(), feat.get('indices') or [],
        )
        if warn:
            warnings.append(warn)
    return composite, warnings


def evaluate_scene(shapes_data, committed_features=None, draft_feature=None):
    if not shapes_data:
        return None

    composite = None
    for item in shapes_data:
        if not item['visible']:
            continue
        curr = build_shape(item['kind'], item['params'], item['position'], item['rotation'])
        op = item['op']

        if composite is None:
            composite = curr
        elif op == 'Union':
            composite = composite + curr
        elif op == 'Cut':
            composite = composite - curr
        elif op == 'Intersect':
            composite = composite & curr
        else:
            composite = composite + curr

    if composite is None:
        return None

    composite, warnings = apply_edge_features(composite, committed_features or [])

    # Pickable edge reference for composing the NEXT feature: the shape as it stands
    # after all committed features, before any in-progress (uncommitted) draft.
    edge_points_buf, edge_count = edge_points_bytes(composite.edges())

    if draft_feature:
        composite, draft_warnings = apply_edge_features(composite, [draft_feature])
        warnings.extend(draft_warnings)

    warning = '; '.join(warnings) if warnings else None

    m = composite.mesh()
    verts = m.vertices
    tris = m.triangles
    normals = m.normals
    tri_normals = m.triangle_normals

    num_tris = len(tris)
    if num_tris == 0:
        return None

    tri_verts = verts[tris]
    tri_norms = normals
    flat_buf = np.concatenate([tri_verts, tri_norms], axis=-1).astype(np.float32).tobytes()

    edge_to_faces = {}
    for i, tri in enumerate(tris):
        for j in range(3):
            u, v = int(tri[j]), int(tri[(j + 1) % 3])
            edge = (min(u, v), max(u, v))
            edge_to_faces.setdefault(edge, []).append(i)

    crease_lines = []
    cos_threshold = np.cos(np.radians(25.0))
    for (u, v), faces in edge_to_faces.items():
        if len(faces) == 1:
            crease_lines.extend([verts[u], verts[v]])
        elif len(faces) == 2:
            n1, n2 = tri_normals[faces[0]], tri_normals[faces[1]]
            if np.dot(n1, n2) < cos_threshold:
                crease_lines.extend([verts[u], verts[v]])

    crease_buf = np.array(crease_lines, dtype=np.float32).tobytes() if crease_lines else b''

    bbox = m.bbox()
    b_min = [float(bbox.min[0]), float(bbox.min[1]), float(bbox.min[2])]
    b_max = [float(bbox.max[0]), float(bbox.max[1]), float(bbox.max[2])]

    return {
        'vertex_bytes': flat_buf,
        'line_bytes': crease_buf,
        'triangle_count': num_tris,
        'line_count': len(crease_lines) // 2,
        'volume': float(composite.volume),
        'surface_area': float(composite.area),
        'bbox_min': b_min,
        'bbox_max': b_max,
        'edge_points': edge_points_buf,
        'edge_count': edge_count,
        'warning': warning,
    }

def export_model(shapes_data, fmt, out_path, committed_features=None):
    if not shapes_data:
        raise ValueError("No shapes in scene to export")

    composite = None
    for item in shapes_data:
        if not item['visible']:
            continue
        curr = build_shape(item['kind'], item['params'], item['position'], item['rotation'])
        op = item['op']

        if composite is None:
            composite = curr
        elif op == 'Union':
            composite = composite + curr
        elif op == 'Cut':
            composite = composite - curr
        elif op == 'Intersect':
            composite = composite & curr
        else:
            composite = composite + curr

    if composite is None:
        raise ValueError("Active shapes produced an empty model")

    composite, _warnings = apply_edge_features(composite, committed_features or [])

    fmt_lower = fmt.lower()
    if fmt_lower == 'step':
        sm.export_step(composite, out_path)
    elif fmt_lower == 'glb':
        sm.export_glb(composite, out_path)
    elif fmt_lower == 'stl':
        sm.export_stl(composite, out_path)
    elif fmt_lower == 'obj':
        sm.export_obj(composite, out_path)
    else:
        raise ValueError(f"Unsupported format: {fmt}")
"#;

fn edge_feature_dict<'py>(py: Python<'py>, f: &EdgeFeature) -> Bound<'py, PyDict> {
    let dict = PyDict::new(py);
    let treatment_str = match f.treatment {
        EdgeTreatment::None => "None",
        EdgeTreatment::Fillet => "Fillet",
        EdgeTreatment::Chamfer => "Chamfer",
    };
    let idx_list = PyList::empty(py);
    for &i in &f.edge_indices {
        let _ = idx_list.append(i as i64);
    }
    let _ = dict.set_item("treatment", treatment_str);
    let _ = dict.set_item("amount", f.amount);
    let _ = dict.set_item("indices", idx_list);
    dict
}

pub fn evaluate_scene(
    shapes: &[ShapeDef],
    committed_features: &[EdgeFeature],
    draft_feature: Option<&EdgeFeature>,
) -> EvaluationResult {
    ensure_python_init();

    Python::attach(|py| {
        let sys = match py.import("sys") {
            Ok(s) => s,
            Err(e) => return EvaluationResult {
                error: Some(format!("Failed to import sys: {e}")),
                ..Default::default()
            },
        };
        if let Ok(path) = sys.getattr("path") {
            if let Ok(path_list) = path.cast_into::<PyList>() {
                let _ = path_list.append("C:\\Users\\T03000\\AppData\\Local\\Programs\\Python\\Python312\\Lib\\site-packages");
            }
        }

        let code_cstr = CString::new(EVALUATOR_PYTHON_SCRIPT).unwrap();
        let file_cstr = CString::new("evaluator.py").unwrap();
        let name_cstr = CString::new("evaluator").unwrap();

        let module = match PyModule::from_code(py, &code_cstr, &file_cstr, &name_cstr) {
            Ok(m) => m,
            Err(e) => return EvaluationResult {
                error: Some(format!("Failed to compile Python evaluator: {e}")),
                ..Default::default()
            },
        };

        let py_shapes = PyList::empty(py);

        for s in shapes {
            let dict = PyDict::new(py);
            let kind_str = match s.kind {
                PrimitiveKind::Box => "Box",
                PrimitiveKind::Cylinder => "Cylinder",
                PrimitiveKind::Sphere => "Sphere",
                PrimitiveKind::Cone => "Cone",
                PrimitiveKind::Torus => "Torus",
            };
            let op_str = match s.op {
                BoolOp::Base => "Base",
                BoolOp::Union => "Union",
                BoolOp::Cut => "Cut",
                BoolOp::Intersect => "Intersect",
            };
            let _ = dict.set_item("kind", kind_str);
            let _ = dict.set_item("params", s.params.to_vec());
            let _ = dict.set_item("position", s.position.to_vec());
            let _ = dict.set_item("rotation", s.rotation.to_vec());
            let _ = dict.set_item("op", op_str);
            let _ = dict.set_item("visible", s.visible);
            let _ = py_shapes.append(dict);
        }

        let eval_fn = match module.getattr("evaluate_scene") {
            Ok(f) => f,
            Err(e) => return EvaluationResult {
                error: Some(format!("Could not get evaluate_scene: {e}")),
                ..Default::default()
            },
        };

        let committed_list = PyList::empty(py);
        for f in committed_features.iter().filter(|f| !f.suppressed) {
            let _ = committed_list.append(edge_feature_dict(py, f));
        }
        let draft_dict = draft_feature.map(|f| edge_feature_dict(py, f));

        let res_obj = match eval_fn.call1((py_shapes, committed_list, draft_dict)) {
            Ok(r) => r,
            Err(e) => return EvaluationResult {
                error: Some(format!("B-rep evaluation failed: {e}")),
                ..Default::default()
            },
        };

        if res_obj.is_none() {
            return EvaluationResult::default();
        }

        let dict = match res_obj.cast_into::<PyDict>() {
            Ok(d) => d,
            Err(e) => return EvaluationResult {
                error: Some(format!("Invalid result type: {e}")),
                ..Default::default()
            },
        };

        let get_bytes = |k: &str| -> Vec<u8> {
            dict.get_item(k).ok().flatten()
                .and_then(|v| v.extract::<Vec<u8>>().ok())
                .unwrap_or_default()
        };
        let get_usize = |k: &str| -> usize {
            dict.get_item(k).ok().flatten()
                .and_then(|v| v.extract::<usize>().ok())
                .unwrap_or(0)
        };
        let get_f64 = |k: &str| -> f64 {
            dict.get_item(k).ok().flatten()
                .and_then(|v| v.extract::<f64>().ok())
                .unwrap_or(0.0)
        };
        let get_f32_3 = |k: &str| -> [f32; 3] {
            dict.get_item(k).ok().flatten()
                .and_then(|v| v.extract::<Vec<f32>>().ok())
                .and_then(|vec| if vec.len() == 3 { Some([vec[0], vec[1], vec[2]]) } else { None })
                .unwrap_or([0.0, 0.0, 0.0])
        };
        let get_opt_string = |k: &str| -> Option<String> {
            dict.get_item(k).ok().flatten()
                .and_then(|v| v.extract::<String>().ok())
        };

        EvaluationResult {
            vertex_buffer: get_bytes("vertex_bytes"),
            line_buffer: get_bytes("line_bytes"),
            triangle_count: get_usize("triangle_count"),
            line_count: get_usize("line_count"),
            volume: get_f64("volume"),
            surface_area: get_f64("surface_area"),
            bbox_min: get_f32_3("bbox_min"),
            bbox_max: get_f32_3("bbox_max"),
            edge_points: get_bytes("edge_points"),
            edge_count: get_usize("edge_count"),
            error: None,
            warning: get_opt_string("warning"),
        }
    })
}

pub fn export_scene(
    shapes: &[ShapeDef],
    format: &str,
    path: &str,
    committed_features: &[EdgeFeature],
) -> Result<(), String> {
    ensure_python_init();

    Python::attach(|py| {
        let sys = py.import("sys").map_err(|e| e.to_string())?;
        if let Ok(p) = sys.getattr("path") {
            if let Ok(pl) = p.cast_into::<PyList>() {
                let _ = pl.append("C:\\Users\\T03000\\AppData\\Local\\Programs\\Python\\Python312\\Lib\\site-packages");
            }
        }

        let code_cstr = CString::new(EVALUATOR_PYTHON_SCRIPT).unwrap();
        let file_cstr = CString::new("evaluator.py").unwrap();
        let name_cstr = CString::new("evaluator").unwrap();

        let module = PyModule::from_code(py, &code_cstr, &file_cstr, &name_cstr)
            .map_err(|e| format!("Script error: {e}"))?;

        let py_shapes = PyList::empty(py);
        for s in shapes {
            let dict = PyDict::new(py);
            let kind_str = match s.kind {
                PrimitiveKind::Box => "Box",
                PrimitiveKind::Cylinder => "Cylinder",
                PrimitiveKind::Sphere => "Sphere",
                PrimitiveKind::Cone => "Cone",
                PrimitiveKind::Torus => "Torus",
            };
            let op_str = match s.op {
                BoolOp::Base => "Base",
                BoolOp::Union => "Union",
                BoolOp::Cut => "Cut",
                BoolOp::Intersect => "Intersect",
            };
            dict.set_item("kind", kind_str).map_err(|e| e.to_string())?;
            dict.set_item("params", s.params.to_vec()).map_err(|e| e.to_string())?;
            dict.set_item("position", s.position.to_vec()).map_err(|e| e.to_string())?;
            dict.set_item("rotation", s.rotation.to_vec()).map_err(|e| e.to_string())?;
            dict.set_item("op", op_str).map_err(|e| e.to_string())?;
            dict.set_item("visible", s.visible).map_err(|e| e.to_string())?;
            py_shapes.append(dict).map_err(|e| e.to_string())?;
        }

        let committed_list = PyList::empty(py);
        for f in committed_features.iter().filter(|f| !f.suppressed) {
            let _ = committed_list.append(edge_feature_dict(py, f));
        }

        let export_fn = module.getattr("export_model").map_err(|e| e.to_string())?;
        export_fn
            .call1((py_shapes, format, path, committed_list))
            .map_err(|e| e.to_string())?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_box_and_cylinder() {
        let shapes = vec![
            ShapeDef::new_box(1, "Box", 20.0, 20.0, 10.0),
            ShapeDef::new_cylinder(2, "Cyl", 5.0, 15.0),
        ];
        let res = evaluate_scene(&shapes, &[], None);
        assert!(res.error.is_none(), "Evaluation error: {:?}", res.error);
        assert!(res.triangle_count > 0, "Expected triangles");
        assert!(res.volume > 0.0, "Expected positive volume");
        assert!(!res.vertex_buffer.is_empty(), "Expected vertex buffer");
        assert!(res.edge_count > 0, "Expected a non-empty edge list");
        assert_eq!(
            res.edge_points.len(),
            res.edge_count * 6 * 4,
            "edge_points should hold 2 points * 3 floats per edge"
        );
    }

    #[test]
    fn test_eval_with_fillet_all_edges() {
        let shapes = vec![ShapeDef::new_box(1, "Box", 20.0, 20.0, 10.0)];
        let feat = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![]);
        let res = evaluate_scene(&shapes, &[feat], None);
        assert!(res.error.is_none(), "Evaluation error: {:?}", res.error);
        assert!(res.triangle_count > 0, "Expected triangles after fillet");
    }

    #[test]
    fn test_eval_with_fillet_selected_edges() {
        let shapes = vec![ShapeDef::new_box(1, "Box", 20.0, 20.0, 10.0)];
        // Fillet only the first two of the 12 box edges.
        let partial = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![0, 1]);
        let res = evaluate_scene(&shapes, &[partial], None);
        assert!(res.error.is_none(), "Evaluation error: {:?}", res.error);
        assert!(res.triangle_count > 0, "Expected triangles after selective fillet");

        let all = EdgeFeature::new(2, EdgeTreatment::Fillet, 2.0, vec![]);
        let res_all = evaluate_scene(&shapes, &[all], None);
        assert!(
            res.triangle_count < res_all.triangle_count,
            "Selective fillet should produce fewer triangles than filleting all edges"
        );
    }

    #[test]
    fn test_sequential_fillet_features_with_different_radii() {
        let shapes = vec![ShapeDef::new_box(1, "Box", 30.0, 30.0, 15.0)];
        // Two sequential fillet features with different radii targeting different edges.
        let feat_a = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![0, 1]);
        let feat_b = EdgeFeature::new(2, EdgeTreatment::Fillet, 1.0, vec![2]);
        let res = evaluate_scene(&shapes, &[feat_a, feat_b], None);
        assert!(res.error.is_none(), "Evaluation error: {:?}", res.error);
        assert!(res.triangle_count > 0, "Expected triangles after two sequential fillets");
    }

    #[test]
    fn test_suppressed_feature_is_skipped() {
        let shapes = vec![ShapeDef::new_box(1, "Box", 20.0, 20.0, 10.0)];
        let mut suppressed = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![]);
        suppressed.suppressed = true;
        let res_suppressed = evaluate_scene(&shapes, &[suppressed], None);

        let res_none = evaluate_scene(&shapes, &[], None);
        assert_eq!(
            res_suppressed.triangle_count, res_none.triangle_count,
            "A suppressed feature should have no geometric effect"
        );
    }

    #[test]
    fn test_draft_feature_previews_without_affecting_edge_points() {
        let shapes = vec![ShapeDef::new_box(1, "Box", 20.0, 20.0, 10.0)];
        let committed = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![0]);
        let draft = EdgeFeature::new(2, EdgeTreatment::Fillet, 1.5, vec![1]);

        let res_committed_only = evaluate_scene(&shapes, &[committed.clone()], None);
        let res_with_draft = evaluate_scene(&shapes, &[committed], Some(&draft));

        // The draft changes the final mesh...
        assert_ne!(res_committed_only.triangle_count, res_with_draft.triangle_count);
        // ...but the pickable edge list (for composing the NEXT feature) reflects only
        // the committed state, not the in-progress draft.
        assert_eq!(res_committed_only.edge_count, res_with_draft.edge_count);
    }

    #[test]
    fn test_stale_edge_indices_skip_instead_of_filleting_everything() {
        // A tiny box has far fewer edges than a huge bogus index list could ever
        // address — simulates a feature whose stored indices no longer exist because
        // the upstream geometry changed since it was created.
        let shapes = vec![ShapeDef::new_box(1, "Box", 10.0, 10.0, 10.0)];
        let stale = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![999, 1000, 1001]);

        let res_stale = evaluate_scene(&shapes, &[stale], None);
        let res_untouched = evaluate_scene(&shapes, &[], None);

        assert!(res_stale.error.is_none(), "Evaluation error: {:?}", res_stale.error);
        assert!(
            res_stale.warning.is_some(),
            "A fully-stale edge selection must surface a warning instead of silently proceeding"
        );
        assert_eq!(
            res_stale.triangle_count, res_untouched.triangle_count,
            "A fully-stale edge selection must be skipped entirely, not silently applied to ALL edges"
        );
    }

    #[test]
    fn test_partially_stale_edge_indices_use_only_the_valid_ones() {
        let shapes = vec![ShapeDef::new_box(1, "Box", 20.0, 20.0, 10.0)];
        // Index 0 is a real edge; the others are bogus.
        let partial = EdgeFeature::new(1, EdgeTreatment::Fillet, 2.0, vec![0, 999, 1000]);
        let single = EdgeFeature::new(2, EdgeTreatment::Fillet, 2.0, vec![0]);

        let res_partial = evaluate_scene(&shapes, &[partial], None);
        let res_single = evaluate_scene(&shapes, &[single], None);

        assert!(res_partial.error.is_none(), "Evaluation error: {:?}", res_partial.error);
        assert!(res_partial.warning.is_some(), "Dropping some stale indices should still be reported");
        assert_eq!(
            res_partial.triangle_count, res_single.triangle_count,
            "Should fillet only the still-valid edge(s), matching a feature that targeted just that edge"
        );
    }
}
