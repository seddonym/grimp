import sys
from pathlib import Path

ASSETS_PATH = Path(__file__).parent / "assets"

TEST_ASSETS = (
    ASSETS_PATH,
    ASSETS_PATH / "multipleroots",
)

# Add test assets to the Python path.
sys.path.extend([str((ASSETS_PATH / path).resolve()) for path in TEST_ASSETS])
