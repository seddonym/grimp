import pytest

from grimp.adaptors.graph import NetworkXBackedImportGraph
from grimp.domain.valueobjects import Module, DirectImport, ImportPath


def test_modules_when_empty():
    graph = NetworkXBackedImportGraph()
    assert graph.modules == set()
    

def test_find_modules_directly_imported_by():
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo'), Module('bar'), Module('baz')
    d, e, f = Module('foo.one'), Module('bar.one'), Module('baz.one')

    graph.add_import(DirectImport(importer=a, imported=b))
    graph.add_import(DirectImport(importer=a, imported=c))
    graph.add_import(DirectImport(importer=a, imported=d))
    graph.add_import(DirectImport(importer=b, imported=e))
    graph.add_import(DirectImport(importer=f, imported=a))

    assert {b, c, d} == graph.find_modules_directly_imported_by(Module('foo'))


def test_find_modules_that_directly_import():
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo'), Module('bar'), Module('baz')
    d, e, f = Module('foo.one'), Module('bar.one'), Module('baz.one')

    graph.add_import(DirectImport(importer=a, imported=b))
    graph.add_import(DirectImport(importer=a, imported=c))
    graph.add_import(DirectImport(importer=a, imported=d))
    graph.add_import(DirectImport(importer=b, imported=e))
    graph.add_import(DirectImport(importer=f, imported=b))

    assert {a, f} == graph.find_modules_that_directly_import(Module('bar'))


