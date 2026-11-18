//! Reader of SAF, the Structural Analysis Format: an `.xlsx` workbook with a sheet per kind of
//! object, a row per object and the first row naming the columns, each with its unit in
//! brackets (`Coordinate X [m]`). Read into the analysis model of `fem-model`, in SI units.
//!
//! What is read:
//!
//! - `StructuralMaterial`: unit mass, E and G modulus, Poisson coefficient, thermal expansion.
//! - `StructuralCrossSection`: parametric shapes (rectangle, circle, pipe, I, T, L, U, box,
//!   tube, angle, channel, rolled I) by their parameters; manufactured profiles by their name;
//!   area and moments where given.
//! - `StructuralPointConnection`: the nodes.
//! - `StructuralCurveMember`: members through their nodes (a member of several nodes a member
//!   per segment, an arc drawn straight), with cross-section, rotation and behaviour.
//! - `StructuralSurfaceMember`: plates, walls and shells with their thickness (a variable one by
//!   its largest value) and material.
//! - `StructuralPointSupport` (in a node) and `RelConnectsStructuralMember` (hinges at member
//!   ends).
//! - `StructuralLoadGroup`, `StructuralLoadCase`, `StructuralLoadCombination`.
//! - `StructuralPointAction` (in a node or on a member), `StructuralPointMoment` (in a node),
//!   `StructuralCurveAction` (on a member), `StructuralSurfaceAction` (on a 2D member).
//!
//! What a workbook holds besides (curve and surface supports, openings, ribs, regions, results)
//! is counted in the notes.

mod units;

use calamine::{open_workbook_from_rs, Data, Reader, Xlsx};
pub use fem_model;
use fem_model::*;
use std::collections::HashMap;

/// The extensions of the format, lower case without the dot.
pub const EXTENSIONS: &[&str] = &["xlsx"];

/// One row of a sheet: its cells by their column's name (lower case), with the column's unit.
struct Row {
    cells: HashMap<String, (Data, Option<f64>)>,
}

impl Row {
    fn text(&self, key: &str) -> String {
        match self.cells.get(key).map(|c| &c.0) {
            None | Some(Data::Empty) => String::new(),
            Some(Data::Float(f)) if f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", *f as i64),
            Some(d) => d.to_string().trim().to_string(),
        }
    }

    /// A number in SI units (by the column's unit, else `default` as the unit's factor).
    fn number(&self, key: &str, default: f64) -> Option<f64> {
        let (d, unit) = self.cells.get(key)?;
        let v = match d {
            Data::Float(f) => *f,
            Data::Int(i) => *i as f64,
            Data::String(s) => s.trim().replace(',', ".").parse().ok()?,
            _ => return None,
        };
        v.is_finite().then(|| v * unit.unwrap_or(default))
    }

    /// The column's unit factor, or `default`.
    fn unit(&self, key: &str, default: f64) -> f64 {
        self.cells.get(key).and_then(|c| c.1).unwrap_or(default)
    }

    /// A list written `a; b; c`.
    fn list(&self, key: &str) -> Vec<String> {
        self.text(key).split(';').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    }

    /// The numbers of a list written `a; b; c`, by the column's unit.
    fn numbers(&self, key: &str, default: f64) -> Vec<f64> {
        let unit = self.unit(key, default);
        self.list(key).iter().filter_map(|s| s.replace(',', ".").parse::<f64>().ok()).map(|v| v * unit).collect()
    }

    fn is(&self, key: &str, value: &str) -> bool {
        self.text(key).eq_ignore_ascii_case(value)
    }
}

/// The rows of every sheet, by the sheet's name in lower case.
fn sheets(data: &[u8]) -> Result<HashMap<String, Vec<Row>>, String> {
    let mut book: Xlsx<_> = open_workbook_from_rs(std::io::Cursor::new(data)).map_err(|e| format!("not a workbook it can read: {e}"))?;
    let mut out = HashMap::new();
    for name in book.sheet_names() {
        let Ok(range) = book.worksheet_range(&name) else { continue };
        let mut rows = range.rows();
        let Some(head) = rows.next() else { continue };
        let columns: Vec<(String, Option<f64>)> = head.iter().map(|h| {
            let (name, unit) = units::header(&h.to_string());
            (name, unit.and_then(|u| units::factor(&u)))
        }).collect();
        let rows: Vec<Row> = rows
            .filter(|r| r.iter().any(|c| !matches!(c, Data::Empty)))
            .map(|r| Row { cells: columns.iter().zip(r).map(|((k, u), c)| (k.clone(), (c.clone(), *u))).collect() })
            .collect();
        out.insert(name.trim().to_lowercase(), rows);
    }
    Ok(out)
}

