import math
import os
from typing import Collection, Set, Dict, Iterable

import joblib  # type: ignore

from grimp import _rustgrimp as rust  # type: ignore[attr-defined]
from grimp.domain.valueobjects import DirectImport
from grimp.application.config import settings
from grimp.application.ports.filesystem import AbstractFileSystem
from grimp.application.ports.modulefinder import ModuleFile, FoundPackage

ImportScanner = rust.ImportScanner

# Calling code can set this environment variable if it wants to tune when to switch to
# multiprocessing, or set it to a large number to disable it altogether.
MIN_NUMBER_OF_MODULES_TO_SCAN_USING_MULTIPROCESSING_ENV_NAME = "GRIMP_MIN_MULTIPROCESSING_MODULES"
# This is an arbitrary number, but setting it too low slows down our functional tests considerably.
# If you change this, update docs/usage.rst too!
DEFAULT_MIN_NUMBER_OF_MODULES_TO_SCAN_USING_MULTIPROCESSING = 50


def scan_imports(
    module_files: Collection[ModuleFile],
    *,
    found_packages: Set[FoundPackage],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
) -> Dict[ModuleFile, Set[DirectImport]]:
    chunks = _create_chunks(module_files)
    return _scan_chunks(
        chunks,
        found_packages,
        include_external_packages,
        exclude_type_checking_imports,
    )


def _create_chunks(module_files: Collection[ModuleFile]) -> tuple[tuple[ModuleFile, ...], ...]:
    """
    Split the module files into chunks, each to be worked on by a separate OS process.
    """
    module_files_tuple = tuple(module_files)

    number_of_module_files = len(module_files_tuple)
    n_chunks = _decide_number_of_processes(number_of_module_files)
    chunk_size = math.ceil(number_of_module_files / n_chunks)

    return tuple(
        module_files_tuple[i * chunk_size : (i + 1) * chunk_size] for i in range(n_chunks)
    )


def _decide_number_of_processes(number_of_module_files: int) -> int:
    min_number_of_modules = int(
        os.environ.get(
            MIN_NUMBER_OF_MODULES_TO_SCAN_USING_MULTIPROCESSING_ENV_NAME,
            DEFAULT_MIN_NUMBER_OF_MODULES_TO_SCAN_USING_MULTIPROCESSING,
        )
    )
    if number_of_module_files < min_number_of_modules:
        # Don't incur the overhead of multiple processes.
        return 1
    return min(joblib.cpu_count(), number_of_module_files)


def _scan_chunk(
    found_packages: Set[FoundPackage],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
    chunk: Iterable[ModuleFile],
) -> Dict[ModuleFile, Set[DirectImport]]:
    file_system: AbstractFileSystem = settings.FILE_SYSTEM
    basic_file_system = file_system.convert_to_basic()
    import_scanner = ImportScanner(
        file_system=basic_file_system,
        found_packages=found_packages,
        # Ensure that the passed exclude_type_checking_imports is definitely a boolean,
        # otherwise the Rust class will error.
        include_external_packages=bool(include_external_packages),
    )
    return {
        module_file: import_scanner.scan_for_imports(
            module_file.module, exclude_type_checking_imports=exclude_type_checking_imports
        )
        for module_file in chunk
    }


def _scan_chunks(
    chunks: Collection[Collection[ModuleFile]],
    found_packages: Set[FoundPackage],
    include_external_packages: bool,
    exclude_type_checking_imports: bool,
) -> Dict[ModuleFile, Set[DirectImport]]:
    number_of_processes = len(chunks)
    import_scanning_jobs = joblib.Parallel(n_jobs=number_of_processes)(
        joblib.delayed(_scan_chunk)(
            found_packages, include_external_packages, exclude_type_checking_imports, chunk
        )
        for chunk in chunks
    )

    imports_by_module_file = {}
    for chunk_imports_by_module_file in import_scanning_jobs:
        imports_by_module_file.update(chunk_imports_by_module_file)
    return imports_by_module_file
