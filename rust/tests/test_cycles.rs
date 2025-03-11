use _rustgrimp::graph::Graph;

#[test]
fn test_find_cycles() {
    // Given
    let mut graph = Graph::default();
    let dependencies: Vec<Vec<String>> = vec![
        vec!["A".to_string(), "B".to_string()],
        vec!["B".to_string(), "C".to_string()],
        vec!["C".to_string(), "A".to_string()],
    ];

    for modules in dependencies {
        let importer_name = &modules[0];
        let imported_name = &modules[1];
        let importer = graph.get_or_add_module(importer_name).token();
        let imported = graph.get_or_add_module(imported_name).token();
        graph.add_import(importer, imported)
    }

    let expected_cycles: Vec<Vec<String>> = vec![
        vec!["A".to_string(), "B".to_string(), "C".to_string()],
    ];
    // When
    let cycles = graph.find_cycles();
    // Then
    assert_eq!(cycles, expected_cycles);
}
