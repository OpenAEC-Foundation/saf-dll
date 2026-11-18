//! A SAF workbook made by hand: a portal frame of two columns and a beam with a floor plate,
//! supports, a hinge, load cases, loads and a combination.

use rust_xlsxwriter::Workbook;
use saf_read::fem_model::*;

/// A cell: a number or a text.
enum C {
    N(f64),
    T(&'static str),
}
use C::*;

fn sheet(book: &mut Workbook, name: &str, rows: &[&[C]]) {
    let ws = book.add_worksheet();
    ws.set_name(name).unwrap();
    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            match cell {
                N(v) => ws.write_number(r as u32, c as u16, *v).unwrap(),
                T(s) => ws.write_string(r as u32, c as u16, *s).unwrap(),
            };
        }
    }
}

fn portal() -> Vec<u8> {
    let mut book = Workbook::new();
    sheet(&mut book, "StructuralMaterial", &[&[T("Name"), T("Type"), T("Unit mass [kg/m3]"), T("E modulus [MPa]"), T("G modulus [MPa]"), T("Poisson Coefficient")], &[T("S235"), T("Steel"), N(7850.0), N(210000.0), N(81000.0), N(0.3)]]);
    sheet(
        &mut book,
        "StructuralCrossSection",
        &[
            &[T("Name"), T("Material"), T("Cross-section type"), T("Shape"), T("Parameters [mm]"), T("Profile"), T("A [m2]")],
            &[T("CS1"), T("S235"), T("Parametric"), T("Rectangle"), T("300; 200"), T(""), N(0.06)],
            &[T("CS2"), T("S235"), T("Manufactured"), T(""), T(""), T("HEB180"), N(0.006525)],
        ],
    );
    sheet(
        &mut book,
        "StructuralPointConnection",
        &[
            &[T("Name"), T("Coordinate X [m]"), T("Coordinate Y [m]"), T("Coordinate Z [m]")],
            &[T("N1"), N(0.0), N(0.0), N(0.0)],
            &[T("N2"), N(0.0), N(0.0), N(3.0)],
            &[T("N3"), N(2.0), N(0.0), N(3.0)],
            &[T("N4"), N(4.0), N(0.0), N(3.0)],
            &[T("N5"), N(4.0), N(0.0), N(0.0)],
            &[T("N6"), N(0.0), N(2.0), N(3.0)],
            &[T("N7"), N(4.0), N(2.0), N(3.0)],
        ],
    );
    sheet(
        &mut book,
        "StructuralCurveMember",
        &[
            &[T("Name"), T("Type"), T("Cross section"), T("Nodes"), T("Segments"), T("LCS Rotation [deg]"), T("Behaviour in analysis")],
            &[T("C1"), T("Column"), T("CS1"), T("N1; N2"), T("Line"), N(90.0), T("Standard")],
            &[T("B1"), T("Beam"), T("CS2"), T("N2; N3; N4"), T("Line; Line"), N(0.0), T("Standard")],
            &[T("C2"), T("Column"), T("CS1"), T("N5; N4"), T("Line"), N(0.0), T("Standard")],
        ],
    );
    sheet(
        &mut book,
        "StructuralSurfaceMember",
        &[
            &[T("Name"), T("Type"), T("Material"), T("Thickness type"), T("Thickness [mm]"), T("Nodes")],
            &[T("S1"), T("Plate"), T("S235"), T("Constant"), N(200.0), T("N2; N4; N7; N6")],
        ],
    );
    sheet(
        &mut book,
        "StructuralPointSupport",
        &[
            &[T("Name"), T("Type"), T("Node"), T("ux"), T("uy"), T("uz"), T("fix"), T("fiy"), T("fiz"), T("Stiffness Z [MN/m]")],
            &[T("Sn1"), T("Fixed"), T("N1"), T("Rigid"), T("Rigid"), T("Rigid"), T("Rigid"), T("Rigid"), T("Rigid"), T("")],
            &[T("Sn2"), T("Custom"), T("N5"), T("Rigid"), T("Rigid"), T("Flexible"), T("Free"), T("Free"), T("Free"), N(100.0)],
        ],
    );
    sheet(
        &mut book,
        "RelConnectsStructuralMember",
        &[&[T("Name"), T("Member"), T("Position"), T("ux"), T("uy"), T("uz"), T("fix"), T("fiy"), T("fiz")], &[T("H1"), T("B1"), T("End"), T("Rigid"), T("Rigid"), T("Rigid"), T("Rigid"), T("Free"), T("Free")]],
    );
    sheet(&mut book, "StructuralLoadGroup", &[&[T("Name"), T("Load group type")], &[T("LG1"), T("Permanent")], &[T("LG2"), T("Variable")]]);
    sheet(
        &mut book,
        "StructuralLoadCase",
        &[
            &[T("Name"), T("Description"), T("Action type"), T("Load group"), T("Load type")],
            &[T("LC1"), T("Self weight"), T("Permanent"), T("LG1"), T("Self weight")],
            &[T("LC2"), T(""), T("Variable"), T("LG2"), T("Static")],
        ],
    );
    sheet(
        &mut book,
        "StructuralLoadCombination",
        &[
            &[T("Name"), T("Category"), T("Type"), T("Load factor 1"), T("Multiplier 1"), T("Load case name 1"), T("Load factor 2"), T("Multiplier 2"), T("Load case name 2")],
            &[T("CO1"), T("ULS"), T("Linear"), N(1.35), N(1.0), T("LC1"), N(1.5), N(1.0), T("LC2")],
        ],
    );
    sheet(
        &mut book,
        "StructuralPointAction",
        &[
            &[T("Name"), T("Direction"), T("Force action"), T("Reference node"), T("Reference member"), T("Value [kN]"), T("Load case"), T("Coordinate definition"), T("Position x [m]")],
            &[T("F1"), T("X"), T("In node"), T("N2"), T(""), N(10.0), T("LC2"), T(""), T("")],
            &[T("F2"), T("Z"), T("On beam"), T(""), T("B1"), N(-20.0), T("LC2"), T("Relative"), N(0.75)],
        ],
    );
    sheet(
        &mut book,
        "StructuralCurveAction",
        &[
            &[T("Name"), T("Force action"), T("Distribution"), T("Direction"), T("Value 1 [kN/m]"), T("Value 2 [kN/m]"), T("Member"), T("Load case"), T("Coordinate definition"), T("Extent")],
            &[T("L1"), T("On beam"), T("Uniform"), T("Z"), N(-5.0), T(""), T("B1"), T("LC2"), T("Relative"), T("Full")],
        ],
    );
    sheet(&mut book, "StructuralSurfaceAction", &[&[T("Name"), T("Direction"), T("Force action"), T("Value [kN/m2]"), T("2D Member"), T("Load case")], &[T("P1"), T("Z"), T("On 2D member"), N(-2.5), T("S1"), T("LC2")]]);
    book.save_to_buffer().unwrap()
}

