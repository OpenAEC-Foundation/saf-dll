//! The units SAF writes in its column headers (`Coordinate X [m]`, `Value [kN/m2]`), as the
//! factor that turns a value into SI units.

/// The SI factor of one unit, with the length of its text matched.
const UNITS: &[(&str, f64)] = &[
    ("GPa", 1e9),
    ("MPa", 1e6),
    ("kPa", 1e3),
    ("Pa", 1.0),
    ("ksi", 6_894_757.293_168),
    ("psi", 6_894.757_293_168),
    ("kNm", 1e3),
    ("MNm", 1e6),
    ("kN", 1e3),
    ("MN", 1e6),
    ("kip", 4_448.221_615_260_5),
    ("lbf", 4.448_221_615_260_5),
    ("lb", 0.453_592_37),
    ("kg", 1.0),
    ("N", 1.0),
    ("mm", 1e-3),
    ("cm", 1e-2),
    ("m", 1.0),
    ("ft", 0.3048),
    ("in", 0.0254),
    ("rad", 1.0),
    ("deg", 1.0),
    ("K", 1.0),
    ("°C", 1.0),
    ("°F", 1.0),
    ("-", 1.0),
    ("%", 1.0),
];

/// The factor of a product of units with powers (`kNm`, `m2`, `mm4`).
fn product(s: &str) -> Option<f64> {
    let mut rest = s.trim();
    let mut factor = 1.0;
    while !rest.is_empty() {
        let (unit, f) = UNITS.iter().find(|(u, _)| rest.starts_with(u))?;
        rest = &rest[unit.len()..];
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        rest = &rest[digits.len()..];
        factor *= f.powi(digits.parse().unwrap_or(1));
    }
    Some(factor)
}

/// The factor of a unit such as `kN/m2` or `MNm/rad/m`; `None` when it is not known.
pub fn factor(unit: &str) -> Option<f64> {
    let mut parts = unit.split('/');
    let mut f = product(parts.next()?)?;
    for denominator in parts {
        f /= product(denominator)?;
    }
    Some(f)
}

/// A column header as its name in lower case with single spaces, and its unit.
pub fn header(text: &str) -> (String, Option<String>) {
    let (name, unit) = match (text.find('['), text.rfind(']')) {
        (Some(a), Some(b)) if b > a => (format!("{}{}", &text[..a], &text[b + 1..]), Some(text[a + 1..b].trim().to_string())),
        _ => (text.to_string(), None),
    };
    (name.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase(), unit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_become_si_factors() {
        assert_eq!(factor("m"), Some(1.0));
        assert_eq!(factor("mm"), Some(1e-3));
        assert_eq!(factor("kN/m2"), Some(1e3));
        assert_eq!(factor("MPa"), Some(1e6));
        assert_eq!(factor("kg/m3"), Some(1.0));
        assert!((factor("m4").unwrap() - 1.0).abs() < 1e-12);
        assert!((factor("mm2").unwrap() - 1e-6).abs() < 1e-18);
        assert!((factor("ft").unwrap() - 0.3048).abs() < 1e-12);
        assert_eq!(factor("MNm/rad"), Some(1e6));
        assert_eq!(factor("furlong"), None);
        assert_eq!(header("Coordinate   X [m]"), ("coordinate x".to_string(), Some("m".to_string())));
        assert_eq!(header("Name"), ("name".to_string(), None));
    }
}
