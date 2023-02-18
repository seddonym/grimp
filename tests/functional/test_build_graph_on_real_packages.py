import pytest  # type: ignore

import grimp


@pytest.mark.parametrize(
    "package_name",
    ("django", "flask", "requests", "sqlalchemy", "google.cloud.audit"),
)
def test_build_graph_on_real_package(package_name):
    # All we care about is whether or not the graph builds without raising an exception.
    grimp.build_graph(package_name, cache_dir=None)