#[test]
fn a_saf_workbook_becomes_an_analysis_model_in_si_units() {
    let doc = saf_read::read("portal.xlsx", &portal()).expect("SAF");
    let a = doc.analysis.expect("analysis");
    assert_eq!(a.nodes.len(), 7);
    // the beam of three nodes is two segments
    assert_eq!(a.members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["C1", "B1.1", "B1.2", "C2"]);
    assert_eq!(a.members[0].kind, MemberKind::Column);
    assert!((a.members[0].angle - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    assert_eq!(a.sections[0].shape, Shape::Rectangle { b: 0.2, h: 0.3 });
    assert_eq!(a.sections[1].name, "CS2 (HEB180)");
    assert_eq!(a.sections[1].props.area, Some(0.006525));
    assert_eq!(a.materials[0].e, Some(210e9));
    assert_eq!((a.plates[0].thickness, a.plates[0].kind), (Some(0.2), PlateKind::Floor));
    assert_eq!(a.nodes[0].support, Some([Dof::Fixed; 6]));
    assert_eq!(a.nodes[4].support.unwrap()[2], Dof::Spring(100e6));
    // the hinge at the end of B1 is at the end of its last segment
    assert_eq!(a.members[2].releases[1], [false, false, false, false, true, true]);
    assert_eq!(a.members[1].releases[1], [false; 6]);
    assert_eq!(a.cases.len(), 2);
    assert_eq!((a.cases[0].kind.clone(), a.cases[0].self_weight), (CaseKind::Permanent, Some(1.0)));
    assert_eq!(a.combinations[0].terms, [(Term::Case(0), 1.35), (Term::Case(1), 1.5)]);
    assert!(a.actions.iter().any(|x| matches!(x.load, Load::Node { node: 1, force, .. } if force == [10e3, 0.0, 0.0])));
    // at three quarters of the beam: half way its second segment
    assert!(a.actions.iter().any(|x| matches!(x.load, Load::MemberPoint { member: 2, at, force, .. } if (at - 0.5).abs() < 1e-9 && force == [0.0, 0.0, -20e3])));
    // the line load on both segments
    assert_eq!(a.actions.iter().filter(|x| matches!(x.load, Load::MemberLine { q, .. } if q == [-5e3, -5e3])).count(), 2);
    assert!(a.actions.iter().any(|x| matches!(x.load, Load::Plate { plate: 0, q, .. } if q == -2.5e3)));
}

#[test]
fn a_workbook_without_nodes_is_no_saf_file() {
    let mut book = Workbook::new();
    sheet(&mut book, "Sheet1", &[&[T("a"), T("b")], &[N(1.0), N(2.0)]]);
    let error = saf_read::read("other.xlsx", &book.save_to_buffer().unwrap()).unwrap_err();
    assert!(error.contains("not a SAF workbook"), "{error}");
}