/// Reads a SAF workbook.
pub fn read(name: &str, data: &[u8]) -> Result<Document, String> {
    let sheets = sheets(data)?;
    let rows = |sheet: &str| sheets.get(sheet).map(Vec::as_slice).unwrap_or(&[]);
    if rows("structuralpointconnection").is_empty() {
        return Err("no sheet StructuralPointConnection: not a SAF workbook".into());
    }
    let mut a = Analysis { name: stem(name), ..Analysis::default() };
    let mut notes = Vec::new();

    // materials and cross-sections
    let mut materials: HashMap<String, usize> = HashMap::new();
    for r in rows("structuralmaterial") {
        let name = r.text("name");
        a.materials.push(Material {
            name: name.clone(),
            density: r.number("unit mass", 1.0),
            e: r.number("e modulus", 1e6),
            g: r.number("g modulus", 1e6),
            nu: r.number("poisson coefficient", 1.0),
            alpha: r.number("thermal expansion", 1.0),
        });
        materials.insert(name, a.materials.len() - 1);
    }
    let mut sections: HashMap<String, usize> = HashMap::new();
    for r in rows("structuralcrosssection") {
        let name = r.text("name");
        let p = r.numbers("parameters", 1e-3);
        let v = |k: usize| p.get(k).copied().unwrap_or(0.0);
        let shape = match r.text("shape").to_lowercase().as_str() {
            "rectangle" => Shape::Rectangle { b: v(1), h: v(0) },
            "circle" => Shape::Circle { d: v(0) },
            "pipe" => Shape::Tube { d: v(0), t: v(1) },
            "i section" => Shape::I { h: v(0), b: v(1).max(v(2)), tw: v(5), tf: v(3).max(v(4)) },
            "i rolled" => Shape::I { h: v(0), b: v(1), tw: v(3), tf: v(2) },
            "t section" => Shape::Tee { h: v(0), b: v(1), tw: v(3), tf: v(2) },
            "t tee" => Shape::Tee { h: v(0), b: v(1), tw: v(3), tf: v(2) },
            "l section" | "l section opposite" => Shape::Angle { h: v(0), b: v(1), t: v(2).max(v(3)) },
            "angle" => Shape::Angle { h: v(0), b: v(1), t: v(2) },
            "u section" => Shape::Channel { h: v(0), b: v(1), tw: v(3), tf: v(2) },
            "channel" => Shape::Channel { h: v(0), b: v(1), tw: v(3), tf: v(2) },
            "box" => Shape::Box { b: v(2), h: v(0), t: v(1).max(v(3)).max(v(4)) },
            "tube" => Shape::Box { b: v(1), h: v(0), t: v(2) },
            _ => Shape::Unknown,
        };
        let shape = match shape {
            // a shape without its sizes is no shape
            Shape::Rectangle { b, h } if b <= 0.0 || h <= 0.0 => Shape::Unknown,
            Shape::Circle { d } if d <= 0.0 => Shape::Unknown,
            s => s,
        };
        let props = SectionProps { area: r.number("a", 1.0), iy: r.number("iy", 1.0), iz: r.number("iz", 1.0), it: r.number("it", 1.0) };
        // a manufactured profile goes by its name (HEB180)
        let profile = r.text("profile");
        let label = if profile.is_empty() || profile == name { name.clone() } else { format!("{name} ({profile})") };
        a.sections.push(Section { name: label, shape, props, material: materials.get(&r.text("material")).copied() });
        sections.insert(name, a.sections.len() - 1);
    }

    // nodes
    let mut node_at: HashMap<String, usize> = HashMap::new();
    for r in rows("structuralpointconnection") {
        let name = r.text("name");
        let p = [r.number("coordinate x", 1.0).unwrap_or(0.0), r.number("coordinate y", 1.0).unwrap_or(0.0), r.number("coordinate z", 1.0).unwrap_or(0.0)];
        a.nodes.push(Node { name: name.clone(), p, support: None });
        node_at.insert(name, a.nodes.len() - 1);
    }

    // members, a member per segment
    let mut member_at: HashMap<String, Vec<usize>> = HashMap::new();
    let mut curved = 0;
    for r in rows("structuralcurvemember") {
        let name = r.text("name");
        let Some(nodes) = r.list("nodes").iter().map(|n| node_at.get(n).copied()).collect::<Option<Vec<usize>>>() else {
            notes.push(format!("member {name}: a node that is not there"));
            continue;
        };
        if r.list("segments").iter().any(|s| !s.eq_ignore_ascii_case("line")) {
            curved += 1;
        }
        let section = sections.get(&r.text("cross section")).copied();
        let kind = match (r.text("type").to_lowercase().as_str(), r.text("behaviour in analysis").to_lowercase().as_str()) {
            (_, "axial force only") => MemberKind::Truss,
            (t, _) if t.contains("column") => MemberKind::Column,
            (t, _) if t.contains("brace") || t.contains("bracing") => MemberKind::Brace,
            _ => MemberKind::Beam,
        };
        let angle = r.number("lcs rotation", 1.0).unwrap_or(0.0).to_radians();
        let segments = nodes.len().saturating_sub(1);
        for (k, pair) in nodes.windows(2).enumerate() {
            a.members.push(Member {
                name: if segments > 1 { format!("{name}.{}", k + 1) } else { name.clone() },
                kind,
                nodes: [pair[0], pair[1]],
                section,
                material: section.and_then(|s| a.sections[s].material),
                angle,
                orientation: None,
                releases: [[false; 6]; 2],
            });
            member_at.entry(name.clone()).or_default().push(a.members.len() - 1);
        }
    }
    if curved > 0 {
        notes.push(format!("{curved} members with curved segments: drawn straight between their nodes"));
    }

    // plates
    let mut plate_at: HashMap<String, usize> = HashMap::new();
    for r in rows("structuralsurfacemember") {
        let name = r.text("name");
        let Some(nodes) = r.list("nodes").iter().map(|n| node_at.get(n).copied()).collect::<Option<Vec<usize>>>() else {
            notes.push(format!("2D member {name}: a node that is not there"));
            continue;
        };
        // a thickness of one number, or `node: thickness` pairs of which the largest
        let unit = r.unit("thickness", 1e-3);
        let thickness = r.number("thickness", 1e-3).or_else(|| r.list("thickness").iter().filter_map(|s| s.rsplit(':').next()?.trim().replace(',', ".").parse::<f64>().ok()).map(|t| t * unit).reduce(f64::max));
        let kind = match (r.text("type").to_lowercase().as_str(), r.text("behavior in analysis").to_lowercase().as_str()) {
            (_, "membrane") => PlateKind::Membrane,
            ("wall", _) => PlateKind::Wall,
            ("plate", _) => PlateKind::Floor,
            _ => PlateKind::Shell,
        };
        a.plates.push(Plate { name: name.clone(), kind, nodes, thickness, material: materials.get(&r.text("material")).copied() });
        plate_at.insert(name, a.plates.len() - 1);
    }

    // supports in nodes, hinges at member ends
    let held = |r: &Row, force: f64| -> [Dof; 6] {
        let mut dofs = [Dof::Free; 6];
        for (d, (key, stiffness)) in [("ux", "stiffness x"), ("uy", "stiffness y"), ("uz", "stiffness z"), ("fix", "stiffness fix"), ("fiy", "stiffness fiy"), ("fiz", "stiffness fiz")].iter().enumerate() {
            dofs[d] = match r.text(key).to_lowercase().as_str() {
                "rigid" | "compression only" | "tension only" => Dof::Fixed,
                "flexible" => r.number(stiffness, force).map_or(Dof::Fixed, Dof::Spring),
                _ => Dof::Free,
            };
        }
        dofs
    };
    for r in rows("structuralpointsupport") {
        let Some(&n) = node_at.get(&r.text("node")) else {
            notes.push(format!("support {}: not in a node, left out", r.text("name")));
            continue;
        };
        let dofs = match r.text("type").to_lowercase().as_str() {
            "fixed" => [Dof::Fixed; 6],
            "hinged" => [Dof::Fixed, Dof::Fixed, Dof::Fixed, Dof::Free, Dof::Free, Dof::Free],
            _ => held(r, 1e6),
        };
        a.nodes[n].support = Some(dofs);
    }
    for r in rows("relconnectsstructuralmember") {
        let Some(members) = member_at.get(&r.text("member")) else { continue };
        let free = held(r, 1e6).map(|d| d == Dof::Free);
        let ends: &[usize] = match r.text("position").to_lowercase().as_str() {
            "begin" => &[0],
            "end" => &[1],
            _ => &[0, 1],
        };
        for &end in ends {
            // the first segment's start, the last segment's end
            let m = if end == 0 { members[0] } else { *members.last().unwrap() };
            a.members[m].releases[end] = free;
        }
    }

    // load groups and cases
    let group_kind: HashMap<String, String> = rows("structuralloadgroup").iter().map(|r| (r.text("name"), r.text("load group type"))).collect();
    let mut case_at: HashMap<String, usize> = HashMap::new();
    for r in rows("structuralloadcase") {
        let name = r.text("name");
        let kind = match r.text("action type").to_lowercase().as_str() {
            "permanent" => CaseKind::Permanent,
            "variable" => CaseKind::Variable,
            "accidental" => CaseKind::Accidental,
            _ => match group_kind.get(&r.text("load group")).map(|k| k.to_lowercase()) {
                Some(k) if k == "permanent" => CaseKind::Permanent,
                Some(k) if k == "variable" => CaseKind::Variable,
                Some(k) if k == "accidental" => CaseKind::Accidental,
                _ => CaseKind::Unknown,
            },
        };
        let self_weight = r.is("load type", "self weight").then_some(1.0);
        let label = match r.text("description") {
            d if d.is_empty() => name.clone(),
            d => format!("{name} · {d}"),
        };
        a.cases.push(LoadCase { name: label, kind, self_weight });
        case_at.insert(name, a.cases.len() - 1);
    }
    for r in rows("structuralloadcombination") {
        let mut terms = Vec::new();
        for k in 1.. {
            let case = r.text(&format!("load case name {k}"));
            if case.is_empty() && !r.cells.contains_key(&format!("load case name {k}")) {
                break;
            }
            let Some(&c) = case_at.get(&case) else { continue };
            let factor = r.number(&format!("load factor {k}"), 1.0).unwrap_or(1.0) * r.number(&format!("multiplier {k}"), 1.0).unwrap_or(1.0);
            terms.push((Term::Case(c), factor));
        }
        let kind = [r.text("category"), r.text("type")].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(", ");
        a.combinations.push(Combination { name: r.text("name"), kind, terms });
    }

    // loads
    let direction = |r: &Row| -> Option<[f64; 3]> {
        match r.text("direction").to_lowercase().as_str() {
            "x" => Some([1.0, 0.0, 0.0]),
            "y" => Some([0.0, 1.0, 0.0]),
            "z" => Some([0.0, 0.0, 1.0]),
            _ => None,
        }
    };
    let axes = |r: &Row| if r.is("coordinate system", "local") { Axes::Local } else { Axes::Global };
    let mut skipped: HashMap<String, usize> = HashMap::new();
    for (sheet, moment) in [("structuralpointaction", false), ("structuralpointmoment", true)] {
        for r in rows(sheet) {
            let Some(&case) = case_at.get(&r.text("load case")) else { continue };
            let value = r.number("value", 1e3).unwrap_or(0.0);
            let v = match direction(r) {
                Some(d) => d.map(|c| c * value),
                // a vector written (x; y; z)
                None => {
                    let unit = r.unit("vector (x;y;z)", 1e3);
                    let parts: Vec<f64> = r.text("vector (x;y;z)").trim_matches(['(', ')']).split(';').filter_map(|s| s.trim().replace(',', ".").parse().ok()).collect();
                    match parts.as_slice() {
                        [x, y, z] => [x * unit, y * unit, z * unit],
                        _ => continue,
                    }
                }
            };
            let in_node = r.is("force action", "in node") || (r.text("force action").is_empty() && !r.text("reference node").is_empty());
            if in_node {
                let Some(&node) = node_at.get(&r.text("reference node")) else { continue };
                let (force, moment_) = if moment { ([0.0; 3], v) } else { (v, [0.0; 3]) };
                a.actions.push(Action { case, load: Load::Node { node, force, moment: moment_ } });
            } else if moment {
                *skipped.entry("point moment on a member".into()).or_default() += 1;
            } else {
                let Some(members) = member_at.get(&r.text("reference member")) else { continue };
                let Some(at) = position(&a, members, r, "position x") else { continue };
                a.actions.push(Action { case, load: Load::MemberPoint { member: at.0, axes: axes(r), at: at.1, force: v } });
            }
        }
    }
    for r in rows("structuralcurveaction") {
        let Some(&case) = case_at.get(&r.text("load case")) else { continue };
        if !r.text("force action").is_empty() && !r.is("force action", "on beam") {
            *skipped.entry(format!("line load {}", r.text("force action").to_lowercase())).or_default() += 1;
            continue;
        }
        let Some(dir) = direction(r) else {
            *skipped.entry("line load along a vector".into()).or_default() += 1;
            continue;
        };
        let Some(members) = member_at.get(&r.text("member")) else { continue };
        let q1 = r.number("value 1", 1e3).unwrap_or(0.0);
        let q2 = if r.is("distribution", "trapez") { r.number("value 2", 1e3).unwrap_or(q1) } else { q1 };
        // the whole member, or a span of it: on each of its segments
        let (from, to) = if r.is("extent", "span") { (r.number("start point", 1.0).unwrap_or(0.0), r.number("end point", 1.0)) } else { (0.0, None) };
        let relative = r.is("coordinate definition", "relative");
        let total: f64 = members.iter().map(|&m| length(&a, m)).sum();
        let (from, to) = match (relative, to) {
            (true, Some(to)) => (from * total, to * total),
            (true, None) => (from * total, total),
            (false, to) => (from, to.unwrap_or(total)),
        };
        let (from, to) = if r.is("origin", "from end") { (total - to, total - from) } else { (from, to) };
        let mut start = 0.0;
        for &m in members {
            let l = length(&a, m);
            let (a0, a1) = (from.max(start), to.min(start + l));
            if a1 > a0 && l > 0.0 {
                let q = |x: f64| if to > from { q1 + (q2 - q1) * (x - from) / (to - from) } else { q1 };
                a.actions.push(Action { case, load: Load::MemberLine { member: m, axes: axes(r), dir, q: [q(a0), q(a1)], from: (a0 - start) / l, to: (a1 - start) / l } });
            }
            start += l;
        }
    }
    for r in rows("structuralsurfaceaction") {
        let Some(&case) = case_at.get(&r.text("load case")) else { continue };
        let Some(&plate) = plate_at.get(&r.text("2d member")) else {
            *skipped.entry("surface load not on a 2D member".into()).or_default() += 1;
            continue;
        };
        let Some(dir) = direction(r) else { continue };
        a.actions.push(Action { case, load: Load::Plate { plate, axes: axes(r), dir, q: r.number("value", 1e3).unwrap_or(0.0) } });
    }
    for (what, n) in skipped {
        notes.push(format!("{n} × {what}: not read"));
    }
    for sheet in ["structuralcurveconnection", "structuralsurfaceconnection", "structuraledgeconnection", "structuralsurfacememberopening", "structuralcurvememberrib", "structuralsurfacememberregion"] {
        let n = rows(sheet).len();
        if n > 0 {
            notes.push(format!("{n} rows of {sheet}: not read"));
        }
    }
    if a.is_empty() && a.nodes.is_empty() {
        return Err("no nodes, members or 2D members".into());
    }
    a.notes = notes.clone();
    Ok(Document { mesh: None, analysis: Some(a), format: "SAF (.xlsx)".into(), notes })
}

