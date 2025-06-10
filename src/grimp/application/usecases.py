"""
Use cases handle application logic.
"""

from typing import Dict, Sequence, Set, Type, Union, cast, Iterable, Collection, Optional
import re

from ..application.ports import caching
from ..application.ports.filesystem import AbstractFileSystem
from ..application.ports.graph import ImportGraph
from ..application.ports.modulefinder import AbstractModuleFinder, FoundPackage, ModuleFile
from ..application.ports.packagefinder import AbstractPackageFinder
from ..domain.valueobjects import DirectImport, Module
from .config import settings
from grimp import _rustgrimp as rust  # type: ignore[attr-defined]

_LEADING_DOT_REGEX = re.compile(r"^(\.+)\w")


class NotSupplied:
    pass


# Calling code can set this environment variable if it wants to tune when to switch to
# multiprocessing, or set it to a large number to disable it altogether.
MIN_NUMBER_OF_MODULES_TO_SCAN_USING_MULTIPROCESSING_ENV_NAME = "GRIMP_MIN_MULTIPROCESSING_MODULES"
# This is an arbitrary number, but setting it too low slows down our functional tests considerably.
# If you change this, update docs/usage.rst too!
DEFAULT_MIN_NUMBER_OF_MODULES_TO_SCAN_USING_MULTIPROCESSING = 50


def build_graph(
    package_name,
    *additional_package_names,
    include_external_packages: bool = False,
    exclude_type_checking_imports: bool = False,
    cache_dir: Union[str, Type[NotSupplied], None] = NotSupplied,
) -> ImportGraph:
    """
    Build and return an import graph for the supplied package name(s).

    Args:
        - package_name: the name of the top level package for which to build the graph.
        - additional_package_names: tuple of the
        - include_external_packages: whether to include any external packages in the graph.
        - exclude_type_checking_imports: whether to exclude imports made in type checking guards.
        - cache_dir: The directory to use for caching the graph.
    Examples:

        # Single package.
        graph = build_graph("mypackage")
        graph = build_graph("mypackage", include_external_packages=True)
        graph = build_graph("mypackage", exclude_type_checking_imports=True)

        # Multiple packages.
        graph = build_graph("mypackage", "anotherpackage", "onemore")
        graph = build_graph(
            "mypackage", "anotherpackage", "onemore", include_external_packages=True,
        )
    """

    file_system: AbstractFileSystem = settings.FILE_SYSTEM

    found_packages = _find_packages(
        file_system=file_system,
        package_names=[package_name] + list(additional_package_names),
    )

    imports_by_module = _scan_packages(
        found_packages=found_packages,
        file_system=file_system,
        include_external_packages=include_external_packages,
        exclude_type_checking_imports=exclude_type_checking_imports,
        cache_dir=cache_dir,
    )

    graph = _assemble_graph(found_packages, imports_by_module)

    return graph


def _find_packages(
    file_system: AbstractFileSystem, package_names: Sequence[object]
) -> Set[FoundPackage]:
    package_names = _validate_package_names_are_strings(package_names)

    module_finder: AbstractModuleFinder = settings.MODULE_FINDER
    package_finder: AbstractPackageFinder = settings.PACKAGE_FINDER

    found_packages: Set[FoundPackage] = set()

    for package_name in package_names:
        package_directory = package_finder.determine_package_directory(
            package_name=package_name, file_system=file_system
        )
        found_package = module_finder.find_package(
            package_name=package_name,
            package_directory=package_directory,
            file_system=file_system,
        )
        found_packages.add(found_package)
    return found_packages


def _validate_package_names_are_strings(
    package_names: Sequence[object],
) -> Sequence[str]:
    for name in package_names:
        if not isinstance(name, str):
            raise TypeError(f"Package names must be strings, got {name.__class__.__name__}.")
    return cast(Sequence[str], package_names)


