import time

from grimp.application.ports.timing import Timer


class SystemClockTimer(Timer):
    def get_current_time(self) -> float:
        return time.time()
