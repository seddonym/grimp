#!/usr/bin/env python
"""Benchmark build_graph vs build_graph_rust."""

import argparse
import os
import shutil
import sys
import time
from dataclasses import dataclass
from typing import Callable

import grimp


@dataclass
class BenchmarkResult:
    """Result of a single benchmark run."""

    name: str
    elapsed: float
    modules: int
    imports: int


def run_benchmark(
    name: str,
    build_func: Callable,
    package_name: str,
    cache_dir: str | None,
) -> BenchmarkResult:
    """Run a single benchmark and return the result."""
    print(f"\n{name}:")
    start = time.perf_counter()
    graph = build_func(package_name, cache_dir=cache_dir)
    elapsed = time.perf_counter() - start

    modules = len(graph.modules)
    imports = len(graph.find_matching_direct_imports(import_expression="** -> **"))

    print(f"  Time:    {elapsed:.3f}s")
    print(f"  Modules: {modules}")
    print(f"  Imports: {imports}")

    return BenchmarkResult(name, elapsed, modules, imports)


def cleanup_cache_dir(cache_dir: str) -> None:
    """Remove cache directory."""
    if os.path.exists(cache_dir):
        shutil.rmtree(cache_dir)


def print_comparison(
    py_results: list[BenchmarkResult], rust_results: list[BenchmarkResult]
) -> None:
    """Print comparison of benchmark results."""
    py_no_cache, py_cold, py_warm = py_results
    rust_no_cache, rust_cold, rust_warm = rust_results

    print("\n" + "=" * 60)
    print("Comparison:")
    print(f"  Python (no cache):          {py_no_cache.elapsed:.3f}s")
    print(
        f"  Python (cold cache):        {py_cold.elapsed:.3f}s  "
        f"({py_no_cache.elapsed / py_cold.elapsed:.2f}x speedup)"
    )
    print(
        f"  Python (warm cache):        {py_warm.elapsed:.3f}s  "
        f"({py_no_cache.elapsed / py_warm.elapsed:.2f}x speedup)"
    )
    print(
        f"  Rust (no cache):            {rust_no_cache.elapsed:.3f}s  "
        f"({py_no_cache.elapsed / rust_no_cache.elapsed:.2f}x vs Python no cache)"
    )
    print(
        f"  Rust (cold cache):          {rust_cold.elapsed:.3f}s  "
        f"({py_no_cache.elapsed / rust_cold.elapsed:.2f}x vs Python no cache)"
    )
    print(
        f"  Rust (warm cache):          {rust_warm.elapsed:.3f}s  "
        f"({py_no_cache.elapsed / rust_warm.elapsed:.2f}x vs Python no cache)"
    )
    print(f"\n  Python cache speedup:       {py_no_cache.elapsed / py_warm.elapsed:.2f}x")
    print(f"  Rust cache speedup:         {rust_no_cache.elapsed / rust_warm.elapsed:.2f}x")

    # Verify correctness
    if py_no_cache.modules != rust_no_cache.modules:
        print(
            f"\n⚠️  Warning: Module count mismatch "
            f"({py_no_cache.modules} vs {rust_no_cache.modules})"
        )
    if py_no_cache.imports != rust_no_cache.imports:
        print(
            f"⚠️  Warning: Import count mismatch ({py_no_cache.imports} vs {rust_no_cache.imports})"
        )


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

    cache_dir = ".grimp_cache_benchmark"

    # Benchmark Python version
    py_no_cache = run_benchmark(
        "Python version without cache (build_graph)",
        grimp.build_graph,
        package_name,
        None,
    )

    cleanup_cache_dir(cache_dir)
    py_cold = run_benchmark(
        "Python version with cache - first run (cold cache)",
        grimp.build_graph,
        package_name,
        cache_dir,
    )

    py_warm = run_benchmark(
        "Python version with cache - second run (warm cache)",
        grimp.build_graph,
        package_name,
        cache_dir,
    )

    # Benchmark Rust version
    rust_no_cache = run_benchmark(
        "Rust version without cache (build_graph_rust)",
        grimp.build_graph_rust,
        package_name,
        None,
    )

    cleanup_cache_dir(cache_dir)
    rust_cold = run_benchmark(
        "Rust version with cache - first run (cold cache)",
        grimp.build_graph_rust,
        package_name,
        cache_dir,
    )

    rust_warm = run_benchmark(
        "Rust version with cache - second run (warm cache)",
        grimp.build_graph_rust,
        package_name,
        cache_dir,
    )

    cleanup_cache_dir(cache_dir)

    # Print comparison
    print_comparison(
        [py_no_cache, py_cold, py_warm],
        [rust_no_cache, rust_cold, rust_warm],
    )


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
