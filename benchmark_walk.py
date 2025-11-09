"""Benchmark os.walk vs powerwalk for package discovery."""

import os
import time
from collections.abc import Iterable

import powerwalk

from grimp.adaptors.filesystem import FileSystem
from grimp.adaptors.modulefinder import ModuleFinder
from grimp.application.ports import modulefinder
from grimp.domain.valueobjects import Module


class PowerwalkModuleFinder(modulefinder.AbstractModuleFinder):
    """ModuleFinder using powerwalk directly, ignoring AbstractFileSystem."""

    def find_package(
        self, package_name: str, package_directory: str, file_system=None
    ) -> modulefinder.FoundPackage:
        module_files: list[modulefinder.ModuleFile] = []

        for module_filename in self._get_python_files_inside_package(package_directory):
            module_name = self._module_name_from_filename(
                package_name, module_filename, package_directory
            )
            module_mtime = os.path.getmtime(module_filename)
            module_files.append(
                modulefinder.ModuleFile(module=Module(module_name), mtime=module_mtime)
            )

        return modulefinder.FoundPackage(
            name=package_name,
            directory=package_directory,
            module_files=frozenset(module_files),
        )

    def _get_python_files_inside_package(self, directory: str) -> Iterable[str]:
        """
        Get a list of Python files within the supplied package directory using powerwalk.

        Return:
            Generator of Python file names.
        """
        for entry in powerwalk.walk(directory, filter="**/*.py"):
            if entry.is_dir:
                continue

            yield entry.path_str

    def _module_name_from_filename(
        self, package_name: str, filename_and_path: str, package_directory: str
    ) -> str:
        """
        Args:
            package_name (string) - the importable name of the top level package. Could
                be namespaced.
            filename_and_path (string) - the full name of the Python file.
            package_directory (string) - the full path of the top level Python package directory.
         Returns:
            Absolute module name for importing (string).
        """
        internal_filename_and_path = filename_and_path[len(package_directory) :]
        internal_filename_and_path_without_extension = internal_filename_and_path[1:-3]
        components = [package_name] + internal_filename_and_path_without_extension.split(os.sep)
        if components[-1] == "__init__":
            components.pop()
        return ".".join(components)


def benchmark(package_name: str, package_directory: str, num_runs: int = 10):
    """Run benchmarks comparing both module finder implementations."""
    print(f"Benchmarking package discovery")
    print(f"Package: {package_name}")
    print(f"Directory: {package_directory}\n")

    os_walk_finder = ModuleFinder()
    powerwalk_finder = PowerwalkModuleFinder()
    file_system = FileSystem()

    # Warm-up and verify both produce same results
    print("Running warm-up and verification...")
    result_os_walk = os_walk_finder.find_package(package_name, package_directory, file_system)
    result_powerwalk = powerwalk_finder.find_package(package_name, package_directory)

    modules_os_walk = {mf.module.name for mf in result_os_walk.module_files}
    modules_powerwalk = {mf.module.name for mf in result_powerwalk.module_files}

    print(f"os.walk found: {len(modules_os_walk)} modules")
    print(f"powerwalk found: {len(modules_powerwalk)} modules")

    # Check for differences
    only_in_os_walk = modules_os_walk - modules_powerwalk
    only_in_powerwalk = modules_powerwalk - modules_os_walk

    if only_in_os_walk:
        print(f"\nWARNING: {len(only_in_os_walk)} modules only found by os.walk:")
        for m in sorted(only_in_os_walk)[:10]:
            print(f"  {m}")
        if len(only_in_os_walk) > 10:
            print(f"  ... and {len(only_in_os_walk) - 10} more")

    if only_in_powerwalk:
        print(f"\nWARNING: {len(only_in_powerwalk)} modules only found by powerwalk:")
        for m in sorted(only_in_powerwalk)[:10]:
            print(f"  {m}")
        if len(only_in_powerwalk) > 10:
            print(f"  ... and {len(only_in_powerwalk) - 10} more")

    print(f"\n{'=' * 60}")
    print(f"Running {num_runs} iterations each...\n")

    # Benchmark os.walk
    os_walk_times = []
    for i in range(num_runs):
        start = time.perf_counter()
        result = os_walk_finder.find_package(package_name, package_directory, file_system)
        elapsed = time.perf_counter() - start
        os_walk_times.append(elapsed)
        print(f"os.walk run {i + 1}: {elapsed:.4f}s")

    print()

    # Benchmark powerwalk
    powerwalk_times = []
    for i in range(num_runs):
        start = time.perf_counter()
        result = powerwalk_finder.find_package(package_name, package_directory)
        elapsed = time.perf_counter() - start
        powerwalk_times.append(elapsed)
        print(f"powerwalk run {i + 1}: {elapsed:.4f}s")

    # Calculate statistics
    print(f"\n{'=' * 60}")
    print("Results:")
    print(f"{'=' * 60}")

    os_walk_avg = sum(os_walk_times) / len(os_walk_times)
    os_walk_min = min(os_walk_times)
    os_walk_max = max(os_walk_times)

    powerwalk_avg = sum(powerwalk_times) / len(powerwalk_times)
    powerwalk_min = min(powerwalk_times)
    powerwalk_max = max(powerwalk_times)

    print(f"\nos.walk:")
    print(f"  Average: {os_walk_avg:.4f}s")
    print(f"  Min:     {os_walk_min:.4f}s")
    print(f"  Max:     {os_walk_max:.4f}s")

    print(f"\npowerwalk:")
    print(f"  Average: {powerwalk_avg:.4f}s")
    print(f"  Min:     {powerwalk_min:.4f}s")
    print(f"  Max:     {powerwalk_max:.4f}s")

    speedup = os_walk_avg / powerwalk_avg
    print(f"\nSpeedup: {speedup:.2f}x")

    if speedup > 1:
        print(f"✓ powerwalk is {speedup:.2f}x faster")
    elif speedup < 1:
        print(f"✗ powerwalk is {1 / speedup:.2f}x slower")
    else:
        print("≈ Both methods have similar performance")


if __name__ == "__main__":
    benchmark("octoenergy", "/Users/peter.byfield/projects/kraken-core/src/octoenergy")
