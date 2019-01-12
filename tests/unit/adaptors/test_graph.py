import re
import pytest  # type: ignore

from grimp.adaptors.graph import ImportGraph


class TestRepr:
    """
    Because sets are unordered, it's difficult to test the repr in a deterministic way,
    so we use regular expressions instead.
    """
    def test_no_modules(self):
        assert '<ImportGraph: empty>' == repr(ImportGraph())

    def test_untruncated(self):
        modules = ['one', 'two', 'three', 'four', 'five']
        graph = ImportGraph()
        for module in modules:
            graph.add_module(module)

        # Assert something in the form <ImportGraph: 'one', 'two', 'three', 'four', 'five'>
        # (but not necessarily in that order).
        re_part = "(" + '|'.join(modules) + ")"
        assert re.match(
            f"<ImportGraph: '{re_part}', '{re_part}', '{re_part}', '{re_part}', '{re_part}'>",
            repr(graph)
        )

    def test_truncated(self):
        modules = ['one', 'two', 'three', 'four', 'five', 'six']
        graph = ImportGraph()
        for module in modules:
            graph.add_module(module)

        re_part = "(" + '|'.join(modules) + ")"
        assert re.match(
            f"<ImportGraph: '{re_part}', '{re_part}', '{re_part}', "
            f"'{re_part}', '{re_part}', ...>",
            repr(graph)
        )


def test_modules_when_empty():
    graph = ImportGraph()
    assert graph.modules == set()


def test_find_modules_directly_imported_by():
    graph = ImportGraph()
    a, b, c = 'foo', 'bar', 'baz'
    d, e, f = 'foo.one', 'bar.one', 'baz.one'

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)
    graph.add_import(importer=a, imported=d)
    graph.add_import(importer=b, imported=e)
    graph.add_import(importer=f, imported=a)

    assert {b, c, d} == graph.find_modules_directly_imported_by('foo')


def test_find_modules_that_directly_import():
    graph = ImportGraph()
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
    graph = ImportGraph()
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
    graph = ImportGraph()
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
    graph = ImportGraph()
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
    graph = ImportGraph()
    a, b, c = 'foo.a', 'foo.b', 'foo.c'
    d, e, f = 'foo.a.one', 'foo.b.one', 'bar.g'

    for module_to_add in (a, b, c, d, e, f):
        graph.add_module(module_to_add)

    assert expected_result == graph.find_descendants(module)


def test_find_shortest_chain_when_exists():
    graph = ImportGraph()
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

    assert (a, b, c) == graph.find_shortest_chain(
        importer=a,
        imported=c,
    )


def test_find_shortest_chain_returns_none_if_not_exists():
    graph = ImportGraph()
    a, b, c = 'foo', 'bar', 'baz'

    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=b, imported=c)

    assert None is graph.find_shortest_chain(
        importer=c,
        imported=a,
    )