/// The length of member `m`.
fn length(a: &Analysis, m: usize) -> f64 {
    let [i, j] = a.members[m].nodes.map(|n| a.nodes[n].p);
    ((j[0] - i[0]).powi(2) + (j[1] - i[1]).powi(2) + (j[2] - i[2]).powi(2)).sqrt()
}

/// The segment of a member and the place on it (0 to 1) of a load's position.
fn position(a: &Analysis, members: &[usize], r: &Row, key: &str) -> Option<(usize, f64)> {
    let total: f64 = members.iter().map(|&m| length(a, m)).sum();
    let x = r.number(key, 1.0).unwrap_or(0.0);
    let x = if r.is("coordinate definition", "relative") { x * total } else { x };
    let x = if r.is("origin", "from end") { total - x } else { x };
    let mut start = 0.0;
    for &m in members {
        let l = length(a, m);
        if x <= start + l + 1e-9 || m == *members.last()? {
            return Some((m, if l > 0.0 { ((x - start) / l).clamp(0.0, 1.0) } else { 0.0 }));
        }
        start += l;
    }
    None
}

fn stem(name: &str) -> String {
    let file = name.rsplit(['/', '\\']).next().unwrap_or(name);
    file.rfind('.').map_or(file, |k| &file[..k]).to_string()
}
