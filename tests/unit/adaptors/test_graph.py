import pytest  # type: ignore

from grimp.adaptors.graph import NetworkXBackedImportGraph


def test_modules_when_empty():
    graph = NetworkXBackedImportGraph()
    assert graph.modules == set()


def test_find_modules_directly_imported_by():
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo', 'bar', 'baz'
    d, e, f = 'foo.one', 'bar.one', 'baz.one'

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)
    graph.add_import(importer=a, imported=d)
    graph.add_import(importer=b, imported=e)
    graph.add_import(importer=f, imported=a)

    assert {b, c, d} == graph.find_modules_directly_imported_by('foo')


def test_find_modules_that_directly_import():
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo', 'bar', 'baz'
    d, e, f = 'foo.one', 'bar.one', 'baz.one'

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)
    graph.add_import(importer=a, imported=d)
    graph.add_import(importer=b, imported=e)
    graph.add_import(importer=f, imported=b)

    assert {a, f} == graph.find_modules_that_directly_import('bar')


@pytest.mark.parametrize(
    'module, as_package, expected_result', (
        ('foo.a', False, {'foo.b', 'foo.c', 'foo.a.d', 'foo.b.e'}),
        ('foo.b.e', False, set()),
        ('foo.a', True, {'foo.b', 'foo.c', 'foo.b.e', 'foo.b.g'}),
        ('foo.b.e', True, set()),
    )
)
def test_find_downstream_modules(module, as_package, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo.a', 'foo.b', 'foo.c'
    d, e, f = 'foo.a.d', 'foo.b.e', 'foo.a.f'
    g = 'foo.b.g'

    graph.add_import(imported=a, importer=b)
    graph.add_import(imported=a, importer=c)
    graph.add_import(imported=c, importer=d)
    graph.add_import(imported=d, importer=e)
    graph.add_import(imported=f, importer=b)
    graph.add_import(imported=f, importer=g)

    assert expected_result == graph.find_downstream_modules(
        module,
        as_package=as_package,
    )


@pytest.mark.parametrize(
    'module, as_package, expected_result', (
        ('foo.d', False, {'foo.d.c', 'foo.a'}),
        ('foo.b.g', False, set()),
        ('foo.d', True, {'foo.a', 'foo.a.f', 'foo.b.g'}),
        ('foo.b.g', True, set()),
    )
)
def test_find_upstream_modules(module, as_package, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo.a', 'foo.d.b', 'foo.d.c'
    d, e, f = 'foo.d', 'foo.c.e', 'foo.a.f'
    g = 'foo.b.g'

    graph.add_import(imported=a, importer=b)
    graph.add_import(imported=a, importer=c)
    graph.add_import(imported=c, importer=d)
    graph.add_import(imported=d, importer=e)
    graph.add_import(imported=f, importer=b)
    graph.add_import(imported=g, importer=f)

    assert expected_result == graph.find_upstream_modules(module, as_package=as_package)


@pytest.mark.parametrize(
    'module, expected_result', (
        ('foo', {'foo.a', 'foo.b', 'foo.c'}),
        ('foo.a', {'foo.a.one'}),
        ('foo.c', set()),
    )
)
def test_find_children(module, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo.a', 'foo.b', 'foo.c'
    d, e, f = 'foo.a.one', 'foo.b.one', 'bar.g'

    for module_to_add in (a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_children(module)


@pytest.mark.parametrize(
    'module, expected_result', (
        ('foo', {'foo.a', 'foo.b', 'foo.c', 'foo.a.one', 'foo.b.one'}),
        ('foo.a', {'foo.a.one'}),
        ('foo.c', set()),
    )
)
def test_find_descendants(module, expected_result):
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo.a', 'foo.b', 'foo.c'
    d, e, f = 'foo.a.one', 'foo.b.one', 'bar.g'

    for module_to_add in (a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_descendants(module)


def test_find_shortest_path_when_exists():
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo', 'bar', 'baz'
    d, e, f = 'long', 'way', 'around'

    # Add short path.
    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=b, imported=c)

    # Add longer path.
    graph.add_import(importer=a, imported=d)
    graph.add_import(importer=d, imported=e)
    graph.add_import(importer=e, imported=f)
    graph.add_import(importer=f, imported=c)

    assert (a, b, c) == graph.find_shortest_path(
        upstream_module=a,
        downstream_module=c,
    )


def test_find_shortest_path_returns_none_if_not_exists():
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo', 'bar', 'baz'

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=b, imported=c)

    assert None is graph.find_shortest_path(
        upstream_module=c,
        downstream_module=a,
    )


@pytest.mark.parametrize(
    'upstream_module, downstream_module, as_packages, expected_result',
    (
        # as_packages not supplied.
        ('a.one', 'a.two', None, True),  # Direct import.
        ('a.one', 'a.two.green', None, False),  # No import.
        ('a.one', 'a.three', None, True),  # Indirect import.
        ('c.one', 'b.two', None, False),  # Downstream imports child of upstream.
        ('b.one', 'b.two', None, False),  # Downstream child imports upstream.
        # Downstream child imports upstream child.
        ('a.one', 'b.two', None, False),
        # Downstream grandchild imports upstream grandchild.
        ('a', 'b', None, False),
        # Downstream grandchild imports upstream grandchild (indirectly).
        ('a', 'd', True, True),
        # 'Weak dependency': downstream child imports module that does not import the upstream
        # module (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module.
        ('e', 'a', None, False),

        # as_packages=False (this will be the same as the block of tests above).
        ('a.one', 'a.two', False, True),  # Direct import.
        ('a.one', 'a.two.green', False, False),  # No import.
        ('a.one', 'a.three', False, True),  # Indirect import.
        ('c.one', 'b.two', False, False),  # Downstream imports child of upstream.
        ('b.one', 'b.two', False, False),  # Downstream child imports upstream.
        # Downstream child imports upstream child.
        ('a.one', 'b.two', False, False),
        # Downstream grandchild imports upstream grandchild.
        ('a', 'b', False, False),
        # Downstream grandchild imports upstream grandchild (indirectly).
        ('a', 'd', True, True),
        # 'Weak dependency': downstream child imports module that does not import the upstream
        # module (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module.
        ('e', 'a', False, False),
        #
        # # as_packages=True.
        ('a.one', 'a.two', True, True),  # Direct import.
        ('a.one', 'a.two.green', True, False),  # No import.
        ('a.one', 'a.three', True, True),  # Indirect import.
        ('c.one', 'b.two', True, True),  # Downstream imports child of upstream.
        ('b.one', 'b.two', True, True),  # Downstream child imports upstream.
        # Downstream child imports upstream child.
        ('a.one', 'b.two', True, True),
        # Downstream grandchild imports upstream grandchild.
        ('a', 'b', True, True),
        # Downstream grandchild imports upstream grandchild (indirectly).
        ('a', 'd', True, True),
        # 'Weak dependency': downstream child imports module that does not import the upstream
        # module (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module. We treat this as False as it's not really a dependency.
        ('e', 'a', True, False),
    )
)
def test_path_exists(upstream_module, downstream_module, as_packages, expected_result):
    graph = NetworkXBackedImportGraph()
    a, a_one, a_one_green, a_two, a_two_green, a_three = (
        'a',
        'a.one',
        'a.one.green',
        'a.two',
        'a.two.green',
        'a.three',
    )
    b, b_one, b_two, b_two_green = (
        'b',
        'b.one',
        'b.two',
        'b.two.green',
    )
    c, c_one, c_one_green = 'c', 'c.one', 'c.one.green'
    d, d_one, d_one_green = 'd', 'd.one', 'd.one.green'
    e, e_one = 'e', 'e.one'

    for module_to_add in (
        a, a_one, a_one_green, a_two, a_two_green, a_three,
        b, b_one, b_two, b_two_green,
        c, c_one, c_one_green,
        d, d_one, d_one_green,
        e, e_one,
    ):
        graph.add_module(module_to_add)

    for importer, imported in (
        (a_two, a_one),
        (c_one, a_two),
        (a_three, c_one),
        (b_two, c_one_green),
        (b_two_green, b_one),
        (b_two_green, a_one_green),
        (d_one_green, b_two_green),
        (e_one, b_one),
    ):
        graph.add_import(importer=importer, imported=imported)

    kwargs = dict(
        upstream_module=upstream_module,
        downstream_module=downstream_module,
    )
    if as_packages is not None:
        kwargs['as_packages'] = as_packages

    assert expected_result == graph.path_exists(**kwargs)


def test_add_module():
    graph = NetworkXBackedImportGraph()
    module = 'foo'

    graph.add_module(module)

    assert graph.modules == {module}


@pytest.mark.parametrize('add_module', (True, False))
def test_add_import(add_module):
    graph = NetworkXBackedImportGraph()
    a, b = 'foo', 'bar'

    # Adding the module should make no difference to the result.
    if add_module:
        graph.add_module(a)

    graph.add_import(importer=a, imported=b)

    assert {a, b} == graph.modules
    assert {b} == graph.find_modules_directly_imported_by(a)
    assert set() == graph.find_modules_directly_imported_by(b)


def test_remove_import():
    graph = NetworkXBackedImportGraph()
    a, b, c = 'foo', 'bar', 'baz'
    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)

    graph.remove_import(importer=a, imported=b)

    assert {a, b, c} == graph.modules
    assert {c} == graph.find_modules_directly_imported_by(a)


class TestGetImportDetails:
    def test_happy_path(self):
        graph = NetworkXBackedImportGraph()

        imports_info = [
            dict(
                importer='mypackage.foo',
                imported='mypackage.bar',
                line_number=1,
                line_contents='from . import bar',
            ),
            dict(
                importer='mypackage.foo',
                imported='mypackage.bar',
                line_number=10,
                line_contents='from .bar import a_function',
            )
        ]
        for import_info in imports_info:
            graph.add_import(**import_info)

        assert imports_info == graph.get_import_details(
            importer='mypackage.foo', imported='mypackage.bar')

    def test_returns_empty_list_when_no_import(self):
        graph = NetworkXBackedImportGraph()

        assert [] == graph.get_import_details(importer='foo', imported='bar')

    def test_returns_only_relevant_imports(self):
        graph = NetworkXBackedImportGraph()

        imports_info = [
            dict(
                importer='mypackage.foo',
                imported='mypackage.bar',
                line_number=1,
                line_contents='from . import bar',
            ),
        ]
        graph.add_import(**imports_info[0])

        # Also add a different import in the same module.
        graph.add_import(
            importer='mypackage.foo',
            imported='mypackage.baz',
            line_number=2,
            line_contents='from . import baz'
        )

        assert imports_info == graph.get_import_details(
            importer='mypackage.foo', imported='mypackage.bar')
