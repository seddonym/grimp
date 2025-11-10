use std::path::PathBuf;

use derive_new::new;

use crate::graph::Graph;

#[derive(Debug, Clone, new)]
pub struct PackageSpec {
    name: String,
    directory: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GraphBuilder {
    package: PackageSpec, // TODO(peter) Support multiple packages
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
}

impl GraphBuilder {
    pub fn new(package: PackageSpec) -> Self {
        GraphBuilder {
            package,
            include_external_packages: false,
            exclude_type_checking_imports: false,
        }
    }

    pub fn include_external_packages(mut self, yes: bool) -> Self {
        self.include_external_packages = yes;
        self
    }

    pub fn exclude_type_checking_imports(mut self, yes: bool) -> Self {
        self.exclude_type_checking_imports = yes;
        self
    }

    pub fn build(&self) -> Graph {
        todo!()
        // 1. Find all python files in the package.
        // Use the `ignore` crate.
        //
        // 2. For each python file in the package, parse the imports.
        // Use the existing `parse_imports_from_code` function.
        //
        // 3. For each python file in the package, resolve the imports.
        // You can reuse the existing logic in `scan_for_imports_no_py_single_module`,
        // but not directly. Copy the minimum, necessary code over to a new module
        // here in `rust/src/graph/builder/`.
        //
        // 4. Assemble the graph. Copy logic from the python implementation `_assemble_graph`.
        //
        // 5. Create a python usecase in `src/grimp/application/usecases.py` called `build_graph_rust`.
        //
        // * Do not do any parallelization yet.
        // * Do not do any caching yet.
    }
}
