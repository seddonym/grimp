from typing import Optional


class GrimpException(Exception):
    """
    Base exception for all custom Grimp exceptions to inherit.
    """


class ModuleNotPresent(GrimpException):
    """
    Indicates that a module was not present in a graph.
    """


class NoSuchContainer(GrimpException):
    """
    Indicates that a passed container was not found as a module in the graph.
    """


class NamespacePackageEncountered(GrimpException):
    """
    Indicates that there was no __init__.py at the top level.

    This indicates a namespace package (see PEP 420), which is not currently supported. More
    typically this is just an oversight which can be fixed by adding the __init__.py file.
    """


class NotATopLevelModule(GrimpException):
    """
    Indicates when a child module was encountered unexpectedly.
    """


class SourceSyntaxError(GrimpException):
    """
    Indicates a syntax error in code that was being statically analysed.
    """

    def __init__(self, filename: str, lineno: Optional[int], text: Optional[str]) -> None:
        """
        Args:
            filename: The file which contained the error.
            lineno: The line number containing the error.
            text: The text containing the error.
        """
        self.filename = filename
        self.lineno = lineno
        self.text = text

    def __str__(self):
        lineno = self.lineno or "?"
        text = self.text or "<unavailable>"
        return f"Syntax error in {self.filename}, line {lineno}: {text}"

    def __eq__(self, other):
        return (self.filename, self.lineno, self.text) == (
            other.filename,
            other.lineno,
            other.text,
        )