def _scan_packages(
    found_packages: Set[FoundPackage],
    file_system: AbstractFileSystem,
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    cache_dir: Union[str, Type[NotSupplied], None],
) -> Dict[Module, Set[DirectImport]]:
    if cache_dir is not None:
        cache_dir_if_supplied = cache_dir if cache_dir != NotSupplied else None
        cache: caching.Cache = settings.CACHE_CLASS.setup(
            file_system=file_system,
            found_packages=found_packages,
            include_external_packages=include_external_packages,
            exclude_type_checking_imports=exclude_type_checking_imports,
            cache_dir=cache_dir_if_supplied,
        )

    module_files_to_scan = {
        module_file
        for found_package in found_packages
        for module_file in found_package.module_files
    }

    imports_by_module_file: Dict[ModuleFile, Set[DirectImport]] = {}

    if cache_dir is not None:
        imports_by_module_file.update(_read_imports_from_cache(module_files_to_scan, cache=cache))

    remaining_module_files_to_scan = module_files_to_scan.difference(imports_by_module_file)
    if remaining_module_files_to_scan:
        imports_by_module_file.update(
            _scan_imports(
                remaining_module_files_to_scan,
                file_system=file_system,
                found_packages=found_packages,
                include_external_packages=include_external_packages,
                exclude_type_checking_imports=exclude_type_checking_imports,
            )
        )

    imports_by_module: Dict[Module, Set[DirectImport]] = {
        k.module: v for k, v in imports_by_module_file.items()
    }

    if cache_dir is not None:
        cache.write(imports_by_module)

    return imports_by_module


def _assemble_graph(
    found_packages: Set[FoundPackage],
    imports_by_module: Dict[Module, Set[DirectImport]],
) -> ImportGraph:
    graph: ImportGraph = settings.IMPORT_GRAPH_CLASS()

    package_modules = {Module(found_package.name) for found_package in found_packages}

    for module, direct_imports in imports_by_module.items():
        graph.add_module(module.name)
        for direct_import in direct_imports:
            # Before we add the import, check to see if the imported module is in fact an
            # external module, and if so, tell the graph that it is a squashed module.
            graph.add_module(
                direct_import.imported.name,
                is_squashed=_is_external(direct_import.imported, package_modules),
            )

            graph.add_import(
                importer=direct_import.importer.name,
                imported=direct_import.imported.name,
                line_number=direct_import.line_number,
                line_contents=direct_import.line_contents,
            )
    return graph


def _is_external(module: Module, package_modules: Set[Module]) -> bool:
    return not any(
        module.is_descendant_of(package_module) or module == package_module
        for package_module in package_modules
    )


def _read_imports_from_cache(
    module_files: Iterable[ModuleFile], *, cache: caching.Cache
) -> Dict[ModuleFile, Set[DirectImport]]:
    imports_by_module_file: Dict[ModuleFile, Set[DirectImport]] = {}
    for module_file in module_files:
        try:
            direct_imports = cache.read_imports(module_file)
        except caching.CacheMiss:
            continue
        else:
            imports_by_module_file[module_file] = direct_imports
    return imports_by_module_file


def _scan_imports(
    module_files: Collection[ModuleFile],
    *,
    file_system: AbstractFileSystem,
    found_packages: Set[FoundPackage],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
) -> Dict[ModuleFile, Set[DirectImport]]:
    # Preparation
    # ===========

    modules: Set[Module] = set()
    for package in found_packages:
        modules |= {mf.module for mf in package.module_files}

    found_packages_by_module: Dict[Module, FoundPackage] = {
        module_file.module: package
        for package in found_packages
        for module_file in package.module_files
    }

    module_filenames_by_module = {}
    for module_file in module_files:
        found_package = found_packages_by_module[module_file.module]
        module_filename = _determine_module_filename(
            file_system, module_file.module, found_package
        )
        module_filenames_by_module[module_file.module] = module_filename

    # Scan raw imported objects in parallel via rust
    # ==============================================

    imported_objects_by_module_name = rust.parse_imported_objects(
        [
            (module_file.module.name, module_filenames_by_module[module_file.module])
            for module_file in module_files
        ]
    )

    # Post-processing of raw imports to obtain final result
    # =====================================================

    imports_by_module_file = {}
    for module_file in module_files:
        module = module_file.module
        module_filename = module_filenames_by_module[module]

        is_package = _module_is_package(file_system, module_filename)

        imports = set()
        for imported_object in imported_objects_by_module_name[module_file.module.name]:
            # Filter on `exclude_type_checking_imports`.
            if exclude_type_checking_imports and imported_object["typechecking_only"]:
                continue

            # Resolve relative imports.
            imported_object_name = _get_absolute_imported_object_name(
                module=module, is_package=is_package, imported_object_name=imported_object["name"]
            )

            # Resolve imported module.
            imported_module = _get_internal_module(imported_object_name, modules=modules)
            if imported_module is None:
                # => External import.

                # Filter on `include_external_packages`.
                if not include_external_packages:
                    continue

                # Distill module.
                imported_module = _distill_external_module(
                    Module(imported_object_name), found_packages=found_packages
                )
                if imported_module is None:
                    continue

            imports.add(
                DirectImport(
                    importer=module,
                    imported=imported_module,
                    line_number=imported_object["line_number"],
                    line_contents=imported_object["line_contents"],
                )
            )

        imports_by_module_file[module_file] = imports

    return imports_by_module_file


