from . import one


def foo():
    from .two import alpha
    return 1
