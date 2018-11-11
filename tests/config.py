from grimp.application.config import settings


class override_settings:
    def __init__(self, **settings_to_override):
        self.settings_to_override = settings_to_override

    def __enter__(self):
        self.original_settings = settings.copy()
        settings.configure(**self.settings_to_override)

    def __exit__(self, exc_type, exc_val, exc_tb):
        for setting_name in self.settings_to_override:
            settings.configure(
                **{setting_name: getattr(self.original_settings, setting_name)}
            )
