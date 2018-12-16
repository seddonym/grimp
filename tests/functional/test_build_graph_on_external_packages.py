import pytest  # type: ignore

import grimp


@pytest.mark.parametrize('package_name', (
    'django',
    'flask',
    'requests',
    'sqlalchemy',
))
def test_build_graph_on_external_package(package_name):
    # All we care about is whether or not the graph builds without raising an exception.
    grimp.build_graph(package_name)