def _determine_module_filename(
    file_system: AbstractFileSystem, module: Module, found_package: FoundPackage
) -> str:
    """
    Work out the full filename of the given module.

    Any given module can either be a straight Python file (foo.py) or else a package
    (in which case the file is an __init__.py within a directory).
    """
    top_level_components = found_package.name.split(".")
    module_components = module.name.split(".")
    leaf_components = module_components[len(top_level_components) :]
    package_directory = found_package.directory

    filename_root = file_system.join(package_directory, *leaf_components)
    candidate_filenames = (
        f"{filename_root}.py",
        file_system.join(filename_root, "__init__.py"),
    )
    for candidate_filename in candidate_filenames:
        if file_system.exists(candidate_filename):
            return candidate_filename
    raise FileNotFoundError(f"Could not find module {module}.")


def _module_is_package(file_system: AbstractFileSystem, module_filename: str) -> bool:
    """
    Whether or not the supplied module filename is a package.
    """
    return file_system.split(module_filename)[-1] == "__init__.py"


def _get_absolute_imported_object_name(
    *, module: Module, is_package: bool, imported_object_name: str
) -> str:
    leading_dot_match = _LEADING_DOT_REGEX.match(imported_object_name)
    if leading_dot_match is None:
        return imported_object_name

    n_leading_dots = len(leading_dot_match.group(1))
    if is_package:
        if n_leading_dots == 1:
            imported_object_name_base = module.name
        else:
            imported_object_name_base = ".".join(module.name.split(".")[: -(n_leading_dots - 1)])
    else:
        imported_object_name_base = ".".join(module.name.split(".")[:-n_leading_dots])
    return imported_object_name_base + "." + imported_object_name[n_leading_dots:]


def _get_internal_module(object_name: str, *, modules: Set[Module]) -> Optional[Module]:
    candidate_module = Module(object_name)
    if candidate_module in modules:
        return candidate_module

    try:
        candidate_module = candidate_module.parent
    except ValueError:
        return None
    else:
        if candidate_module in modules:
            return candidate_module
        else:
            return None


def _distill_external_module(
    module: Module, *, found_packages: Set[FoundPackage]
) -> Optional[Module]:
    """
    Given a module that we already know is external, turn it into a module to add to the graph.

    The 'distillation' process involves removing any unwanted subpackages. For example,
    Module("django.models.db") should be turned into simply Module("django").

    The process is more complex for potential namespace packages, as it's not possible to
    determine the portion package simply from name. Rather than adding the overhead of a
    filesystem read, we just get the shallowest component that does not clash with an internal
    module namespace. Take, for example, a Module("foo.blue.alpha.one"). If one of the found
    packages is foo.blue.beta, the module will be distilled to Module("foo.blue.alpha").
    Alternatively, if the found package is foo.green, the distilled module will
    be Module("foo.blue").
    """
    # If it's a module that is a parent of one of the internal packages, return None
    # as it doesn't make sense and is probably an import of a namespace package.
    if any(Module(package.name).is_descendant_of(module) for package in found_packages):
        return None

    # If it shares a namespace with an internal module, get the shallowest component that does
    # not clash with an internal module namespace.
    candidate_portions: Set[Module] = set()
    for found_package in sorted(found_packages, key=lambda p: p.name, reverse=True):
        root_module = Module(found_package.name)
        if root_module.is_descendant_of(module.root):
            (
                internal_path_components,
                external_path_components,
            ) = root_module.name.split(
                "."
            ), module.name.split(".")
            external_namespace_components = []
            while external_path_components[0] == internal_path_components[0]:
                external_namespace_components.append(external_path_components[0])
                external_path_components = external_path_components[1:]
                internal_path_components = internal_path_components[1:]
            external_namespace_components.append(external_path_components[0])
            candidate_portions.add(Module(".".join(external_namespace_components)))

    if candidate_portions:
        # If multiple found packages share a namespace with this module, use the deepest one
        # as we know that that will be a namespace too.
        deepest_candidate_portion = sorted(
            candidate_portions, key=lambda p: len(p.name.split("."))
        )[-1]
        return deepest_candidate_portion
    else:
        return module.root
