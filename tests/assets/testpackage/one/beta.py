from testpackage.one import alpha
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from testpackage.two import alpha


def foo():
    return alpha.BAR
