import pytest  # type: ignore

import grimp


@pytest.mark.parametrize(
    "package_name",
    ("django", "flask", "requests", "sqlalchemy", "google.cloud.audit"),
)
def test_build_graph_on_real_package(package_name, snapshot):
    graph = grimp.build_graph(package_name, cache_dir=None)
    imports = graph.find_matching_direct_imports(import_expression="** -> **")
    assert imports == snapshot

    graph_from_rust = grimp.build_graph_rust(package_name, cache_dir=None)
    imports_from_rust = graph_from_rust.find_matching_direct_imports(import_expression="** -> **")
    sort_key = lambda i: (i["importer"], i["imported"])  # noqa:E731
    assert sorted(imports_from_rust, key=sort_key) == sorted(imports, key=sort_key)


@pytest.mark.parametrize(
    "package_name",
    ("django", "django.db", "django.db.models"),
)
def test_nominate_cycle_breakers_django(package_name, snapshot):
    graph = grimp.build_graph("django")

    cycle_breakers = graph.nominate_cycle_breakers(package_name)

    assert cycle_breakers == snapshot
