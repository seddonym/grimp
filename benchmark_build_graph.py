#!/usr/bin/env python
"""Benchmark build_graph vs build_graph_rust."""

import argparse
import os
import sys
import time

import grimp


def benchmark_build_graph(package_name: str, working_dir: str | None = None) -> None:
    """Benchmark both graph building implementations."""
    if working_dir:
        os.chdir(working_dir)
        print(f"Changed directory to: {working_dir}")
        # Add working directory to Python path
        if working_dir not in sys.path:
            sys.path.insert(0, working_dir)
            print(f"Added to PYTHONPATH: {working_dir}\n")

    print(f"Benchmarking graph building for package: {package_name}")
    print("=" * 60)

    # Benchmark Python version
    print("\nPython version (build_graph):")
    start = time.perf_counter()
    graph_py = grimp.build_graph(package_name, cache_dir=None)
    elapsed_py = time.perf_counter() - start

    modules_py = len(graph_py.modules)
    imports_py = len(graph_py.find_matching_direct_imports(import_expression="** -> **"))

    print(f"  Time:    {elapsed_py:.3f}s")
    print(f"  Modules: {modules_py}")
    print(f"  Imports: {imports_py}")

    # Benchmark Rust version
    print("\nRust version (build_graph_rust):")
    start = time.perf_counter()
    graph_rust = grimp.build_graph_rust(package_name, cache_dir=None)
    elapsed_rust = time.perf_counter() - start

    modules_rust = len(graph_rust.modules)
    imports_rust = len(graph_rust.find_matching_direct_imports(import_expression="** -> **"))

    print(f"  Time:    {elapsed_rust:.3f}s")
    print(f"  Modules: {modules_rust}")
    print(f"  Imports: {imports_rust}")

    # Compare
    print("\n" + "=" * 60)
    print("Comparison:")
    speedup = elapsed_py / elapsed_rust if elapsed_rust > 0 else float("inf")
    print(f"  Speedup: {speedup:.2f}x")
    print(f"  Python:  {elapsed_py:.3f}s")
    print(f"  Rust:    {elapsed_rust:.3f}s")

    # Verify correctness
    if modules_py != modules_rust:
        print(f"\n⚠️  Warning: Module count mismatch ({modules_py} vs {modules_rust})")
    if imports_py != imports_rust:
        print(f"⚠️  Warning: Import count mismatch ({imports_py} vs {imports_rust})")


def main():
    parser = argparse.ArgumentParser(description="Benchmark build_graph vs build_graph_rust")
    parser.add_argument("package", help="Package name to analyze")
    parser.add_argument("-d", "--directory", help="Working directory to change to before running")

    args = parser.parse_args()

    try:
        benchmark_build_graph(args.package, args.directory)
    except Exception as e:
        print(f"\nError: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