@pytest.mark.parametrize(
    'module, as_subpackage, expected_result', (
        (Module('foo.a'), False, {Module('foo.b'), Module('foo.c'), Module('foo.a.d'),
                                  Module('foo.b.e')}),
        (Module('foo.b.e'), False, set()),
        (Module('foo.a'), True, {Module('foo.b'), Module('foo.c'), Module('foo.b.e'),
                                 Module('foo.b.g')}),
        (Module('foo.b.e'), True, set()),
    )
)
def test_find_downstream_modules(module, as_subpackage, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo.a'), Module('foo.b'), Module('foo.c')
    d, e, f = Module('foo.a.d'), Module('foo.b.e'), Module('foo.a.f')
    g = Module('foo.b.g')

    graph.add_import(DirectImport(imported=a, importer=b))
    graph.add_import(DirectImport(imported=a, importer=c))
    graph.add_import(DirectImport(imported=c, importer=d))
    graph.add_import(DirectImport(imported=d, importer=e))
    graph.add_import(DirectImport(imported=f, importer=b))
    graph.add_import(DirectImport(imported=f, importer=g))

    assert expected_result == graph.find_downstream_modules(
        module,
        as_subpackage=as_subpackage,
    )


@pytest.mark.parametrize(
    'module, as_subpackage, expected_result', (
        (Module('foo.d'), False, {Module('foo.d.c'), Module('foo.a')}),
        (Module('foo.b.g'), False, set()),
        (Module('foo.d'), True, {Module('foo.a'), Module('foo.a.f'), Module('foo.b.g')}),
        (Module('foo.b.g'), True, set()),
    )
)
def test_find_upstream_modules(module, as_subpackage, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo.a'), Module('foo.d.b'), Module('foo.d.c')
    d, e, f = Module('foo.d'), Module('foo.c.e'), Module('foo.a.f')
    g = Module('foo.b.g')

    graph.add_import(DirectImport(imported=a, importer=b))
    graph.add_import(DirectImport(imported=a, importer=c))
    graph.add_import(DirectImport(imported=c, importer=d))
    graph.add_import(DirectImport(imported=d, importer=e))
    graph.add_import(DirectImport(imported=f, importer=b))
    graph.add_import(DirectImport(imported=g, importer=f))

    assert expected_result == graph.find_upstream_modules(module,
                                                          as_subpackage=as_subpackage)


@pytest.mark.parametrize(
    'module, expected_result', (
        (Module('foo'), {Module('foo.a'), Module('foo.b'), Module('foo.c')}),
        (Module('foo.a'), {Module('foo.a.one')}),
        (Module('foo.c'), set()),
    )
)
def test_find_children(module, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo.a'), Module('foo.b'), Module('foo.c')
    d, e, f = Module('foo.a.one'), Module('foo.b.one'), Module('bar.g')

    for module_to_add in (a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_children(module)


@pytest.mark.parametrize(
    'module, expected_result', (
        (Module('foo'), {Module('foo.a'), Module('foo.b'), Module('foo.c'),
                         Module('foo.a.one'), Module('foo.b.one')}),
        (Module('foo.a'), {Module('foo.a.one')}),
        (Module('foo.c'), set()),
    )
)
def test_find_descendants(module, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo.a'), Module('foo.b'), Module('foo.c')
    d, e, f = Module('foo.a.one'), Module('foo.b.one'), Module('bar.g')

    for module_to_add in (a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_descendants(module)


def test_find_shortest_path_when_exists():
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo'), Module('bar'), Module('baz')
    d, e, f = Module('long'), Module('way'), Module('around')

    # Add short path.
    graph.add_import(DirectImport(importer=a, imported=b))
    graph.add_import(DirectImport(importer=b, imported=c))

    # Add longer path.
    graph.add_import(DirectImport(importer=a, imported=d))
    graph.add_import(DirectImport(importer=d, imported=e))
    graph.add_import(DirectImport(importer=e, imported=f))
    graph.add_import(DirectImport(importer=f, imported=c))

    assert ImportPath(a, b, c) == graph.find_shortest_path(
        upstream_module=a,
        downstream_module=c,
    )


def test_find_shortest_path_returns_none_if_not_exists():
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo'), Module('bar'), Module('baz')

    graph.add_import(DirectImport(importer=a, imported=b))
    graph.add_import(DirectImport(importer=b, imported=c))

    assert None == graph.find_shortest_path(
        upstream_module=c,
        downstream_module=a,
    )


@pytest.mark.parametrize(
    'upstream_module, downstream_module, as_subpackages, expected_result',
    (
        # as_subpackages not supplied.
        (Module('a.one'), Module('a.two'), None, True),  # Direct import.
        (Module('a.one'), Module('a.two.green'), None, False),  # No import.
        (Module('a.one'), Module('a.three'), None, True),  # Indirect import.
        (Module('c.one'), Module('b.two'), None, False),  # Downstream imports child of upstream.
        (Module('b.one'), Module('b.two'), None, False),  # Downstream child imports upstream.
        # Downstream child imports upstream child.
        (Module('a.one'), Module('b.two'), None, False),
        # Downstream grandchild imports upstream grandchild.
        (Module('a'), Module('b'), None, False),
        # Downstream grandchild imports upstream grandchild (indirectly).
        (Module('a'), Module('d'), True, True),
        # 'Weak dependency': downstream child imports module that does not import the upstream
        # module (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module.
        (Module('e'), Module('a'), None, False),

        # as_subpackages=False (this will be the same as the block of tests above).
        (Module('a.one'), Module('a.two'), False, True),  # Direct import.
        (Module('a.one'), Module('a.two.green'), False, False),  # No import.
        (Module('a.one'), Module('a.three'), False, True),  # Indirect import.
        (Module('c.one'), Module('b.two'), False, False),  # Downstream imports child of upstream.
        (Module('b.one'), Module('b.two'), False, False),  # Downstream child imports upstream.
        # Downstream child imports upstream child.
        (Module('a.one'), Module('b.two'), False, False),
        # Downstream grandchild imports upstream grandchild.
        (Module('a'), Module('b'), False, False),
        # Downstream grandchild imports upstream grandchild (indirectly).
        (Module('a'), Module('d'), True, True),
        # 'Weak dependency': downstream child imports module that does not import the upstream
        # module (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module.
        (Module('e'), Module('a'), False, False),
        #
        # # as_subpackages=True.
        (Module('a.one'), Module('a.two'), True, True),  # Direct import.
        (Module('a.one'), Module('a.two.green'), True, False),  # No import.
        (Module('a.one'), Module('a.three'), True, True),  # Indirect import.
        (Module('c.one'), Module('b.two'), True, True),  # Downstream imports child of upstream.
        (Module('b.one'), Module('b.two'), True, True),  # Downstream child imports upstream.
        # Downstream child imports upstream child.
        (Module('a.one'), Module('b.two'), True, True),
        # Downstream grandchild imports upstream grandchild.
        (Module('a'), Module('b'), True, True),
        # Downstream grandchild imports upstream grandchild (indirectly).
        (Module('a'), Module('d'), True, True),
        # 'Weak dependency': downstream child imports module that does not import the upstream
        # module (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module. We treat this as False as it's not really a dependency.
        (Module('e'), Module('a'), True, False),
    )
)
def test_path_exists(upstream_module, downstream_module, as_subpackages, expected_result):
    graph = NetworkXBackedImportGraph()
    a, a_one, a_one_green, a_two, a_two_green, a_three = (
        Module('a'),
        Module('a.one'),
        Module('a.one.green'),
        Module('a.two'),
        Module('a.two.green'),
        Module('a.three'),
    )
    b, b_one, b_two, b_two_green = (
        Module('b'),
        Module('b.one'),
        Module('b.two'),
        Module('b.two.green'),
    )
    c, c_one, c_one_green = Module('c'), Module('c.one'), Module('c.one.green')
    d, d_one, d_one_green = Module('d'), Module('d.one'), Module('d.one.green')
    e, e_one = Module('e'), Module('e.one')

    for module_to_add in (
        a, a_one, a_one_green, a_two, a_two_green, a_three,
        b, b_one, b_two, b_two_green,
        c, c_one, c_one_green,
        d, d_one, d_one_green,
        e, e_one,
    ):
        graph.add_module(module_to_add)

    for direct_import in (
        DirectImport(importer=a_two, imported=a_one),
        DirectImport(importer=c_one, imported=a_two),
        DirectImport(importer=a_three, imported=c_one),
        DirectImport(importer=b_two, imported=c_one_green),
        DirectImport(importer=b_two_green, imported=b_one),
        DirectImport(importer=b_two_green, imported=a_one_green),
        DirectImport(importer=d_one_green, imported=b_two_green),
        DirectImport(importer=e_one, imported=b_one),
    ):
        graph.add_import(direct_import)

    kwargs = dict(
        upstream_module=upstream_module,
        downstream_module=downstream_module,
    )
    if as_subpackages is not None:
        kwargs['as_subpackages'] = as_subpackages

    assert expected_result == graph.path_exists(**kwargs)


def test_add_module():
    graph = NetworkXBackedImportGraph()
    module = Module('foo')

    graph.add_module(module)

    assert graph.modules == {module}


@pytest.mark.parametrize('add_module', (True, False))
def test_add_import(add_module):
    graph = NetworkXBackedImportGraph()
    a, b = Module('foo'), Module('bar')

    # Adding the module should make no difference to the result.
    if add_module:
        graph.add_module(a)

    graph.add_import(DirectImport(importer=a, imported=b))

    assert {a, b} == graph.modules
    assert {b} == graph.find_modules_directly_imported_by(a)
    assert set() == graph.find_modules_directly_imported_by(b)


def test_remove_import():
    graph = NetworkXBackedImportGraph()
    a, b, c = Module('foo'), Module('bar'), Module('baz')
    graph.add_import(DirectImport(importer=a, imported=b))
    graph.add_import(DirectImport(importer=a, imported=c))

    graph.remove_import(DirectImport(importer=a, imported=b))

    assert {a, b, c} == graph.modules
    assert {c} == graph.find_modules_directly_imported_by(a)