@pytest.mark.parametrize(
    'importer, imported, as_packages, expected_result',
    (
        # This block: as_packages not supplied.
        ('a.two', 'a.one', None, True),  # Importer directly imports imported.
        ('a.three', 'a.one', None, True),  # Importer indirectly imports imported.
        # Importer does not import the imported, even indirectly.
        ('a.two.green', 'a.one', None, False),
        ('b.two', 'c.one', None, False),  # Importer imports the child of the imported.
        ('b.two', 'b', None, False),  # Importer is child of imported (but doesn't import).
        # Importer's child imports imported's child.
        ('b.two', 'a.one', None, False),
        # Importer's grandchild directly imports imported's grandchild.
        ('b', 'a', None, False),
        # Importer's grandchild indirectly imports imported's grandchild.
        ('d', 'a', None, False),
        # 'Weak dependency': importer's child imports module that does not import imported
        # (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module.
        # The chains are: e.one -> b.one; b.two -> c.one.green; c.one -> a.two.
        ('e', 'a', None, False),

        # This block: as_packages=False (should be identical to block above).
        ('a.two', 'a.one', False, True),
        ('a.three', 'a.one', False, True),
        ('a.two.green', 'a.one', False, False),
        ('b.two', 'c.one', False, False),
        ('b.two', 'b', False, False),
        ('b.two', 'a.one', False, False),
        ('b', 'a', False, False),
        ('d', 'a', False, False),
        ('e', 'a', False, False),

        # This block: as_packages=True.
        ('a.two', 'a.one', True, True),  # Importer directly imports imported.
        ('a.three', 'a.one', True, True),  # Importer indirectly imports imported.
        # Importer does not import the imported, even indirectly.
        ('a.two.green', 'a.one', True, False),
        # Importer imports the child of the imported (b.two -> c.one.green).
        ('b.two', 'c.one', True, True),
        # Importer is child of imported (but doesn't import). This doesn't
        # make sense if as_packages is True, so it should raise an exception.
        ('b.two', 'b', True, ValueError()),
        # Importer's child imports imported's child (b.two.green -> a.one.green).
        ('b.two', 'a.one', True, True),
        # Importer's grandchild directly imports imported's grandchild
        # (b.two.green -> a.one.green).
        ('b', 'a', True, True),
        # Importer's grandchild indirectly imports imported's grandchild.
        # (d.one.green -> b.two.green -> a.one.green).
        ('d', 'a', True, True),
        # 'Weak dependency': importer's child imports module that does not import imported
        # (even directly). However another module in the intermediate subpackage *does*
        # import the upstream module. We treat this as False as it's not really a dependency.
        # The chains are: e.one -> b.one; b.two -> c.one.green; c.one -> a.two.
        ('e', 'a', True, False),
    )
)
def test_chain_exists(importer, imported, as_packages, expected_result):
    """
    Build a graph to analyse for chains. This is much easier to debug visually,
    so here is the dot syntax for the graph, which can be viewed using a dot file viewer.

        digraph {
            a;
            a_one;
            a_one_green;
            a_two -> a_one;
            a_two_green;
            a_three -> c_one;
            b;
            b_one;
            b_two -> c_one_green;
            b_two_green -> b_one;
            b_two_green -> a_one_green;
            c;
            c_one -> a_two;
            c_one_green;
            d;
            d_one;
            d_one_green -> b_two_green;
            e;
            e_one -> b_one;
        }
    """
    graph = ImportGraph()
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

    for _importer, _imported in (
        (a_two, a_one),
        (c_one, a_two),
        (a_three, c_one),
        (b_two, c_one_green),
        (b_two_green, b_one),
        (b_two_green, a_one_green),
        (d_one_green, b_two_green),
        (e_one, b_one),
    ):
        graph.add_import(importer=_importer, imported=_imported)

    kwargs = dict(
        imported=imported,
        importer=importer,
    )
    if as_packages is not None:
        kwargs['as_packages'] = as_packages

    if isinstance(expected_result, Exception):
        with pytest.raises(expected_result.__class__):
            graph.chain_exists(**kwargs)
    else:
        assert expected_result == graph.chain_exists(**kwargs)


def test_add_module():
    graph = ImportGraph()
    module = 'foo'

    graph.add_module(module)

    assert graph.modules == {module}


@pytest.mark.parametrize('add_module', (True, False))
def test_add_import(add_module):
    graph = ImportGraph()
    a, b = 'foo', 'bar'

    # Adding the module should make no difference to the result.
    if add_module:
        graph.add_module(a)

    graph.add_import(importer=a, imported=b)

    assert {a, b} == graph.modules
    assert {b} == graph.find_modules_directly_imported_by(a)
    assert set() == graph.find_modules_directly_imported_by(b)


def test_remove_import():
    graph = ImportGraph()
    a, b, c = 'foo', 'bar', 'baz'
    graph.add_import(importer=a, imported=b)
    graph.add_import(importer=a, imported=c)

    graph.remove_import(importer=a, imported=b)

    assert {a, b, c} == graph.modules
    assert {c} == graph.find_modules_directly_imported_by(a)


class TestGetImportDetails:
    def test_happy_path(self):
        graph = ImportGraph()

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
        graph = ImportGraph()

        assert [] == graph.get_import_details(importer='foo', imported='bar')

    def test_returns_only_relevant_imports(self):
        graph = ImportGraph()

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
