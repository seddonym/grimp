class GrimpException(Exception):
    """
    Base exception for all custom Grimp exceptions to inherit.
    """


class ModuleNotPresent(GrimpException):
    """
    Indicates that a module was not present in a graph.
    """
    pass
