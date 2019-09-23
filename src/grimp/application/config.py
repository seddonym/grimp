from typing import Any


class Settings:
    def __init__(self):
        self._config = {}

    def configure(self, **config_dict: Any):
        self._config.update(config_dict)

    def __getattr__(self, name):
        if name[:2] != "__":
            return self._config[name]
        return super().__getattr__(name)

    def copy(self) -> "Settings":
        new_instance = self.__class__()
        new_instance.configure(**self._config)
        return new_instance


settings = Settings()
