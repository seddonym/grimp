import pytest

import grimp


@pytest.fixture(params=["python", "rust"], ids=["python", "rust"])
def build_graph(request):
    """Fixture that provides both Python and Rust graph building implementations."""
    if request.param == "python":
        return grimp.build_graph
    else:
        return grimp.build_graph_rust
