use pyo3::prelude::*;
use pyo3::types::{PyUnicode, PyDict};
use std::collections::{HashSet, HashMap};



#[test]
fn test_bidi() {
    let blue = "blue";
    let green = "green";
    let yellow = "yellow";
    let blue_alpha = "blue.alpha";
    let blue_beta = "blue.beta";

    let importers_by_imported: HashMap<&str, HashSet<&str>> =
        HashMap::from([
            (blue, HashSet::from([green, blue_alpha])),
            (green, HashSet::from([yellow, blue_beta])),
            (blue_alpha, HashSet::new()),
            (yellow, HashSet::new()),
            (blue_beta, HashSet::new()),
        ]);
    let importeds_by_importer: HashMap<&str, HashSet<&str>> = HashMap::from([
            (green, HashSet::from([blue])),
            (blue_alpha, HashSet::from([blue])),
            (yellow, HashSet::from([green])),
            (blue_beta, HashSet::from([green])),
            (blue, HashSet::new()),
        ]);
    let path_or_none: Option<Vec<&str>> = _bidirectional_shortest_path(
        &yellow, &blue, importers_by_imported, importeds_by_importer
    );
    assert_eq!(path_or_none, Some(Vec::from([yellow, green, blue])));
}

/// Return a tuple of modules in the shortest path between importer and imported.
//
//  If no path can be found, return None.
//
//  Args:
//    importer: the module doing the importing; the starting point.
//    imported: the module being imported: the end point.
//    importers_by_imported: Map of modules directly imported by each key.
//    importeds_by_importer: Map of all the modules that directly import each key.
#[pyfunction]
fn bidirectional_shortest_path<'a>(
    importer: &'a PyUnicode,
    imported: &'a PyUnicode,
    importers_by_imported: &'a PyDict,
    importeds_by_importer: &'a PyDict,
) -> PyResult<Option<Vec<&'a str>>> {
    let _importer: &str = importer.extract()?;
    let _imported: &str = imported.extract()?;
    let _importers_by_imported: HashMap<&str, HashSet<&str>> = importers_by_imported.extract()?;
    let _importeds_by_importer: HashMap<&str, HashSet<&str>> = importeds_by_importer.extract()?;

    let path_or_none: Option<Vec<&str>> = _bidirectional_shortest_path(
        _importer, _imported, _importers_by_imported, _importeds_by_importer
    );
    Ok(path_or_none)
}


fn _bidirectional_shortest_path<'a>(
    importer: &'a str,
    imported: &'a str,
    importers_by_imported: HashMap<&'a str, HashSet<&'a str>>,
    importeds_by_importer: HashMap<&'a str, HashSet<&'a str>>,
) -> Option<Vec<&'a str>> {
    let results_or_none = search_for_path(
        importer,
        imported,
        importers_by_imported,
        importeds_by_importer
    );
    match results_or_none {
        Some(results) => {

            let (pred, succ, initial_w) = results;

            let mut w_or_none: Option<&str> = Some(initial_w);
            // Transform results into tuple.
            let mut path: Vec<&str> = Vec::new();
            // From importer to w:
            while w_or_none.is_some() {
                let w = w_or_none.unwrap();
                path.push(w);
                w_or_none = pred[&w];
            }
            path.reverse();

            // From w to imported:
            w_or_none = succ[path.last().unwrap()];
            while w_or_none.is_some() {
                let w = w_or_none.unwrap();
                path.push(w);
                w_or_none = succ[&w];
            }

            Some(path)
        },
        None => None
    }
}
/// Performs a breadth first search from both source and target, meeting in the middle.
//
//  Returns:
//      (pred, succ, w) where
//         - pred is a dictionary of predecessors from w to the source, and
//         - succ is a dictionary of successors from w to the target.
//
fn search_for_path<'a>(
    importer: &'a str,
    imported: &'a str,
    importers_by_imported: HashMap<&'a str, HashSet<&'a str>>,
    importeds_by_importer: HashMap<&'a str, HashSet<&'a str>>,
) -> Option<
        (
            HashMap<&'a str, Option<&'a str>>,
            HashMap<&'a str, Option<&'a str>>,
            &'a str
        )
     >
{
    if importer == imported {

        Some(
            (
                HashMap::from([
                    (imported, None),
                ]),
                HashMap::from([
                    (importer, None),
                ]),
                importer
            )
        )
    }
    else {
        let mut pred: HashMap<&str, Option<&str>> = HashMap::from([(importer, None)]);
        let mut succ: HashMap<&str, Option<&str>> = HashMap::from([(imported, None)]);

        // Initialize fringes, start with forward.
        let mut forward_fringe: Vec<&str> = Vec::from([importer]);
        let mut reverse_fringe: Vec<&str> = Vec::from([imported]);
        let mut this_level: Vec<&str>;

        while forward_fringe.len() > 0 && reverse_fringe.len() > 0 {
            if forward_fringe.len() <= reverse_fringe.len() {
                this_level = forward_fringe.to_vec();
                forward_fringe = Vec::new();
                for v in this_level {
                    for w in &importeds_by_importer[v] {
                        if !pred.contains_key(w) {
                            forward_fringe.push(w);
                            pred.insert(w, Some(v));
                        }
                        if succ.contains_key(w) {
                            // Found path.
                            return Some(
                                (
                                    pred,
                                    succ,
                                    w,
                                )
                            )
                        }
                        // TOD
                    }
                }
            } else {
                this_level = reverse_fringe.to_vec();
                reverse_fringe = Vec::new();
                for v in this_level {
                    for w in &importers_by_imported[v] {
                        if !succ.contains_key(w) {
                            succ.insert(w, Some(v));
                            reverse_fringe.push(w);
                        }
                        if pred.contains_key(w) {
                            // Found path.
                            return Some(
                                (
                                    pred,
                                    succ,
                                    w,
                                )
                            )
                        }
                    }
                }
            }
        }
        None
    }
}


/// A Python module implemented in Rust.
#[pymodule]
fn _grimp_rust(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(bidirectional_shortest_path, m)?)?;
    //m.add_function(wrap_pyfunction!(foo, m)?)?;
    Ok(())
}